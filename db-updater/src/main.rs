use redis::{Client, Value};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use database::{establish_connection, schema, NewOrder, NewTrade, Order, Trade, Balance};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
enum DBUpdateEvent {
    OrderCreated(OrderData),
    OrderUpdated(OrderUpdateData), 
    TradeExecuted(TradeData),
    BalanceUpdated { 
        user_id: Uuid, 
        token_id: Uuid, 
        available: i64, 
        locked: i64 
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OrderData {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String,
    pub order_kind: String,
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OrderUpdateData {
    pub id: Uuid,
    pub filled_quantity: i64,
    pub status: String,
}

// Add these structs to match engine's Order and Trade
#[derive(Debug, Clone, Deserialize, Serialize)]
struct EngineOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String, // Will be "Buy" or "Sell"
    pub order_kind: String, // Will be "Market" or "Limit"  
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: String, // Will be "Pending", "PartiallyFilled", etc.
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EngineTradeData {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,    // Add this field
    pub seller_user_id: Uuid,   // Add this field
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TradeData {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: i64,
    pub quantity: i64,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("🗃️ Starting DB-Updater Service...");
    
    let redis_client = Client::open("redis://127.0.0.1:6379/")?;
    let mut conn = redis_client.get_async_connection().await?;
    let mut db_conn = establish_connection();
    
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
        // Read from queue with consumer group
        let results: Value = redis::cmd("XREADGROUP")
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
                match process_db_update(update, &mut db_conn).await {
                    Ok(_) => {
                        // Acknowledge successful processing
                        let _: () = redis::cmd("XACK")
                            .arg("db_update_queue")
                            .arg(consumer_group)
                            .arg(&stream_id)
                            .query_async(&mut conn)
                            .await?;
                        
                        tracing::info!("✅ Processed and acknowledged: {}", stream_id);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to process DB update {}: {}", stream_id, e);
                        // Message will remain in pending list and can be retried
                    }
                }
            }
        }
    }
}

/// Parse Redis stream response into database update events
fn parse_db_updates(results: Value) -> Option<Vec<(String, DBUpdateEvent)>> {
    match results {
        Value::Bulk(streams) => {
            let mut updates = Vec::new();
            
            for stream in streams {
                if let Value::Bulk(stream_data) = stream {
                    if stream_data.len() >= 2 {
                        if let Value::Bulk(messages) = &stream_data[1] {
                            for message in messages {
                                if let Value::Bulk(msg_data) = message {
                                    if msg_data.len() >= 2 {
                                        // Extract stream ID
                                        let stream_id = match &msg_data[0] {
                                            Value::Data(bytes) => String::from_utf8_lossy(bytes).to_string(),
                                            _ => continue,
                                        };
                                        
                                        // Extract fields
                                        if let Value::Bulk(fields) = &msg_data[1] {
                                            if let Some(update) = parse_message_fields(fields) {
                                                updates.push((stream_id, update));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if updates.is_empty() { None } else { Some(updates) }
        }
        _ => None,
    }
}

/// Parse message fields into database update event
fn parse_message_fields(fields: &[Value]) -> Option<DBUpdateEvent> {
    let mut field_map = std::collections::HashMap::new();
    
    // Parse key-value pairs
    for chunk in fields.chunks(2) {
        if chunk.len() == 2 {
            let key = match &chunk[0] {
                Value::Data(bytes) => String::from_utf8_lossy(bytes).to_string(),
                _ => continue,
            };
            let value = match &chunk[1] {
                Value::Data(bytes) => String::from_utf8_lossy(bytes).to_string(),
                _ => continue,
            };
            field_map.insert(key, value);
        }
    }
    
    // Get event type and data
    let event_type = field_map.get("type")?;
    let data_json = field_map.get("data")?;
    
    // Parse based on event type
    match event_type.as_str() {
        "order_created" => {
            serde_json::from_str::<OrderData>(data_json)
                .ok()
                .map(DBUpdateEvent::OrderCreated)
        }
        "order_updated" => {
            serde_json::from_str::<OrderUpdateData>(data_json)
                .ok()
                .map(DBUpdateEvent::OrderUpdated)
        }
        "trade_executed" => {
            serde_json::from_str::<TradeData>(data_json)
                .ok()
                .map(DBUpdateEvent::TradeExecuted)
        }
        "balance_updated" => {
            #[derive(Deserialize)]
            struct BalanceUpdateData {
                user_id: Uuid,
                token_id: Uuid,
                available: i64,
                locked: i64,
            }
            
            serde_json::from_str::<BalanceUpdateData>(data_json)
                .ok()
                .map(|data| DBUpdateEvent::BalanceUpdated {
                    user_id: data.user_id,
                    token_id: data.token_id,
                    available: data.available,
                    locked: data.locked,
                })
        }
        _ => {
            tracing::warn!("Unknown event type: {}", event_type);
            None
        }
    }
}

/// Process database update event
async fn process_db_update(
    update: DBUpdateEvent, 
    db_conn: &mut PgConnection
) -> Result<(), Box<dyn std::error::Error>> {
    
    use schema::*;
    
    match update {
        DBUpdateEvent::OrderCreated(order_data) => {
            tracing::info!("💾 Creating order {} in database", order_data.id);
            
            let new_order = NewOrder {
                user_id: order_data.user_id,
                market_id: order_data.market_id,
                order_type: order_data.order_type,
                order_kind: order_data.order_kind,
                price: order_data.price,
                quantity: order_data.quantity,
                filled_quantity: Some(order_data.filled_quantity),
                status: Some(order_data.status),
            };
            
            diesel::insert_into(orders::table)
                .values(&new_order)
                .execute(db_conn)?;
                
            tracing::info!("✅ Order {} created successfully", order_data.id);
        }
        
        DBUpdateEvent::OrderUpdated(order_update) => {
            tracing::info!("💾 Updating order {} in database", order_update.id);
            
            diesel::update(orders::table.find(order_update.id))
                .set((
                    orders::filled_quantity.eq(order_update.filled_quantity),
                    orders::status.eq(order_update.status),
                    orders::updated_at.eq(diesel::dsl::now),
                ))
                .execute(db_conn)?;
                
            tracing::info!("✅ Order {} updated successfully", order_update.id);
        }
        
        DBUpdateEvent::TradeExecuted(trade_data) => {
            tracing::info!("💾 Creating trade {} in database", trade_data.id);
            
            let new_trade = NewTrade {
                market_id: trade_data.market_id,
                buyer_order_id: trade_data.buyer_order_id,
                seller_order_id: trade_data.seller_order_id,
                price: trade_data.price,
                quantity: trade_data.quantity,
            };
            
            diesel::insert_into(trades::table)
                .values(&new_trade)
                .execute(db_conn)?;
                
            tracing::info!("✅ Trade {} created successfully", trade_data.id);
        }
        
        DBUpdateEvent::BalanceUpdated { user_id, token_id, available, locked } => {
            tracing::info!("💾 Updating balance for user {} token {}", user_id, token_id);
            
            // Try to update existing balance
            let updated_rows = diesel::update(
                balances::table
                    .filter(balances::user_id.eq(user_id))
                    .filter(balances::token_id.eq(token_id))
            )
            .set((
                balances::amount.eq(available),
                balances::locked_amount.eq(locked),
                balances::updated_at.eq(diesel::dsl::now),
            ))
            .execute(db_conn)?;
            
            // If no rows updated, create new balance record
            if updated_rows == 0 {
                use database::NewBalance;
                
                let new_balance = NewBalance {
                    user_id,
                    token_id,
                    amount: Some(available),
                    locked_amount: Some(locked),
                };
                
                diesel::insert_into(balances::table)
                    .values(&new_balance)
                    .execute(db_conn)?;
                    
                tracing::info!("✅ New balance created for user {} token {}", user_id, token_id);
            } else {
                tracing::info!("✅ Balance updated for user {} token {}", user_id, token_id);
            }
        }
    }
    
    Ok(())
}