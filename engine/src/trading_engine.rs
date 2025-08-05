use std::collections::{HashMap, BTreeMap, VecDeque};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tokio::time::{interval, Duration};
use redis::{aio::ConnectionManager, AsyncCommands, Commands}; 
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: OrderType, // Buy/Sell
    pub order_kind: OrderKind, // Market/Limit
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: OrderStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType { Buy, Sell }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderKind { Market, Limit }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus { Pending, PartiallyFilled, Filled, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: i64,
    pub quantity: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: Uuid,
    pub bids: BTreeMap<i64, VecDeque<Order>>, // Price -> Orders (descending)
    pub asks: BTreeMap<i64, VecDeque<Order>>, // Price -> Orders (ascending)
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTicker {
    pub market_id: Uuid,
    pub last_price: i64,
    pub volume_24h: i64,
    pub high_24h: i64,
    pub low_24h: i64,
    pub change_24h: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBalance {
    pub user_id: Uuid,
    pub token_balances: HashMap<Uuid, TokenBalance>, // token_id -> balance
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub available: i64,
    pub locked: i64,
}

// Events for DB-Updater queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DBUpdateEvent {
    OrderCreated(Order),
    OrderUpdated(Order),
    TradeExecuted(Trade),
    BalanceUpdated { user_id: Uuid, token_id: Uuid, available: i64, locked: i64 },
}

// Events for WebSocket PubSub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    OrderBookUpdate { market_id: Uuid, orderbook: OrderBookSnapshot },
    TickerUpdate(MarketTicker),
    TradeUpdate(Trade),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub bids: Vec<(i64, i64)>, // (price, total_quantity)
    pub asks: Vec<(i64, i64)>,
    pub timestamp: i64,
}

pub struct TradingEngine {
    // IN-MEMORY: Core trading data
    orderbooks: HashMap<Uuid, OrderBook>,
    balances: HashMap<Uuid, UserBalance>,
    tickers: HashMap<Uuid, MarketTicker>,
    
    // REDIS: Communication layer
    redis_manager: ConnectionManager,
    
    // COUNTERS: For snapshot triggers
    operations_since_snapshot: u64,
    snapshot_interval: u64, // Snapshot every N operations
}

impl TradingEngine {
    pub async fn new(redis_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = redis::Client::open(redis_url)?;
        let redis_manager = ConnectionManager::new(client).await?;
        
        Ok(TradingEngine {
            orderbooks: HashMap::new(),
            balances: HashMap::new(),
            tickers: HashMap::new(),
            redis_manager,
            operations_since_snapshot: 0,
            snapshot_interval: 100, // Snapshot every 100 operations
        })
    }
    
    /// Main order processing function
    pub async fn process_order(&mut self, order_request: crate::redis_manager::OrderRequest) -> crate::redis_manager::OrderResponse {
        // 1. Convert request to internal order
        let order = self.create_order_from_request(order_request.clone());
        
        // 2. Validate balances
        if !self.validate_order_balance(&order) {
            return crate::redis_manager::OrderResponse {
                request_id: order_request.request_id,
                success: false,
                status: "REJECTED".to_string(),
                order_id: None,
                message: "Insufficient balance".to_string(),
                filled_quantity: None,
                remaining_quantity: None,
                average_price: None,
                trades: None,
            };
        }
        
        // 3. Execute matching in memory
        let (updated_order, trades) = self.match_order(order).await;
        
        // 4. Update in-memory state
        self.update_balances_from_trades(&trades).await;
        self.update_ticker_from_trades(&trades).await;
        
        // 5. Queue database updates (async, non-blocking)
        self.queue_db_updates(&updated_order, &trades).await;
        
        // 6. Publish real-time updates (async, non-blocking)
        self.publish_market_events(&updated_order, &trades).await;
        
        // 7. Check if snapshot needed
        self.operations_since_snapshot += 1;
        if self.operations_since_snapshot >= self.snapshot_interval {
            self.take_snapshots().await;
            self.operations_since_snapshot = 0;
        }
        
        // 8. Return response
        crate::redis_manager::OrderResponse {
            request_id: order_request.request_id,
            success: true,
            status: if updated_order.filled_quantity == updated_order.quantity { "FILLED" } else { "PARTIALLY_FILLED" }.to_string(),
            order_id: Some(updated_order.id),
            message: "Order processed successfully".to_string(),
            filled_quantity: Some(updated_order.filled_quantity),
            remaining_quantity: Some(updated_order.quantity - updated_order.filled_quantity),
            average_price: self.calculate_average_price(&trades),
            trades: Some(trades.into_iter().map(|t| crate::redis_manager::TradeInfo {
                trade_id: t.id,
                price: t.price,
                quantity: t.quantity,
                timestamp: t.timestamp,
            }).collect()),
        }
    }
    
    /// Execute order matching against orderbook
    async fn match_order(&mut self, mut order: Order) -> (Order, Vec<Trade>) {
        let mut trades = Vec::new();
        let orderbook = self.orderbooks.entry(order.market_id).or_insert_with(|| OrderBook {
            market_id: order.market_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_updated: Utc::now().timestamp_millis(),
        });
        
        // Simplified matching logic (you can make this more sophisticated)
        match order.order_kind {
            OrderKind::Market => {
                // Market orders: match immediately at best available prices
                let (opposite_side, same_side) = match order.order_type {
                    OrderType::Buy => (&mut orderbook.asks, &mut orderbook.bids),
                    OrderType::Sell => (&mut orderbook.bids, &mut orderbook.asks),
                };
                
                // Match against opposite side
                let mut remaining_quantity = order.quantity;
                let mut prices_to_remove = Vec::new();
                
                for (&price, order_queue) in opposite_side.iter_mut() {
                    if remaining_quantity == 0 { break; }
                    
                    while let Some(mut matching_order) = order_queue.pop_front() {
                        if remaining_quantity == 0 { break; }
                        
                        let trade_quantity = remaining_quantity.min(matching_order.quantity - matching_order.filled_quantity);
                        
                        // Create trade
                        let trade = Trade {
                            id: Uuid::new_v4(),
                            market_id: order.market_id,
                            buyer_order_id: if matches!(order.order_type, OrderType::Buy) { order.id } else { matching_order.id },
                            seller_order_id: if matches!(order.order_type, OrderType::Sell) { order.id } else { matching_order.id },
                            price,
                            quantity: trade_quantity,
                            timestamp: Utc::now().timestamp_millis(),
                        };
                        
                        trades.push(trade);
                        
                        // Update orders
                        order.filled_quantity += trade_quantity;
                        matching_order.filled_quantity += trade_quantity;
                        remaining_quantity -= trade_quantity;
                        
                        // Handle filled/partial fills
                        if matching_order.filled_quantity < matching_order.quantity {
                            order_queue.push_front(matching_order); // Put back if not fully filled
                            break;
                        }
                    }
                    
                    if order_queue.is_empty() {
                        prices_to_remove.push(price);
                    }
                }
                
                // Clean up empty price levels
                for price in prices_to_remove {
                    opposite_side.remove(&price);
                }
                
                // Update order status
                order.status = if order.filled_quantity == order.quantity {
                    OrderStatus::Filled
                } else if order.filled_quantity > 0 {
                    OrderStatus::PartiallyFilled
                } else {
                    OrderStatus::Pending
                };
            }
            OrderKind::Limit => {
                // Limit orders: try to match, then add to book if not fully filled
                // Similar logic but add remaining quantity to orderbook
                // Implementation similar to market order matching...
                
                // For now, simplified: add to orderbook
                let price = order.price.unwrap();
                let order_queue = match order.order_type {
                    OrderType::Buy => orderbook.bids.entry(price).or_insert_with(VecDeque::new),
                    OrderType::Sell => orderbook.asks.entry(price).or_insert_with(VecDeque::new),
                };
                order_queue.push_back(order.clone());
                order.status = OrderStatus::Pending;
            }
        }
        
        orderbook.last_updated = Utc::now().timestamp_millis();
        (order, trades)
    }
    
    /// Queue updates for db-updater service
    async fn queue_db_updates(&mut self, order: &Order, trades: &[Trade]) {
        let mut conn = self.redis_manager.clone();
        
        // Queue order update
        let order_event = DBUpdateEvent::OrderUpdated(order.clone());
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("order_updated")
            .arg("data")
            .arg(serde_json::to_string(&order_event).unwrap())
            .query_async(&mut conn)
            .await;
        
        // Queue trade events
        for trade in trades {
            let trade_event = DBUpdateEvent::TradeExecuted(trade.clone());
            let _: Result<String, _> = redis::cmd("XADD")
                .arg("db_update_queue")
                .arg("*")
                .arg("type")
                .arg("trade_executed")
                .arg("data")
                .arg(serde_json::to_string(&trade_event).unwrap())
                .query_async(&mut conn)
                .await;
        }
        
        tracing::info!("📤 Queued {} DB updates", 1 + trades.len());
    }
    
    /// Publish real-time market events
    async fn publish_market_events(&mut self, order: &Order, trades: &[Trade]) {
        let mut conn = self.redis_manager.clone();
        
        // Publish orderbook update
        if let Some(orderbook) = self.orderbooks.get(&order.market_id) {
            let snapshot = self.create_orderbook_snapshot(orderbook);
            let event = MarketEvent::OrderBookUpdate {
                market_id: order.market_id,
                orderbook: snapshot,
            };
            
            let channel = format!("market_events:{}", order.market_id);
            let message = serde_json::to_string(&event).unwrap();
            
            // Now this will work because AsyncCommands is imported
            let _: Result<(), _> = conn.publish(channel, message).await;
        }
        
        // Publish ticker update
        if let Some(ticker) = self.tickers.get(&order.market_id) {
            let event = MarketEvent::TickerUpdate(ticker.clone());
            let channel = format!("ticker_events:{}", order.market_id);
            let message = serde_json::to_string(&event).unwrap();
            
            let _: Result<(), _> = conn.publish(channel, message).await;
        }
        
        // Publish trade events
        for trade in trades {
            let event = MarketEvent::TradeUpdate(trade.clone());
            let channel = format!("trade_events:{}", order.market_id);
            let message = serde_json::to_string(&event).unwrap();
            
            let _: Result<(), _> = conn.publish(channel, message).await;
        }
        
        tracing::info!("📡 Published market events for {} trades", trades.len());
    }
    
    /// Take periodic snapshots for recovery
    async fn take_snapshots(&mut self) {
        let mut conn = self.redis_manager.clone();
        let timestamp = Utc::now().timestamp_millis();
        
        // Snapshot orderbooks
        for (market_id, orderbook) in &self.orderbooks {
            let key = format!("snapshot:orderbook:{}", market_id);
            let _: Result<(), _> = redis::cmd("SET")
                .arg(key)
                .arg(serde_json::to_string(orderbook).unwrap())
                .arg("EX")
                .arg(3600) // 1 hour TTL
                .query_async(&mut conn)
                .await;
        }
        
        // Snapshot balances
        for (user_id, balance) in &self.balances {
            let key = format!("snapshot:balance:{}", user_id);
            let _: Result<(), _> = redis::cmd("SET")
                .arg(key)
                .arg(serde_json::to_string(balance).unwrap())
                .arg("EX")
                .arg(3600)
                .query_async(&mut conn)
                .await;
        }
        
        // Snapshot tickers
        for (market_id, ticker) in &self.tickers {
            let key = format!("snapshot:ticker:{}", market_id);
            let _: Result<(), _> = redis::cmd("SET")
                .arg(key)
                .arg(serde_json::to_string(ticker).unwrap())
                .arg("EX")
                .arg(300) // 5 minutes TTL (tickers change frequently)
                .query_async(&mut conn)
                .await;
        }
        
        tracing::info!("💾 Took snapshots at {}", timestamp);
    }
    
    // Helper methods...
    fn create_order_from_request(&self, req: crate::redis_manager::OrderRequest) -> Order {
        Order {
            id: Uuid::new_v4(),
            user_id: req.user_id,
            market_id: req.market_id,
            order_type: if req.order_type == "Buy" { OrderType::Buy } else { OrderType::Sell },
            order_kind: if req.order_kind == "Market" { OrderKind::Market } else { OrderKind::Limit },
            price: req.price,
            quantity: req.quantity,
            filled_quantity: 0,
            status: OrderStatus::Pending,
            created_at: req.timestamp,
        }
    }
    
    fn validate_order_balance(&self, order: &Order) -> bool {
        // Simplified balance validation
        // In real implementation, check user's token balances
        true // For now, always return true
    }
    
    async fn update_balances_from_trades(&mut self, trades: &[Trade]) {
        // Update user balances based on executed trades
        // Implementation depends on your token/balance structure
    }
    
    async fn update_ticker_from_trades(&mut self, trades: &[Trade]) {
        for trade in trades {
            let ticker = self.tickers.entry(trade.market_id).or_insert_with(|| MarketTicker {
                market_id: trade.market_id,
                last_price: trade.price,
                volume_24h: 0,
                high_24h: trade.price,
                low_24h: trade.price,
                change_24h: 0.0,
                timestamp: trade.timestamp,
            });
            
            ticker.last_price = trade.price;
            ticker.volume_24h += trade.quantity;
            ticker.timestamp = trade.timestamp;
            
            if trade.price > ticker.high_24h {
                ticker.high_24h = trade.price;
            }
            if trade.price < ticker.low_24h {
                ticker.low_24h = trade.price;
            }
        }
    }
    
    fn calculate_average_price(&self, trades: &[Trade]) -> Option<i64> {
        if trades.is_empty() {
            return None;
        }
        
        let total_value: i64 = trades.iter().map(|t| t.price * t.quantity).sum();
        let total_quantity: i64 = trades.iter().map(|t| t.quantity).sum();
        
        Some(total_value / total_quantity)
    }
    
    fn create_orderbook_snapshot(&self, orderbook: &OrderBook) -> OrderBookSnapshot {
        let bids: Vec<(i64, i64)> = orderbook.bids.iter()
            .map(|(&price, orders)| {
                let total_quantity = orders.iter().map(|o| o.quantity - o.filled_quantity).sum();
                (price, total_quantity)
            })
            .collect();
            
        let asks: Vec<(i64, i64)> = orderbook.asks.iter()
            .map(|(&price, orders)| {
                let total_quantity = orders.iter().map(|o| o.quantity - o.filled_quantity).sum();
                (price, total_quantity)
            })
            .collect();
            
        OrderBookSnapshot {
            bids,
            asks,
            timestamp: orderbook.last_updated,
        }
    }
}