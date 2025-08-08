use redis::{Client, Commands, aio::ConnectionManager, AsyncCommands};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// Unified message types (same as API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EngineMessage {
    Order(OrderRequest),
    Balance(BalanceRequest),
    CancelOrder(CancelOrderRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]  
pub enum EngineResponse {
    Order(OrderResponse),
    Balance(BalanceResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    pub request_id: String,
    pub user_id: Uuid,
    pub order_id: Uuid,
    pub market_id: Uuid,
    pub timestamp: i64,
}

// Re-use the same structs as API for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub request_id: String,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String, // Buy/Sell
    pub order_kind: String, // Market/Limit
    pub price: Option<i64>,
    pub quantity: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub request_id: String,
    pub success: bool,
    pub status: String, // "Filled", "PartiallyFilled", "Pending", "Rejected"
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

// Balance types (add these)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceRequest {
    pub request_id: String,
    pub user_id: Uuid,
    pub token_id: Uuid,
    pub operation: BalanceOperation,
    pub amount: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BalanceOperation {
    Deposit,
    Withdraw,
    GetBalances,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub request_id: String,
    pub success: bool,
    pub message: String,
    pub new_balance: i64,
    pub balances: Option<Vec<UserTokenBalance>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokenBalance {
    pub token_id: Uuid,
    pub available: i64,
    pub locked: i64,
}

// Redis manager
pub struct EngineRedisManager {
    connection_manager: ConnectionManager,
}

impl EngineRedisManager {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        let connection_manager = ConnectionManager::new(client).await?;
        
        Ok(EngineRedisManager {
            connection_manager,
        })
    }

     /// 🚀 UNIFIED: Consume all message types from single queue
     pub async fn consume_messages(
        &self,
        consumer_group: &str,
        consumer_name: &str,
        batch_size: usize,
    ) -> Result<Vec<(String, EngineMessage)>, redis::RedisError> {
        let mut conn = self.connection_manager.clone();
        
        // Create consumer group if it doesn't exist
        let _: Result<String, _> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg("engine_processing_queue") // Unified queue name
            .arg(consumer_group)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;
        
        // Read from stream with consumer group
        let results: redis::Value = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(consumer_group)
            .arg(consumer_name)
            .arg("COUNT")
            .arg(batch_size)
            .arg("BLOCK")
            .arg(1000) // 1 second timeout
            .arg("STREAMS")
            .arg("engine_processing_queue")
            .arg(">") // Only new messages
            .query_async(&mut conn)
            .await?;
        
        let mut messages = Vec::new();
        
        // Parse Redis stream response
        if let redis::Value::Bulk(streams) = results {
            if let Some(redis::Value::Bulk(stream_data)) = streams.get(0) {
                if let Some(redis::Value::Bulk(stream_messages)) = stream_data.get(1) {
                    for message in stream_messages {
                        if let redis::Value::Bulk(msg_parts) = message {
                            if let (Some(redis::Value::Data(stream_id)), Some(redis::Value::Bulk(fields))) = 
                                (msg_parts.get(0), msg_parts.get(1)) {
                                
                                let stream_id_str = String::from_utf8_lossy(stream_id);
                                let mut data = None;
                                let mut message_type = None;
                                
                                // Parse field-value pairs
                                for chunk in fields.chunks(2) {
                                    if let (Some(redis::Value::Data(key)), Some(redis::Value::Data(value))) = 
                                        (chunk.get(0), chunk.get(1)) {
                                        let key_str = String::from_utf8_lossy(key);
                                        let value_str = String::from_utf8_lossy(value);
                                        
                                        match key_str.as_ref() {
                                            "data" => data = Some(value_str.to_string()),
                                            "message_type" => message_type = Some(value_str.to_string()),
                                            _ => {}
                                        }
                                    }
                                }
                                
                                if let Some(data_str) = data {
                                    if let Ok(engine_message) = serde_json::from_str::<EngineMessage>(&data_str) {
                                        messages.push((stream_id_str.to_string(), engine_message));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(messages)
    }

    /// Consume orders from the processing queue
    // pub async fn consume_orders(
    //     &self,
    //     consumer_group: &str,
    //     consumer_name: &str,
    //     batch_size: usize,
    // ) -> Result<Vec<(String, OrderRequest)>, redis::RedisError> {
    //     let mut conn = self.connection_manager.clone();
        
    //     // Create consumer group if it doesn't exist
    //     let _: Result<String, _> = redis::cmd("XGROUP")
    //         .arg("CREATE")
    //         .arg("order_processing_queue")
    //         .arg(consumer_group)
    //         .arg("0")
    //         .arg("MKSTREAM")
    //         .query_async(&mut conn)
    //         .await;
        
    //     // Read from stream with consumer group
    //     let results: redis::Value = redis::cmd("XREADGROUP")
    //         .arg("GROUP")
    //         .arg(consumer_group)
    //         .arg(consumer_name)
    //         .arg("COUNT")
    //         .arg(batch_size)
    //         .arg("BLOCK")
    //         .arg(1000) // 1 second timeout
    //         .arg("STREAMS")
    //         .arg("order_processing_queue")
    //         .arg(">") // Only new messages
    //         .query_async(&mut conn)
    //         .await?;
        
    //     let mut orders = Vec::new();
        
    //     // Parse Redis stream response
    //     if let redis::Value::Bulk(streams) = results {
    //         if let Some(redis::Value::Bulk(stream_data)) = streams.get(0) {
    //             if let Some(redis::Value::Bulk(messages)) = stream_data.get(1) {
    //                 for message in messages {
    //                     if let redis::Value::Bulk(msg_parts) = message {
    //                         if let (Some(redis::Value::Data(stream_id)), Some(redis::Value::Bulk(fields))) = 
    //                             (msg_parts.get(0), msg_parts.get(1)) {
                                
    //                             let stream_id_str = String::from_utf8_lossy(stream_id);
    //                             let mut data = None;
                                
    //                             // Parse field-value pairs
    //                             for chunk in fields.chunks(2) {
    //                                 if let (Some(redis::Value::Data(key)), Some(redis::Value::Data(value))) = 
    //                                     (chunk.get(0), chunk.get(1)) {
    //                                     let key_str = String::from_utf8_lossy(key);
    //                                     if key_str == "data" {
    //                                         let value_str = String::from_utf8_lossy(value);
    //                                         data = Some(value_str.to_string());
    //                                         break;
    //                                     }
    //                                 }
    //                             }
                                
    //                             if let Some(data_str) = data {
    //                                 if let Ok(order) = serde_json::from_str::<OrderRequest>(&data_str) {
    //                                     orders.push((stream_id_str.to_string(), order));
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
        
    //     Ok(orders)
    // }

    /// 🚀 UNIFIED: Send any response type
    pub async fn send_unified_response(
        &self,
        response: EngineResponse,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.connection_manager.clone();
        
        let request_id = match &response {
            EngineResponse::Order(resp) => resp.request_id.clone(),
            EngineResponse::Balance(resp) => resp.request_id.clone(),
        };
        
        let response_channel = format!("engine_response:{}", request_id);
        let response_json = serde_json::to_string(&response).unwrap();
        
        let _: () = conn.publish(&response_channel, response_json).await?;
        tracing::info!("📤 Sent unified response for request {}", request_id);
        Ok(())
    }

    /// Acknowledge message processing (unified)
    pub async fn ack_message(
        &self,
        consumer_group: &str,
        stream_id: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.connection_manager.clone();
        
        let _: () = redis::cmd("XACK")
            .arg("engine_processing_queue") // Unified queue
            .arg(consumer_group)
            .arg(stream_id)
            .query_async(&mut conn)
            .await?;
        
        Ok(())
    }

    // /// Send response back to API
    // pub async fn send_response(
    //     &self,
    //     response: OrderResponse,
    // ) -> Result<(), redis::RedisError> {
    //     let mut conn = self.connection_manager.clone();
    //     let response_channel = format!("order_response:{}", response.request_id);
    //     let response_json = serde_json::to_string(&response).unwrap();
        
    //     let _: () = conn.publish(&response_channel, response_json).await?;
    //     tracing::info!("📤 Sent response for request {}", response.request_id);
    //     Ok(())
    // }

    // /// Acknowledge message processing (mark as processed in consumer group)
    // pub async fn ack_message(
    //     &self,
    //     consumer_group: &str,
    //     stream_id: &str,
    // ) -> Result<(), redis::RedisError> {
    //     let mut conn = self.connection_manager.clone();
        
    //     let _: () = redis::cmd("XACK")
    //         .arg("order_processing_queue")
    //         .arg(consumer_group)
    //         .arg(stream_id)
    //         .query_async(&mut conn)
    //         .await?;
        
    //     Ok(())
    // }
}