use redis::{Client, Value};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use database::{establish_connection, schema, NewOrder, NewTrade, Order, Trade, Balance};
use uuid::Uuid;
use std::collections::HashMap;

// This enum MUST match exactly what the engine sends
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
enum DBUpdateEvent {
    OrderCreated(EngineOrder),
    OrderUpdated(EngineOrder),      // Engine sends full Order struct
    TradeExecuted(EngineTrade),     // Engine sends full Trade struct
    BalanceUpdated { 
        user_id: Uuid, 
        token_id: Uuid, 
        available: i64, 
        locked: i64 
    },
}

// These structs match exactly what the engine sends
#[derive(Debug, Clone, Deserialize, Serialize)]
struct EngineOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: EngineOrderType,
    pub order_kind: EngineOrderKind,
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: EngineOrderStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EngineTrade {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
}

// These enums match exactly what the engine sends
#[derive(Debug, Clone, Deserialize, Serialize)]
enum EngineOrderType { Buy, Sell }

#[derive(Debug, Clone, Deserialize, Serialize)]
enum EngineOrderKind { Market, Limit }

#[derive(Debug, Clone, Deserialize, Serialize)]
enum EngineOrderStatus { Pending, PartiallyFilled, Filled, Cancelled }

#[derive(Debug, Clone)]
struct PendingUpdate {
    stream_id: String,
    event: DBUpdateEvent,
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
    
    // Create consumer group (ignore error if already exists)
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
            .arg(20) // Process more messages at once for better batching
            .arg("BLOCK")
            .arg(1000)
            .arg("STREAMS")
            .arg("db_update_queue")
            .arg(">")
            .query_async(&mut conn)
            .await?;
        
        // Process updates with dependency ordering
        if let Some(updates) = parse_db_updates(results) {
            tracing::info!("📥 Received {} updates", updates.len());
            
            // Group and order updates
            let ordered_updates = order_updates_by_dependencies(updates);
            println!("ordered_updates: {:?}", ordered_updates);
            
            // Process in dependency order within a transaction
            db_conn.transaction::<_, Box<dyn std::error::Error>, _>(|tx_conn| {
                let mut processed_stream_ids = Vec::new();
                
                // Process orders first (they are dependencies for trades)
                for update in &ordered_updates.orders {
                    match process_db_update(update.event.clone(), tx_conn) {
                        Ok(_) => {
                            processed_stream_ids.push(update.stream_id.clone());
                            tracing::info!("✅ Processed order update: {}", update.stream_id);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to process order update {}: {}", update.stream_id, e);
                            return Err(e);
                        }
                    }
                }
                
                // Then process trades (they depend on orders)
                for update in &ordered_updates.trades {
                    match process_db_update(update.event.clone(), tx_conn) {
                        Ok(_) => {
                            processed_stream_ids.push(update.stream_id.clone());
                            tracing::info!("✅ Processed trade update: {}", update.stream_id);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to process trade update {}: {}", update.stream_id, e);
                            return Err(e);
                        }
                    }
                }
                
                // Finally process balances (they can be processed independently)
                for update in &ordered_updates.balances {
                    match process_db_update(update.event.clone(), tx_conn) {
                        Ok(_) => {
                            processed_stream_ids.push(update.stream_id.clone());
                            tracing::info!("✅ Processed balance update: {}", update.stream_id);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to process balance update {}: {}", update.stream_id, e);
                            return Err(e);
                        }
                    }
                }
                
                Ok(processed_stream_ids)
            })?;
            
            // Acknowledge all successfully processed messages
            for stream_id in &ordered_updates.all_stream_ids {
                let _: () = redis::cmd("XACK")
                    .arg("db_update_queue")
                    .arg(consumer_group)
                    .arg(stream_id)
                    .query_async(&mut conn)
                    .await?;
                
                tracing::debug!("✅ Acknowledged: {}", stream_id);
            }
            
            tracing::info!("✅ Batch processed {} updates successfully", ordered_updates.all_stream_ids.len());
        } else {
            tracing::debug!("No new messages");
        }
    }
}

#[derive(Debug)]
struct OrderedUpdates {
    orders: Vec<PendingUpdate>,
    trades: Vec<PendingUpdate>,
    balances: Vec<PendingUpdate>,
    all_stream_ids: Vec<String>,
}

/// Order updates by dependencies: orders first, then trades, then balances
fn order_updates_by_dependencies(updates: Vec<(String, DBUpdateEvent)>) -> OrderedUpdates {
    let mut orders = Vec::new();
    let mut trades = Vec::new();
    let mut balances = Vec::new();
    let mut all_stream_ids = Vec::new();
    
    for (stream_id, event) in updates {
        all_stream_ids.push(stream_id.clone());
        
        match &event {
            DBUpdateEvent::OrderCreated(_) | DBUpdateEvent::OrderUpdated(_) => {
                orders.push(PendingUpdate { stream_id, event });
            }
            DBUpdateEvent::TradeExecuted(_) => {
                trades.push(PendingUpdate { stream_id, event });
            }
            DBUpdateEvent::BalanceUpdated { .. } => {
                balances.push(PendingUpdate { stream_id, event });
            }
        }
    }
    
    tracing::info!("📋 Ordered updates: {} orders, {} trades, {} balances", 
        orders.len(), trades.len(), balances.len());
    
    OrderedUpdates {
        orders,
        trades,
        balances,
        all_stream_ids,
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
                                            } else {
                                                tracing::warn!("Failed to parse message fields for {}", stream_id);
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
        _ => {
            tracing::debug!("No stream data received");
            None
        }
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
    
    tracing::debug!("Parsing event type: {}", event_type);
    
    // Parse based on event type
    match event_type.as_str() {
        "order_created" => {
            match serde_json::from_str::<EngineOrder>(data_json) {
                Ok(order) => Some(DBUpdateEvent::OrderCreated(order)),
                Err(e) => {
                    tracing::error!("Failed to parse order_created: {}", e);
                    None
                }
            }
        }
        "order_updated" => {
            match serde_json::from_str::<EngineOrder>(data_json) {
                Ok(order) => Some(DBUpdateEvent::OrderUpdated(order)),
                Err(e) => {
                    tracing::error!("Failed to parse order_updated: {}", e);
                    None
                }
            }
        }
        "trade_executed" => {
            match serde_json::from_str::<EngineTrade>(data_json) {
                Ok(trade) => Some(DBUpdateEvent::TradeExecuted(trade)),
                Err(e) => {
                    tracing::error!("Failed to parse trade_executed: {}", e);
                    None
                }
            }
        }
        "balance_updated" => {
            #[derive(Deserialize)]
            struct BalanceUpdateData {
                user_id: Uuid,
                token_id: Uuid,
                available: i64,
                locked: i64,
            }
            
            match serde_json::from_str::<BalanceUpdateData>(data_json) {
                Ok(data) => Some(DBUpdateEvent::BalanceUpdated {
                    user_id: data.user_id,
                    token_id: data.token_id,
                    available: data.available,
                    locked: data.locked,
                }),
                Err(e) => {
                    tracing::error!("Failed to parse balance_updated: {}", e);
                    None
                }
            }
        }
        _ => {
            tracing::warn!("Unknown event type: {}", event_type);
            None
        }
    }
}

/// Process database update event
fn process_db_update(
    update: DBUpdateEvent, 
    db_conn: &mut PgConnection
) -> Result<(), Box<dyn std::error::Error>> {
    
    use schema::*;
    
    match update {
        DBUpdateEvent::OrderCreated(order_data) => {
            tracing::info!("💾 Creating order {} in database", order_data.id);
            
            let new_order = NewOrder {
                id: order_data.id,
                user_id: order_data.user_id,
                market_id: order_data.market_id,
                order_type: match order_data.order_type {
                    EngineOrderType::Buy => "BUY".to_string(),
                    EngineOrderType::Sell => "SELL".to_string(),
                },
                order_kind: match order_data.order_kind {
                    EngineOrderKind::Market => "MARKET".to_string(),
                    EngineOrderKind::Limit => "LIMIT".to_string(),
                },
                price: order_data.price,
                quantity: order_data.quantity,
                filled_quantity: Some(order_data.filled_quantity),
                status: Some(match order_data.status {
                    EngineOrderStatus::Pending => "PENDING".to_string(),
                    EngineOrderStatus::PartiallyFilled => "PARTIALLY_FILLED".to_string(),
                    EngineOrderStatus::Filled => "FILLED".to_string(),
                    EngineOrderStatus::Cancelled => "CANCELLED".to_string(),
                }),
            };
            
            // Use INSERT ON CONFLICT for idempotency
            diesel::insert_into(orders::table)
                .values(&new_order)
                .on_conflict(orders::id)
                .do_nothing()
                .execute(db_conn)?;
                
            tracing::debug!("✅ Order {} created successfully", order_data.id);
        }
        
        DBUpdateEvent::OrderUpdated(order_data) => {
            tracing::info!("💾 Updating order {} in database", order_data.id);
            
            let status_str = match order_data.status {
                EngineOrderStatus::Pending => "PENDING",
                EngineOrderStatus::PartiallyFilled => "PARTIALLY_FILLED",
                EngineOrderStatus::Filled => "FILLED",
                EngineOrderStatus::Cancelled => "CANCELLED",
            };
            
            // First try to update existing order
            let updated_rows = diesel::update(orders::table.find(order_data.id))
                .set((
                    orders::filled_quantity.eq(order_data.filled_quantity),
                    orders::status.eq(status_str),
                    orders::updated_at.eq(diesel::dsl::now),
                ))
                .execute(db_conn)?;
                
            // If no rows were updated, the order doesn't exist yet - create it
            if updated_rows == 0 {
                tracing::warn!("Order {} not found for update, creating it", order_data.id);
                
                let new_order = NewOrder {
                    id: order_data.id,
                    user_id: order_data.user_id,
                    market_id: order_data.market_id,
                    order_type: match order_data.order_type {
                        EngineOrderType::Buy => "BUY".to_string(),
                        EngineOrderType::Sell => "SELL".to_string(),
                    },
                    order_kind: match order_data.order_kind {
                        EngineOrderKind::Market => "MARKET".to_string(),
                        EngineOrderKind::Limit => "LIMIT".to_string(),
                    },
                    price: order_data.price,
                    quantity: order_data.quantity,
                    filled_quantity: Some(order_data.filled_quantity),
                    status: Some(status_str.to_string()),
                };
                
                diesel::insert_into(orders::table)
                    .values(&new_order)
                    .on_conflict(orders::id)
                    .do_nothing()
                    .execute(db_conn)?;
            }
                
            tracing::debug!("✅ Order {} updated successfully", order_data.id);
        }
        
        DBUpdateEvent::TradeExecuted(trade_data) => {
            tracing::info!("💾 Creating trade {} in database", trade_data.id);
            
            let new_trade = NewTrade {
                market_id: trade_data.market_id,
                buyer_order_id: trade_data.buyer_order_id,
                seller_order_id: trade_data.seller_order_id,
                buyer_user_id: trade_data.buyer_user_id,
                seller_user_id: trade_data.seller_user_id,
                price: trade_data.price,
                quantity: trade_data.quantity,
            };
            
            // Use INSERT ON CONFLICT for idempotency
            diesel::insert_into(trades::table)
                .values(&new_trade)
                .on_conflict(trades::id)
                .do_nothing()
                .execute(db_conn)?;
                
            tracing::debug!("✅ Trade {} created successfully", trade_data.id);
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
                    .on_conflict((balances::user_id, balances::token_id))
                    .do_update()
                    .set((
                        balances::amount.eq(available),
                        balances::locked_amount.eq(locked),
                        balances::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(db_conn)?;
                    
                tracing::debug!("✅ Balance upserted for user {} token {}", user_id, token_id);
            } else {
                tracing::debug!("✅ Balance updated for user {} token {}", user_id, token_id);
            }
        }
    }
    
    Ok(())
}