use redis::{Client, aio::ConnectionManager, Commands};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::time::{timeout, Duration};
use chrono::Utc;
use futures_util::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub request_id: String,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String, // BUY/SELL
    pub order_kind: String, // MARKET/LIMIT
    pub price: Option<i64>,
    pub quantity: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub request_id: String,
    pub success: bool,
    pub status: String, // "FILLED", "PARTIALLY_FILLED", "PENDING", "REJECTED"
    pub order_id: Option<Uuid>,
    pub message: String,
    pub filled_quantity: Option<i64>,
    pub remaining_quantity: Option<i64>,
    pub average_price: Option<i64>,
    pub trades: Option<Vec<TradeInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeInfo {
    pub trade_id: Uuid,
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderProcessingResult {
    Success(OrderResponse),
    Timeout,
    Error(String),
}

pub struct RedisManager {
    connection_manager: ConnectionManager,
    client: Client,
}

impl RedisManager {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        let connection_manager = ConnectionManager::new(client.clone()).await?;
        
        Ok(RedisManager { 
            connection_manager, 
            client 
        })
    }   

    /// Main function: Send order to queue and wait for response
    /// This is what the API will use for all order operations
    pub async fn send_and_wait(
        &self,
        order_request: OrderRequest,
        timeout_secs: u64,
    ) -> OrderProcessingResult {
        let request_id = order_request.request_id.clone();
        
        // Step 1: Subscribe to response channel BEFORE queuing
        // This prevents race conditions
        let response_channel = format!("order_response:{}", request_id);
        
        let subscribe_result = async {
            let conn = self.client.get_async_connection().await?;
            let mut pubsub = conn.into_pubsub();
            pubsub.subscribe(&response_channel).await?;
            Ok::<_, redis::RedisError>(pubsub)
        }.await;
        
        let mut pubsub = match subscribe_result {
            Ok(ps) => ps,
            Err(e) => return OrderProcessingResult::Error(format!("Failed to subscribe: {}", e)),
        };
        
        // Step 2: Queue the order
        if let Err(e) = self.queue_order_internal(order_request).await {
            return OrderProcessingResult::Error(format!("Failed to queue order: {}", e));
        }
        
        // Step 3: Wait for response with timeout
        let wait_result = timeout(Duration::from_secs(timeout_secs), async {
            loop {
                match pubsub.on_message().next().await {
                    Some(msg) => {
                        let payload: Result<String, _> = msg.get_payload();
                        match payload {
                            Ok(response_str) => {
                                match serde_json::from_str::<OrderResponse>(&response_str) {
                                    Ok(response) => return Ok(response),
                                    Err(e) => {
                                        println!("Failed to parse response: {}", e);
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to get message payload: {}", e);
                                continue;
                            }
                        }
                    }
                    None => {
                        // Connection closed or error
                        return Err("PubSub connection closed".to_string());
                    }
                }
            }
        }).await;
        
        match wait_result {
            Ok(Ok(response)) => OrderProcessingResult::Success(response),
            Ok(Err(e)) => OrderProcessingResult::Error(e),
            Err(_) => OrderProcessingResult::Timeout,
        }
    }

    /// Internal function to queue order to Redis Stream
    async fn queue_order_internal(&self, order_request: OrderRequest) -> Result<String, redis::RedisError> {
        let mut conn = self.connection_manager.clone();
        let order_json = serde_json::to_string(&order_request).unwrap();
        
        // Add to Redis Stream - this is what the engine will consume
        let stream_id: String = redis::cmd("XADD")
            .arg("order_processing_queue") // Queue name
            .arg("*") // Auto-generate stream ID
            .arg("request_id")
            .arg(&order_request.request_id)
            .arg("data")
            .arg(&order_json) 
            .arg("timestamp")
            .arg(order_request.timestamp)
            .query_async(&mut conn)
            .await?;
            
        println!("✅ Queued order {} to stream {}", order_request.request_id, stream_id);
        Ok(stream_id)
    }
}

// Singleton pattern
use tokio::sync::OnceCell;
static REDIS_MANAGER: OnceCell<RedisManager> = OnceCell::const_new();

pub async fn get_redis_manager() -> &'static RedisManager {
    REDIS_MANAGER
        .get_or_init(|| async {
            RedisManager::new("redis://127.0.0.1:6379/")
                .await
                .expect("Failed to create Redis manager")
        })
        .await
}