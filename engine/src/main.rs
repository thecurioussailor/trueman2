mod redis_manager;
mod trading_engine;

use redis_manager::EngineRedisManager;
use trading_engine::TradingEngine;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Matching Engine...");
    
    // Connect to Redis
    let redis_manager = EngineRedisManager::new("redis://127.0.0.1:6379/").await?;
    info!("✅ Connected to Redis");

    // Create TradingEngine instance
    let mut trading_engine = TradingEngine::new("redis://127.0.0.1:6379/").await?;
    info!("✅ TradingEngine initialized");
    
    let consumer_group = "matching_engine_group";
    let consumer_name = "engine_1";
    
    info!("🔄 Starting order processing loop...");
    
    loop {
        match redis_manager.consume_orders(consumer_group, consumer_name, 10).await {
            Ok(orders) => {
                if !orders.is_empty() {
                    info!("📥 Received {} orders to process", orders.len());
                    
                    // Process each order
                    for (stream_id, order_request) in orders {
                        info!("Processing order: {}", order_request.request_id);
                        
                        // Process the order
                        let response = trading_engine.process_order(order_request).await;
                        
                        // Send response back to API
                        if let Err(e) = redis_manager.send_response(response).await {
                            error!("Failed to send response: {}", e);
                        }
                        
                        // Acknowledge the message
                        if let Err(e) = redis_manager.ack_message(consumer_group, &stream_id).await {
                            error!("Failed to acknowledge message: {}", e);
                        }
                    }
                } else {
                    // No orders, sleep briefly
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                error!("Error consuming orders: {}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}