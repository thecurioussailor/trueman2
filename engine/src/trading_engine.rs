use std::collections::{HashMap, BTreeMap, VecDeque};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tokio::time::{interval, Duration};
use redis::{aio::ConnectionManager, AsyncCommands, Commands}; 
use chrono::Utc;
use diesel::prelude::*;
use database::{establish_connection, Market, Token, schema::{markets, tokens}};

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
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
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

// Add this enhanced MarketInfo struct near the top with other structs (around line 72)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub id: Uuid,
    pub symbol: String,
    pub base_currency: TokenInfo,    // Full token info instead of just ID
    pub quote_currency: TokenInfo,   // Full token info instead of just ID
    pub min_order_size: i64,
    pub tick_size: i64,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_active: bool,
}

pub struct TradingEngine {
    // IN-MEMORY: Core trading data
    orderbooks: HashMap<Uuid, OrderBook>,
    balances: HashMap<Uuid, UserBalance>,
    tickers: HashMap<Uuid, MarketTicker>,
    markets: HashMap<Uuid, MarketInfo>,
    
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
            markets: HashMap::new(),

            redis_manager,
            operations_since_snapshot: 0,
            snapshot_interval: 10, // Snapshot every 10 operations
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
        println!("Order: {:?}", order);
        
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
        println!("Trades: {:?}, Orders: {:?}", trades, updated_order);
        
        // // // 4. Update in-memory state
        // self.update_balances_from_trades(&trades).await;
        // self.update_ticker_from_trades(&trades).await;
        
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
        let market_info = match self.markets.get(&order.market_id).cloned() {
            Some(market) => market,
            None => {
                tracing::error!("❌ Market not found: {}", order.market_id);
                order.status = OrderStatus::Cancelled;
                return (order, Vec::new());
            }
        };

        tracing::info!("🔄 Matching {:?} {:?} order: {} {} @ {:?} in market {}", 
            order.order_kind, 
            order.order_type,
            order.quantity, 
            market_info.base_currency.symbol,
            order.price, 
            market_info.symbol
       );

        // Create or get orderbook - but don't hold the reference
    if !self.orderbooks.contains_key(&order.market_id) {
        tracing::info!("📚 Creating new orderbook for market {}", market_info.symbol);
        self.orderbooks.insert(order.market_id, OrderBook {
            market_id: order.market_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_updated: Utc::now().timestamp_millis(),
        });
    }
        
        // Simplified matching logic (you can make this more sophisticated)
        // Now execute the matching logic
        let trades = match order.order_kind {
            OrderKind::Market => {
                self.execute_market_order(&mut order, &market_info).await
            }
            OrderKind::Limit => {
                self.execute_limit_order(&mut order, &market_info).await
            }
        };
           
        // Update balances based on trades
        if !trades.is_empty() {
            self.update_balances_from_trades(&trades, &market_info).await;
            self.update_ticker_from_trades(&trades, &market_info).await;
        }

        // Update orderbook timestamp
        if let Some(orderbook) = self.orderbooks.get_mut(&order.market_id) {
            orderbook.last_updated = Utc::now().timestamp_millis();
        }
        (order, trades)
    }
    
    async fn execute_market_order(&mut self, order: &mut Order, market_info: &MarketInfo) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_quantity = order.quantity;

        // Get the orderbook
        let orderbook = match self.orderbooks.get_mut(&order.market_id) {
            Some(ob) => ob,
            None => return trades, // Should not happen, but safe fallback
        };

        // Get the opposite side of the orderbook
        let opposite_side = match order.order_type {
            OrderType::Buy => &mut orderbook.asks,   // Buy orders match against asks (sellers)
            OrderType::Sell => &mut orderbook.bids, // Sell orders match against bids (buyers)
        };
        // For market orders, we iterate through prices in the best order:
        // - For buy orders: lowest ask prices first (ascending)
        // - For sell orders: highest bid prices first (descending)
        let mut prices_to_remove = Vec::new();

        // BTreeMap is naturally sorted, but we need different iteration order
        let price_levels: Vec<i64> = match order.order_type {
            OrderType::Buy => opposite_side.keys().cloned().collect(), // Ascending (best asks first)
            OrderType::Sell => opposite_side.keys().rev().cloned().collect(), // Descending (best bids first)
        };
        for price in price_levels {
            if remaining_quantity == 0 { break; }
    
            if let Some(order_queue) = opposite_side.get_mut(&price) {
                while let Some(mut matching_order) = order_queue.pop_front() {
                    if remaining_quantity == 0 { break; }
    
                    let available_quantity = matching_order.quantity - matching_order.filled_quantity;
                    let trade_quantity = remaining_quantity.min(available_quantity);
                    println!("{:?}", trade_quantity);
    
                    // Create trade
                    let trade = Trade {
                        id: Uuid::new_v4(),
                        market_id: order.market_id,
                        buyer_order_id: if matches!(order.order_type, OrderType::Buy) { 
                            order.id 
                        } else { 
                            matching_order.id 
                        },
                        seller_order_id: if matches!(order.order_type, OrderType::Sell) { 
                            order.id 
                        } else { 
                            matching_order.id 
                        },
                        buyer_user_id: if matches!(order.order_type, OrderType::Buy) { 
                            order.user_id 
                        } else { 
                            matching_order.user_id 
                        },
                        seller_user_id: if matches!(order.order_type, OrderType::Sell) { 
                            order.user_id 
                        } else { 
                            matching_order.user_id 
                        },
                        price,
                        quantity: trade_quantity, // This IS the quantity that was traded!
                        timestamp: Utc::now().timestamp_millis(),
                    };
    
                    trades.push(trade.clone());
    
                    // Update orders
                    order.filled_quantity += trade_quantity;
                    matching_order.filled_quantity += trade_quantity;
                    remaining_quantity -= trade_quantity;
    
                    tracing::info!("✅ Trade executed: {} {} @ {} in {}", 
                        trade_quantity, 
                        market_info.base_currency.symbol, 
                        price, 
                        market_info.symbol
                    );
    
                    // Update matching order status
                    if matching_order.filled_quantity >= matching_order.quantity {
                        matching_order.status = OrderStatus::Filled;
                        // Don't put it back in the queue - it's fully filled
                    } else {
                        matching_order.status = OrderStatus::PartiallyFilled;
                        order_queue.push_front(matching_order); // Put back partially filled order
                        break; // This price level still has liquidity
                    }
                }
    
                if order_queue.is_empty() {
                    prices_to_remove.push(price);
                }
            }
        }

        // Clean up empty price levels
        for price in prices_to_remove {
            opposite_side.remove(&price);
        }
        // Update order status
        order.status = if order.filled_quantity >= order.quantity {
            OrderStatus::Filled
        } else if order.filled_quantity > 0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Cancelled // Market order couldn't be filled
        };

        trades
    }

    /// Execute a limit order (try to match, then add remainder to orderbook)
    async fn execute_limit_order(&mut self, order: &mut Order, market_info: &MarketInfo) -> Vec<Trade> {
        println!("Executing limit order: {:?}", order);
        let mut trades = Vec::new();
        let order_price = order.price.expect("Limit order must have a price");
        let mut remaining_quantity = order.quantity;

        // Get the orderbook
        let orderbook = match self.orderbooks.get_mut(&order.market_id) {
            Some(ob) => ob,
            None => return trades, // Should not happen, but safe fallback
        };
        println!("Orderbook: {:?}", orderbook);

        // First, try to match against existing orders
        let opposite_side = match order.order_type {
            OrderType::Buy => &mut orderbook.asks,   // Buy orders match against asks
            OrderType::Sell => &mut orderbook.bids, // Sell orders match against bids
        };
        println!("Opposite side: {:?}", opposite_side);
        let mut prices_to_remove = Vec::new();

        // Get prices that can match with this limit order
        let matching_prices: Vec<i64> = match order.order_type {
            OrderType::Buy => {
                // Buy limit order matches with asks at or below the limit price
                opposite_side.keys()
                    .filter(|&&ask_price| ask_price <= order_price)
                    .cloned()
                    .collect()
            }
            OrderType::Sell => {
                // Sell limit order matches with bids at or above the limit price
                opposite_side.keys()
                    .filter(|&&bid_price| bid_price >= order_price)
                    .rev() // Start with highest bids
                    .cloned()
                    .collect()
            }
        };

        // Execute matches
        for price in matching_prices {
            if remaining_quantity == 0 { break; }

            if let Some(order_queue) = opposite_side.get_mut(&price) {
                while let Some(mut matching_order) = order_queue.pop_front() {
                    if remaining_quantity == 0 { break; }

                    let available_quantity = matching_order.quantity - matching_order.filled_quantity;
                    let trade_quantity = remaining_quantity.min(available_quantity);

                    // Create trade at the maker's price (price improvement for taker)
                    let trade = Trade {
                        id: Uuid::new_v4(),
                        market_id: order.market_id,
                        buyer_order_id: if matches!(order.order_type, OrderType::Buy) { 
                            order.id 
                        } else { 
                            matching_order.id 
                        },
                        seller_order_id: if matches!(order.order_type, OrderType::Sell) { 
                            order.id 
                        } else { 
                            matching_order.id 
                        },
                        buyer_user_id: if matches!(order.order_type, OrderType::Buy) { 
                            order.user_id 
                        } else { 
                            matching_order.user_id 
                        },
                        seller_user_id: if matches!(order.order_type, OrderType::Sell) { 
                            order.user_id 
                        } else { 
                            matching_order.user_id 
                        },
                        price,
                        quantity: trade_quantity, // This IS the quantity that was traded!
                        timestamp: Utc::now().timestamp_millis(),
                    };

                    trades.push(trade.clone());

                    // Update orders
                    order.filled_quantity += trade_quantity;
                    matching_order.filled_quantity += trade_quantity;
                    remaining_quantity -= trade_quantity;

                    tracing::info!("✅ Limit order trade: {} {} @ {} in {}", 
                        trade_quantity, 
                        market_info.base_currency.symbol, 
                        price, 
                        market_info.symbol
                    );

                    // Update matching order status
                    if matching_order.filled_quantity >= matching_order.quantity {
                        matching_order.status = OrderStatus::Filled;
                    } else {
                        matching_order.status = OrderStatus::PartiallyFilled;
                        order_queue.push_front(matching_order);
                        break;
                    }
                }

                if order_queue.is_empty() {
                    prices_to_remove.push(price);
                }
            }
        }

        // Clean up empty price levels
        for price in prices_to_remove {
            opposite_side.remove(&price);
        }

        // Add remaining quantity to the orderbook if not fully filled
        if remaining_quantity > 0 {
            let same_side = match order.order_type {
                OrderType::Buy => &mut orderbook.bids,
                OrderType::Sell => &mut orderbook.asks,
            };

            // Create a new order for the remaining quantity
            let mut remaining_order = order.clone();
            remaining_order.quantity = remaining_quantity;
            remaining_order.filled_quantity = 0;
            remaining_order.status = OrderStatus::Pending;

            let price_level = same_side.entry(order_price).or_insert_with(VecDeque::new);
            price_level.push_back(remaining_order);

            tracing::info!("📋 Added {} {} to {} orderbook at price {} in {}", 
                remaining_quantity, 
                market_info.base_currency.symbol,
                if matches!(order.order_type, OrderType::Buy) { "bid" } else { "ask" },
                order_price,
                market_info.symbol
            );
        }

        // Update order status
        order.status = if order.filled_quantity >= order.quantity {
            OrderStatus::Filled
        } else if order.filled_quantity > 0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Pending
        };

        trades
    }
    
    /// Update user balances based on executed trades
    async fn update_balances_from_trades(&mut self, trades: &[Trade], market_info: &MarketInfo) {
        for trade in trades {

            let buyer_id = trade.buyer_user_id;  // Now using User ID
            let seller_id = trade.seller_user_id; 

                self.update_user_balance(
                    buyer_id, 
                    market_info.quote_currency.id, 
                    -(trade.price * trade.quantity), // Decrease USDC
                    0
                ).await;
                
                self.update_user_balance(
                    buyer_id, 
                    market_info.base_currency.id, 
                    trade.quantity, // Increase SOL/BTC/ETH
                    0
                ).await;

                // Update seller balance: increase quote currency, decrease base currency
                self.update_user_balance(
                    seller_id, 
                    market_info.quote_currency.id, 
                    trade.price * trade.quantity, // Increase USDC
                    0
                ).await;
                
                self.update_user_balance(
                    seller_id, 
                    market_info.base_currency.id, 
                    -trade.quantity, // Decrease SOL/BTC/ETH
                    0
                ).await;

                tracing::info!("💰 Updated balances for trade: {} {} @ {} between users {} and {}", 
                    trade.quantity, 
                    market_info.base_currency.symbol, 
                    trade.price,
                    buyer_id,
                    seller_id
                );
            }
        }

    /// Update individual user balance
    async fn update_user_balance(&mut self, user_id: Uuid, token_id: Uuid, amount_delta: i64, locked_delta: i64) {
        let user_balance = self.balances.entry(user_id).or_insert_with(|| UserBalance {
            user_id,
            token_balances: HashMap::new(),
        });

        let token_balance = user_balance.token_balances.entry(token_id).or_insert_with(|| TokenBalance {
            available: 0,
            locked: 0,
        });

        token_balance.available += amount_delta;
        token_balance.locked += locked_delta;

        // Queue balance update for database
        let balance_event = DBUpdateEvent::BalanceUpdated {
            user_id,
            token_id,
            available: token_balance.available,
            locked: token_balance.locked,
        };

        // Send to db-updater queue
        let mut conn = self.redis_manager.clone();
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("balance_updated")
            .arg("data")
            .arg(serde_json::to_string(&balance_event).unwrap())
            .query_async(&mut conn)
            .await;
}
    /// Update market ticker from trades
    async fn update_ticker_from_trades(&mut self, trades: &[Trade], market_info: &MarketInfo) {
        if trades.is_empty() { return; }

        let ticker = self.tickers.entry(trades[0].market_id).or_insert_with(|| MarketTicker {
            market_id: trades[0].market_id,
            last_price: 0,
            volume_24h: 0,
            high_24h: 0,
            low_24h: 0,
            change_24h: 0.0,
            timestamp: Utc::now().timestamp_millis(),
        });

        // Update with latest trade price
        if let Some(last_trade) = trades.last() {
            ticker.last_price = last_trade.price;
            ticker.timestamp = last_trade.timestamp;
            
            // Update high/low (simplified - in production you'd track 24h period)
            if ticker.high_24h == 0 || last_trade.price > ticker.high_24h {
                ticker.high_24h = last_trade.price;
            }
            if ticker.low_24h == 0 || last_trade.price < ticker.low_24h {
                ticker.low_24h = last_trade.price;
            }

            // Add volume
            for trade in trades {
                ticker.volume_24h += trade.quantity;
            }

            tracing::info!("📊 Updated ticker for {}: last_price={}, volume_24h={}", 
                market_info.symbol, 
                ticker.last_price, 
                ticker.volume_24h
            );
        }
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
    tracing::info!("💾 Starting snapshot process...");
    tracing::info!("📊 Current state: {} balances, {} orderbooks, {} tickers", 
        self.balances.len(), 
        self.orderbooks.len(), 
        self.tickers.len()
    );
    
    let mut conn = self.redis_manager.clone();
    let timestamp = Utc::now().timestamp_millis();
    let mut snapshot_count = 0;
    
    // Snapshot balances (most important)
    for (user_id, balance) in &self.balances {
        let key = format!("snapshot:balance:{}", user_id);
        tracing::debug!("💾 Saving balance snapshot for user: {}", user_id);
        
        match serde_json::to_string(balance) {
            Ok(balance_json) => {
                match redis::cmd("SET")
                    .arg(&key)
                    .arg(&balance_json)
                    .arg("EX")
                    .arg(3600)
                    .query_async::<_, ()>(&mut conn)
                    .await
                {
                    Ok(_) => {
                        snapshot_count += 1;
                        tracing::debug!("✅ Balance snapshot saved: {}", key);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to save balance snapshot {}: {}", key, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to serialize balance for {}: {}", user_id, e);
            }
        }
    }
    
    // Snapshot orderbooks
    for (market_id, orderbook) in &self.orderbooks {
        let key = format!("snapshot:orderbook:{}", market_id);
        tracing::debug!("💾 Saving orderbook snapshot for market: {}", market_id);
        
        match serde_json::to_string(orderbook) {
            Ok(orderbook_json) => {
                match redis::cmd("SET")
                    .arg(&key)
                    .arg(&orderbook_json)
                    .arg("EX")
                    .arg(3600)
                    .query_async::<_, ()>(&mut conn)
                    .await
                {
                    Ok(_) => {
                        snapshot_count += 1;
                        tracing::debug!("✅ Orderbook snapshot saved: {}", key);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to save orderbook snapshot {}: {}", key, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to serialize orderbook for {}: {}", market_id, e);
            }
        }
    }
    
    // Snapshot tickers
    for (market_id, ticker) in &self.tickers {
        let key = format!("snapshot:ticker:{}", market_id);
        tracing::debug!("💾 Saving ticker snapshot for market: {}", market_id);
        
        match serde_json::to_string(ticker) {
            Ok(ticker_json) => {
                match redis::cmd("SET")
                    .arg(&key)
                    .arg(&ticker_json)
                    .arg("EX")
                    .arg(300) // 5 minutes TTL
                    .query_async::<_, ()>(&mut conn)
                    .await
                {
                    Ok(_) => {
                        snapshot_count += 1;
                        tracing::debug!("✅ Ticker snapshot saved: {}", key);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to save ticker snapshot {}: {}", key, e);
                    }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to serialize ticker for {}: {}", market_id, e);
                }
            }
        }
    
        tracing::info!("✅ Snapshot process completed: {} snapshots saved at {}", snapshot_count, timestamp);
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
        tracing::info!("💾 Taking snapshots");
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

     // Add this method to load markets from database
    pub async fn load_markets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        
        
        let mut connection = establish_connection();
        
        // Load all active markets
        let markets_result = markets::table
            .filter(markets::is_active.eq(true))
            .select(Market::as_select())
            .load::<Market>(&mut connection)?;
        
        tracing::info!("📊 Loading {} active markets from database...", markets_result.len());
        
        for market in markets_result {
            // Get base token - following your exact pattern
            let base_token = match tokens::table
                .filter(tokens::id.eq(market.base_currency_id))
                .filter(tokens::is_active.eq(true))
                .first::<Token>(&mut connection) {
                Ok(token) => TokenInfo {
                    id: token.id,
                    symbol: token.symbol,
                    name: token.name,
                    decimals: token.decimals,
                    is_active: token.is_active,
                },
                Err(_) => {
                    tracing::warn!("⚠️  Skipping market {} - base token not found or inactive", market.symbol);
                    continue;
                }
            };

            // Get quote token - following your exact pattern
            let quote_token = match tokens::table
                .filter(tokens::id.eq(market.quote_currency_id))
                .filter(tokens::is_active.eq(true))
                .first::<Token>(&mut connection) {
                Ok(token) => TokenInfo {
                    id: token.id,
                    symbol: token.symbol,
                    name: token.name,
                    decimals: token.decimals,
                    is_active: token.is_active,
                },
                Err(_) => {
                    tracing::warn!("⚠️  Skipping market {} - quote token not found or inactive", market.symbol);
                    continue;
                }
            };

            // Create MarketInfo with full token details
            let market_info = MarketInfo {
                id: market.id,
                symbol: market.symbol.clone(),
                base_currency: base_token,
                quote_currency: quote_token,
                min_order_size: market.min_order_size,
                tick_size: market.tick_size,
                is_active: market.is_active,
                created_at: market.created_at,
            };
            
            self.markets.insert(market.id, market_info.clone());
            tracing::info!("📈 Loaded market: {} ({}/{}) - ID: {}", 
                market.symbol, 
                market_info.base_currency.symbol, 
                market_info.quote_currency.symbol,
                market.id
            );
        }
        
        tracing::info!("✅ Successfully loaded {} markets with token details", self.markets.len());
        Ok(())
    }

    // Enhanced balance validation with token symbols for better logging
    fn validate_order_balance(&self, order: &Order) -> bool {
        // Get market information
        let market = match self.markets.get(&order.market_id) {
            Some(market) => market,
            None => {
                tracing::error!("❌ Market not found: {}", order.market_id);
                return false;
            }
        };
        
        // Get user balance
        let user_balance = match self.balances.get(&order.user_id) {
            Some(balance) => balance,
            None => {
                tracing::warn!("❌ No balance found for user: {} in market {}", 
                    order.user_id, market.symbol);
                return false;
            }
        };
        
        match order.order_type {
            OrderType::Buy => {
                // For buy orders, user needs sufficient quote currency (usually USDC)
                let required_token_id = market.quote_currency.id;
                let required_amount = match order.price {
                    Some(price) => price * order.quantity,
                    None => {
                        // Market order - estimate against current best ask price
                        let estimated_price = self.estimate_market_buy_price(order.market_id, order.quantity);
                        match estimated_price {
                            Some(price) => price * order.quantity,
                            None => {
                                tracing::warn!("❌ No liquidity available for market buy in {} market", 
                                    market.symbol);
                                return false;
                            }
                        }
                    }
                };
                
                if let Some(token_balance) = user_balance.token_balances.get(&required_token_id) {
                    let has_sufficient = token_balance.available >= required_amount;
                    if !has_sufficient {
                        tracing::warn!(
                            "❌ Insufficient {} balance for BUY order in {} market. Required: {}, Available: {}", 
                            market.quote_currency.symbol,
                            market.symbol,
                            required_amount, 
                            token_balance.available
                        );
                    } else {
                        tracing::info!(
                            "✅ Sufficient {} balance for BUY order in {} market. Required: {}, Available: {}", 
                            market.quote_currency.symbol,
                            market.symbol,
                            required_amount, 
                            token_balance.available
                        );
                    }
                    has_sufficient
                } else {
                    tracing::warn!("❌ No {} balance found for user in {} market", 
                        market.quote_currency.symbol, market.symbol);
                    false
                }
            }
            OrderType::Sell => {
                // For sell orders, user needs sufficient base currency (SOL/BTC/ETH)
                let required_token_id = market.base_currency.id;
                let required_amount = order.quantity;
                
                if let Some(token_balance) = user_balance.token_balances.get(&required_token_id) {
                    let has_sufficient = token_balance.available >= required_amount;
                    if !has_sufficient {
                        tracing::warn!(
                            "❌ Insufficient {} balance for SELL order in {} market. Required: {}, Available: {}", 
                            market.base_currency.symbol,
                            market.symbol,
                            required_amount, 
                            token_balance.available
                        );
                    } else {
                        tracing::info!(
                            "✅ Sufficient {} balance for SELL order in {} market. Required: {}, Available: {}", 
                            market.base_currency.symbol,
                            market.symbol,
                            required_amount, 
                            token_balance.available
                        );
                    }
                    has_sufficient
                } else {
                    tracing::warn!("❌ No {} balance found for user in {} market", 
                        market.base_currency.symbol, market.symbol);
                    false
                }
            }
        }
    }
    fn estimate_market_buy_price(&self, market_id: Uuid, quantity: i64) -> Option<i64> {
        let orderbook = self.orderbooks.get(&market_id)?;
        let mut remaining_quantity = quantity;
        let mut total_cost = 0i64;
        
        // Calculate cost by walking through asks (ascending price order)
        for (&price, orders) in &orderbook.asks {
            if remaining_quantity <= 0 { break; }
            
            let available_at_price: i64 = orders.iter()
                .map(|o| o.quantity - o.filled_quantity)
                .sum();
            
            let quantity_to_buy = remaining_quantity.min(available_at_price);
            total_cost += price * quantity_to_buy;
            remaining_quantity -= quantity_to_buy;
        }
        
        if remaining_quantity > 0 {
            // Not enough liquidity - return conservative estimate using highest price found
            if let Some((&highest_price, _)) = orderbook.asks.iter().last() {
                Some(highest_price + (highest_price / 10)) // Add 10% buffer
            } else {
                None // No asks available
            }
        } else {
            // We can fulfill the order - return average price
            Some(total_cost / quantity)
        }
    }
}
