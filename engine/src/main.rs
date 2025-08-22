mod redis_manager;
mod trading_engine;
mod decimal_utils;

use redis_manager::{EngineRedisManager, EngineMessage, EngineResponse};
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

      // Load markets from database
    if let Err(e) = trading_engine.load_markets().await {
        error!("Failed to load markets: {}", e);
        return Err(e);
    } else {
        info!("✅ Markets loaded successfully");
    }

    if let Err(e) = trading_engine.load_balance_snapshots().await {
        error!("Failed to load balance snapshots: {}", e);
    } else {
        info!("✅ Balance snapshots loaded successfully");
    }

    let consumer_group = "matching_engine_group";
    let consumer_name = "engine_1";
    
    info!("🔄 Starting order processing loop...");
    
    loop {
        match redis_manager.consume_messages(consumer_group, consumer_name, 10).await {
            Ok(messages) => {
                println!("🔍 Received messages: {:?}", messages);
                if !messages.is_empty() {
                    info!("📥 Received {} messages to process", messages.len());
                    
                    // Process each message
                    for (stream_id, message) in messages {
                        let response = match message {
                            EngineMessage::Order(order_request) => {
                                info!("🔄 Processing order: {}", order_request.request_id);
                                let order_response = trading_engine.process_order(order_request).await;
                                EngineResponse::Order(order_response)
                            }
                            EngineMessage::Balance(balance_request) => {
                                info!("💰 Processing balance: {}", balance_request.request_id);
                                let balance_response = trading_engine.process_balance_request(balance_request).await;
                                EngineResponse::Balance(balance_response)
                            }
                            EngineMessage::CancelOrder(cancel_order_request) => {
                                info!("🔄 Processing cancel order: {}", cancel_order_request.request_id);
                                let cancel_order_response = trading_engine.process_cancel_order(cancel_order_request).await;
                                EngineResponse::Order(cancel_order_response)
                            }
                        };

                        // Send unified response
                        if let Err(e) = redis_manager.send_unified_response(response).await {
                            error!("Failed to send response: {}", e);
                        }

                        // Acknowledge message
                        if let Err(e) = redis_manager.ack_message(consumer_group, &stream_id).await {
                            error!("Failed to acknowledge message: {}", e);
                        }
                    }
                } else {
                    // No messages, sleep briefly
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                error!("Error consuming messages: {}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}