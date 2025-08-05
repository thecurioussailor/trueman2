use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use database::establish_connection;

#[derive(Debug, Clone, Deserialize)]
enum DBUpdateEvent {
    OrderCreated(Order),
    OrderUpdated(Order), 
    TradeExecuted(Trade),
    BalanceUpdated { user_id: uuid::Uuid, token_id: uuid::Uuid, available: i64, locked: i64 },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    tracing::info!("🗃️ Starting DB-Updater Service...");
    
    let redis_client = Client::open("redis://127.0.0.1:6379/")?;
    let mut conn = redis_client.get_async_connection().await?;
    let db_conn = establish_connection();
    
    let consumer_group = "db_updater_group";
    let consumer_name = "db_updater_1";
    
    // Create consumer group
    let _: Result<String, _> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg("db_update_queue")
        .arg(consumer_group)
        .arg("0")
        .arg("MKSTREAM")
        .query_async(&mut conn)
        .await;
    
    tracing::info!("📥 Listening for database updates...");
    
    loop {
        // Read from queue
        let results: redis::Value = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(consumer_group)
            .arg(consumer_name)
            .arg("COUNT")
            .arg(10)
            .arg("BLOCK")
            .arg(1000)
            .arg("STREAMS")
            .arg("db_update_queue")
            .arg(">")
            .query_async(&mut conn)
            .await?;
        
        // Process updates
        if let Some(updates) = parse_db_updates(results) {
            for (stream_id, update) in updates {
                match process_db_update(update, &db_conn).await {
                    Ok(_) => {
                        // Acknowledge message
                        let _: () = redis::cmd("XACK")
                            .arg("db_update_queue")
                            .arg(consumer_group)
                            .arg(&stream_id)
                            .query_async(&mut conn)
                            .await?;
                    }
                    Err(e) => {
                        tracing::error!("Failed to process DB update: {}", e);
                    }
                }
            }
        }
    }
}

async fn process_db_update(update: DBUpdateEvent, db_conn: &PgConnection) -> Result<(), Box<dyn std::error::Error>> {
    match update {
        DBUpdateEvent::OrderUpdated(order) => {
            // Save order to database
            tracing::info!("💾 Saving order {} to database", order.id);
            // Implementation depends on your database schema
        }
        DBUpdateEvent::TradeExecuted(trade) => {
            // Save trade to database
            tracing::info!("💾 Saving trade {} to database", trade.id);
            // Implementation depends on your database schema
        }
        DBUpdateEvent::BalanceUpdated { user_id, token_id, available, locked } => {
            // Update balance in database
            tracing::info!("💾 Updating balance for user {} token {}", user_id, token_id);
            // Implementation depends on your database schema
        }
    }
    Ok(())
}