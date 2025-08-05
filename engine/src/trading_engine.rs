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
        
        tracing::info!("🔄 Processing order: {} {} {} @ {:?}", 
            order_request.order_type,
            order_request.quantity,
            order_request.order_kind,
            order_request.price
        );
        
        // 1. Convert request to internal order
        let order = self.create_order_from_request(order_request.clone());
        
        // 2. Validate balances (now we have balance data!)
        if !self.validate_order_balance(&order) {
            tracing::warn!("❌ Order rejected: Insufficient balance");
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
        
        tracing::info!("✅ Order processed successfully: {} trades executed", trades.len());
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
    
    /// Enhanced balance validation (now actually checks in-memory balances)
    fn validate_order_balance(&self, order: &Order) -> bool {
        if let Some(user_balance) = self.balances.get(&order.user_id) {
            // For now, simplified validation
            // In real implementation, you'd:
            // 1. Get the market's base/quote tokens
            // 2. Calculate required balance based on order type
            // 3. Check if user has sufficient available balance
            
            // Simplified: assume we're checking if user has any balance
            return !user_balance.token_balances.is_empty();
        }
        
        // No balance found for user
        false
    }
    
    /// Update balances based on executed trades
    async fn update_balances_from_trades(&mut self, trades: &[Trade]) {
        for trade in trades {
            // Update buyer balance (reduce quote token, increase base token)
            // Update seller balance (reduce base token, increase quote token)
            // For now, this is simplified - you'd need market token pair info
            
            tracing::debug!("Updating balances for trade: {} @ {}", trade.quantity, trade.price);
            // TODO: Implement actual balance updates based on your token pair logic
        }
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

    /// 💰 Process balance requests (deposits, withdrawals, queries)
    pub async fn process_balance_request(
        &mut self,
        balance_request: crate::redis_manager::BalanceRequest
    ) -> crate::redis_manager::BalanceResponse {
        match balance_request.operation {
            crate::redis_manager::BalanceOperation::Deposit => {
                self.process_deposit(balance_request).await
            }
            crate::redis_manager::BalanceOperation::Withdraw => {
                self.process_withdrawal(balance_request).await
            }
            crate::redis_manager::BalanceOperation::GetBalances => {
                self.get_user_balances(balance_request).await
            }
        }
    }
    
    /// Process deposit request
    async fn process_deposit(&mut self, request: crate::redis_manager::BalanceRequest) -> crate::redis_manager::BalanceResponse {
        tracing::info!("💳 Processing deposit: {} {} for user {}", 
            request.amount, 
            request.token_id, 
            request.user_id
        );
    
        // Get or create user balance and update it
        let (new_balance, locked_amount) = {
            // Get or create user balance
            let user_balance = self.balances.entry(request.user_id).or_insert_with(|| UserBalance {
                user_id: request.user_id,
                token_balances: HashMap::new(),
            });
            
            // Add to available balance
            let token_balance = user_balance.token_balances.entry(request.token_id).or_insert_with(|| TokenBalance {
                available: 0,
                locked: 0,
            });
            
            token_balance.available += request.amount;
            
            // Extract the values we need
            (token_balance.available, token_balance.locked)
        }; // 🔑 The mutable borrow to self.balances is released here
        
        // Now we can safely call methods that need mutable self
        // Queue database update (async, non-blocking)
        self.queue_balance_db_update(
            request.user_id, 
            request.token_id, 
            new_balance, 
            locked_amount
        ).await;
        
        // Take snapshot if needed
        self.operations_since_snapshot += 1;
        if self.operations_since_snapshot >= self.snapshot_interval {
            self.take_snapshots().await;
            self.operations_since_snapshot = 0;
        }
        
        tracing::info!("✅ Deposit processed successfully. New balance: {}", new_balance);
        
        crate::redis_manager::BalanceResponse {
            request_id: request.request_id,
            success: true,
            message: format!("Successfully deposited {}", request.amount),
            new_balance,
            balances: None,
        }
    }
    
    /// Process withdrawal request
    async fn process_withdrawal(&mut self, request: crate::redis_manager::BalanceRequest) -> crate::redis_manager::BalanceResponse {
        tracing::info!("💸 Processing withdrawal: {} {} for user {}", 
            request.amount, 
            request.token_id, 
            request.user_id
        );
    
        // Check if user has sufficient balance and get the new balance value
        let withdrawal_result = {
            if let Some(user_balance) = self.balances.get_mut(&request.user_id) {
                if let Some(token_balance) = user_balance.token_balances.get_mut(&request.token_id) {
                    let available_balance = token_balance.available - token_balance.locked;
                    
                    if available_balance >= request.amount {
                        // Process withdrawal
                        token_balance.available -= request.amount;
                        let new_balance = token_balance.available;
                        let locked_amount = token_balance.locked;
                        
                        // Return success with the new balance values
                        Some((new_balance, locked_amount))
                    } else {
                        tracing::warn!("❌ Insufficient balance for withdrawal. Available: {}, Requested: {}", 
                            available_balance, request.amount);
                        None
                    }
                } else {
                    tracing::warn!("❌ No token balance found for token {}", request.token_id);
                    None
                }
            } else {
                tracing::warn!("❌ No user balance found for user {}", request.user_id);
                None
            }
        }; // 🔑 The mutable borrow to self.balances is released here
    
        // Now we can safely call methods that need mutable self
        match withdrawal_result {
            Some((new_balance, locked_amount)) => {
                // Queue database update (now safe because previous borrow is released)
                self.queue_balance_db_update(
                    request.user_id, 
                    request.token_id, 
                    new_balance, 
                    locked_amount
                ).await;
                
                // Take snapshot if needed
                self.operations_since_snapshot += 1;
                if self.operations_since_snapshot >= self.snapshot_interval {
                    self.take_snapshots().await;
                    self.operations_since_snapshot = 0;
                }
                
                tracing::info!("✅ Withdrawal processed successfully. New balance: {}", new_balance);
                
                crate::redis_manager::BalanceResponse {
                    request_id: request.request_id,
                    success: true,
                    message: format!("Successfully withdrew {}", request.amount),
                    new_balance,
                    balances: None,
                }
            }
            None => {
                // Insufficient balance or user not found
                crate::redis_manager::BalanceResponse {
                    request_id: request.request_id,
                    success: false,
                    message: "Insufficient balance or user not found".to_string(),
                    new_balance: 0,
                    balances: None,
                }
            }
        }
    }
    
    /// Get user balances for all tokens
    async fn get_user_balances(&self, request: crate::redis_manager::BalanceRequest) -> crate::redis_manager::BalanceResponse {
        tracing::info!("📊 Getting balances for user {}", request.user_id);

        let balances = if let Some(user_balance) = self.balances.get(&request.user_id) {
            user_balance.token_balances.iter().map(|(&token_id, balance)| {
                crate::redis_manager::UserTokenBalance {
                    token_id,
                    available: balance.available,
                    locked: balance.locked,
                }
            }).collect()
        } else {
            tracing::info!("No balances found for user {}", request.user_id);
            Vec::new()
        };
        
        tracing::info!("✅ Retrieved {} token balances for user", balances.len());
        
        crate::redis_manager::BalanceResponse {
            request_id: request.request_id,
            success: true,
            message: "Balances retrieved successfully".to_string(),
            new_balance: 0, // Not applicable for balance queries
            balances: Some(balances),
        }
    }
    
    /// Queue balance update for database persistence
    async fn queue_balance_db_update(&mut self, user_id: Uuid, token_id: Uuid, available: i64, locked: i64) {
        let balance_event = DBUpdateEvent::BalanceUpdated {
            user_id,
            token_id, 
            available,
            locked,
        };
        
        let mut conn = self.redis_manager.clone();
        let balance_json = serde_json::to_string(&balance_event).unwrap();
        
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("balance_updated")
            .arg("data")
            .arg(&balance_json)
            .arg("timestamp")
            .arg(chrono::Utc::now().timestamp_millis())
            .query_async(&mut conn)
            .await;
            
        tracing::debug!("📤 Queued balance DB update for user {} token {}", user_id, token_id);
    }

    /// Load initial balances from database snapshots
    pub async fn load_balance_snapshots(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.redis_manager.clone();
        tracing::info!("📥 Loading balance snapshots from Redis...");
        
        // Get all snapshot keys
        let snapshot_keys: Vec<String> = redis::cmd("KEYS")
            .arg("snapshot:balance:*")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        
        let mut loaded_count = 0;
        
        for key in snapshot_keys {
            if let Ok(snapshot_data) = redis::cmd("GET")
                .arg(&key)
                .query_async::<_, String>(&mut conn)
                .await
            {
                if let Ok(user_balance) = serde_json::from_str::<UserBalance>(&snapshot_data) {
                    self.balances.insert(user_balance.user_id, user_balance);
                    loaded_count += 1;
                }
            }
        }
        
        tracing::info!("✅ Loaded {} balance snapshots", loaded_count);
        Ok(())
    }

}