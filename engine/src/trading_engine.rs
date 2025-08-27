use std::collections::{HashMap, BTreeMap, VecDeque};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tokio::time::{interval, Duration};
use redis::{aio::ConnectionManager, AsyncCommands, Commands};
use primitive_types::U256;
use chrono::Utc;
use diesel::prelude::*;
use database::{establish_connection, Market, Token, schema::{markets, tokens}};
use crate::decimal_utils::{
    price_from_atomic_units, quantity_from_atomic_units,
    EnhancedDepthUpdate, EnhancedMarketTicker, convert_ticker_to_decimal,
    convert_trade_to_decimal, format_price_to_tick_precision, format_quantity_to_precision
};
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
pub struct Reservation {
    pub token_id: Uuid,
    pub amount: i64
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

// Update the DepthUpdate struct to include both decimal and atomic values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthUpdate {
    pub market_id: Uuid,
    pub seq: u64,
    pub ts: i64,
    pub bids: Vec<(f64, f64)>,        // Decimal values (price, quantity)
    pub asks: Vec<(f64, f64)>,        // Decimal values (price, quantity)
    pub bids_atomic: Vec<(i64, i64)>, // Atomic values for debugging
    pub asks_atomic: Vec<(i64, i64)>, // Atomic values for debugging
}

pub struct TradingEngine {
    // IN-MEMORY: Core trading data
    orderbooks: HashMap<Uuid, OrderBook>,
    balances: HashMap<Uuid, UserBalance>,
    tickers: HashMap<Uuid, MarketTicker>,
    markets: HashMap<Uuid, MarketInfo>,
    depth_seq: HashMap<Uuid, u64>,
    ticker_seq: HashMap<Uuid, u64>,
    
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
            depth_seq: HashMap::new(),
            ticker_seq: HashMap::new(),

            redis_manager,
            operations_since_snapshot: 0,
            snapshot_interval: 10, // Snapshot every 10 operations
        })
    }
    
    // KEEP THIS FUNCTION
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
        
        // 2) Validate + lock funds BEFORE persisting created
        let market = match self.markets.get(&order.market_id).cloned() {
            Some(m) => m,
            None => {
                tracing::warn!("❌ Market not found");
                return crate::redis_manager::OrderResponse {
                    request_id: order_request.request_id, success: false, status: "REJECTED".to_string(),
                    order_id: None, message: "Market not found".to_string(),
                    filled_quantity: None, remaining_quantity: None, average_price: None, trades: None
                };
            }
        };

        // 2. Validate balances (now we have balance data!)
        println!("Validating and locking order balance order: {:?}", order);
        let reservation = match self.validate_and_lock_order_balance(&order).await {
            Ok(reservation) => reservation,
            Err(msg) => {
                return crate::redis_manager::OrderResponse {
                    request_id: order_request.request_id,
                    success: false,
                    status: "REJECTED".to_string(),
                    order_id: None,
                    message: msg,
                    filled_quantity: None,
                    remaining_quantity: None,
                    average_price: None,
                    trades: None,
                };
            }
        };

        self.queue_order_created(&order).await;
        // 3. Execute matching in memory
        let (updated_order, matched_orders, trades) = self.match_order(order).await;
        println!("Trades: {:?}, Orders: {:?}", trades, updated_order);
        
        if matches!(updated_order.order_kind, OrderKind::Market) {
            match updated_order.order_type {
                OrderType::Buy => {
                    let market = self.markets.get(&updated_order.market_id).unwrap();
                    let mut executed_cost = 0i64;
                    
                    // Calculate executed cost using safe_multiply_divide for each trade
                    for trade in &trades {
                        match self.safe_multiply_divide(trade.price, trade.quantity, market.base_currency.decimals) {
                            Ok(cost) => executed_cost += cost,
                            Err(e) => {
                                tracing::error!("Error calculating trade cost for refund: {}", e);
                                // Continue with other trades, don't fail the entire order
                            }
                        }
                    }
                    if reservation.token_id == self.markets.get(&updated_order.market_id).unwrap().quote_currency.id {
                        let refund = reservation.amount.saturating_sub(executed_cost);
                        if refund > 0 {
                            self.unlock(updated_order.user_id, reservation.token_id, refund).await;
                        }
                    }
                }
                OrderType::Sell => {
                    let remaining = updated_order.quantity - updated_order.filled_quantity;
                    if remaining > 0 && reservation.token_id == self.markets.get(&updated_order.market_id).unwrap().base_currency.id {
                        self.unlock(updated_order.user_id, reservation.token_id, remaining).await;
                    }
                }
            }
        }

        let market_id = updated_order.market_id;
        println!("Publishing depth for market {}", market_id);
        self.publish_depth(market_id).await;
        if !trades.is_empty() {
            self.publish_depth(market_id).await;
            self.publish_trades(market_id, &trades).await;   // new trades
            self.publish_ticker(market_id).await;            // last price from trades
        }
        
        // 5. Queue database updates (async, non-blocking)
        self.queue_db_updates(&updated_order, &matched_orders, &trades).await;
        
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
            average_price: self.calculate_average_price(&trades, market_id),
            trades: Some(trades.into_iter().map(|t| crate::redis_manager::TradeInfo {
                trade_id: t.id,
                price: t.price,
                quantity: t.quantity,
                timestamp: t.timestamp,
            }).collect()),
        }
    }
    
    // KEEP THIS FUNCTION
    async fn match_order(&mut self, mut order: Order) -> (Order, Vec<Order>, Vec<Trade>) {
        let market_info = match self.markets.get(&order.market_id).cloned() {
            Some(market) => market,
            None => {
                tracing::error!("❌ Market not found: {}", order.market_id);
                order.status = OrderStatus::Cancelled;
                return (order, Vec::new(), Vec::new());
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
        
        let (trades, matched_orders) = match order.order_kind {
            OrderKind::Market => {
                self.execute_market_order(&mut order, &market_info).await
            }
            OrderKind::Limit => {
                self.execute_limit_order(&mut order, &market_info).await
            }
        };

        println!("Trades*********{:?}", trades);
           
        // Update balances based on trades
        if !trades.is_empty() {
            self.update_balances_from_trades(&trades, &market_info).await;
            self.update_ticker_from_trades(&trades, &market_info).await;
        }

        // Update orderbook timestamp
        if let Some(orderbook) = self.orderbooks.get_mut(&order.market_id) {
            orderbook.last_updated = Utc::now().timestamp_millis();
        }
        (order, matched_orders, trades)
    }
    // KEEP THIS FUNCTION
    fn build_depth(&self, market_id: Uuid, top_n: usize) -> Option<(Vec<(i64,i64)>, Vec<(i64,i64)>)> {
        let ob = self.orderbooks.get(&market_id)?;
        
        let bids = ob.bids.iter().rev().take(top_n).filter_map(|(&price, orders)| {
            let mut total_quantity = 0i64;
            for order in orders {
                let remaining = order.quantity - order.filled_quantity;
                match total_quantity.checked_add(remaining) {
                    Some(new_total) => total_quantity = new_total,
                    None => {
                        tracing::warn!("Quantity overflow in depth for market {} at price {}", market_id, price);
                        return None;
                    }
                }
            }
            Some((price, total_quantity))
        }).collect();
        
        let asks = ob.asks.iter().take(top_n).filter_map(|(&price, orders)| {
            let mut total_quantity = 0i64;
            for order in orders {
                let remaining = order.quantity - order.filled_quantity;
                match total_quantity.checked_add(remaining) {
                    Some(new_total) => total_quantity = new_total,
                    None => {
                        tracing::warn!("Quantity overflow in depth for market {} at price {}", market_id, price);
                        return None;
                    }
                }
            }
            Some((price, total_quantity))
        }).collect();
        
        Some((bids, asks))
    }

    // Update the publish_depth function
async fn publish_depth(&mut self, market_id: Uuid) {
    println!("Publishing depth for market {}", market_id);
    if let Some((bids_atomic, asks_atomic)) = self.build_depth(market_id, 50) { // top 50
        if let Some(market_info) = self.markets.get(&market_id) {
            // Convert to decimal format
            let bids_decimal: Vec<(f64, f64)> = bids_atomic.iter().map(|&(price, quantity)| {
                let raw_price = price_from_atomic_units(price, market_info);
                let raw_quantity = quantity_from_atomic_units(quantity, market_info);
                let formatted_price = format_price_to_tick_precision(raw_price, market_info.tick_size, market_info.quote_currency.decimals);
                let formatted_quantity = format_quantity_to_precision(raw_quantity, market_info.min_order_size, market_info.base_currency.decimals);
                (formatted_price, formatted_quantity)
            }).collect();

            let asks_decimal: Vec<(f64, f64)> = asks_atomic.iter().map(|&(price, quantity)| {
                let raw_price = price_from_atomic_units(price, market_info);
                let raw_quantity = quantity_from_atomic_units(quantity, market_info);
                let formatted_price = format_price_to_tick_precision(raw_price, market_info.tick_size, market_info.quote_currency.decimals);
                let formatted_quantity = format_quantity_to_precision(raw_quantity, market_info.min_order_size, market_info.base_currency.decimals);
                (formatted_price, formatted_quantity)
            }).collect();

            let seq = self.depth_seq.entry(market_id).and_modify(|s| *s += 1).or_insert(1);
            let enhanced_update = EnhancedDepthUpdate {
                market_id,
                seq: *seq,
                ts: chrono::Utc::now().timestamp_millis(),
                bids: bids_decimal,
                asks: asks_decimal,
                bids_atomic,
                asks_atomic,
            };
            
            let mut conn = self.redis_manager.clone();
            let _: Result<(), _> = conn.publish(
                format!("depth:{}", market_id),
                serde_json::to_string(&enhanced_update).unwrap()
            ).await;
        }
    }
}

    // Update the publish_ticker function
async fn publish_ticker(&mut self, market_id: Uuid) {
    if let Some(atomic_ticker) = self.tickers.get(&market_id) {
        if let Some(market_info) = self.markets.get(&market_id) {
            let enhanced_ticker = convert_ticker_to_decimal(atomic_ticker, market_info);
            
            let _seq = self.ticker_seq.entry(market_id).and_modify(|s| *s += 1).or_insert(1);
            let mut conn = self.redis_manager.clone();
            let _: Result<(), _> = conn.publish(
                format!("ticker:{}", market_id),
                serde_json::to_string(&enhanced_ticker).unwrap()
            ).await;
        }
    }
}

    
    async fn publish_trades(&mut self, market_id: Uuid, trades: &[Trade]) {
        if trades.is_empty() { return; }
        if let Some(market_info) = self.markets.get(&market_id) {
            let mut conn = self.redis_manager.clone();
            for trade in trades {
                // Convert trade to decimal format
                let enhanced_trade = convert_trade_to_decimal(trade, market_info);
                
                let _: Result<(), _> = conn.publish(
                    format!("trades:{}", market_id),
                    serde_json::to_string(&enhanced_trade).unwrap()
                ).await;
            }
        }
    }
    // KEEP THIS FUNCTION
    async fn execute_market_order(&mut self, order: &mut Order, market_info: &MarketInfo) -> (Vec<Trade>, Vec<Order>) {
        let mut trades = Vec::new();
        let mut matched_orders = Vec::new();
        let mut remaining_quantity = order.quantity;

        // Get the orderbook
        let orderbook = match self.orderbooks.get_mut(&order.market_id) {
            Some(ob) => ob,
            None => return (trades, matched_orders), // Should not happen, but safe fallback
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
                        matched_orders.push(matching_order.clone());
                        // Don't put it back in the queue - it's fully filled
                    } else {
                        matching_order.status = OrderStatus::PartiallyFilled;
                        matched_orders.push(matching_order.clone());
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

        (trades, matched_orders)
    }

    // KEEP THIS FUNCTION
    async fn execute_limit_order(&mut self, order: &mut Order, market_info: &MarketInfo) -> (Vec<Trade>, Vec<Order>) {
        println!("Executing limit order: {:?}", order);
        let mut trades = Vec::new();
        let mut matched_orders = Vec::new();
        let order_price = order.price.expect("Limit order must have a price");
        let mut remaining_quantity = order.quantity;

        // Get the orderbook
        let orderbook = match self.orderbooks.get_mut(&order.market_id) {
            Some(ob) => ob,
            None => return (trades, matched_orders), // Should not happen, but safe fallback
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

                    tracing::info!("Limit order trade: {} {} @ {} in {}", 
                        trade_quantity, 
                        market_info.base_currency.symbol, 
                        price, 
                        market_info.symbol
                    );

                    // Update matching order status
                    if matching_order.filled_quantity >= matching_order.quantity {
                        matching_order.status = OrderStatus::Filled;
                        matched_orders.push(matching_order.clone());
                    } else {
                        matching_order.status = OrderStatus::PartiallyFilled;
                        matched_orders.push(matching_order.clone());
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

        (trades, matched_orders)
    }
    
    // KEEP THIS FUNCTION
    async fn update_balances_from_trades(&mut self, trades: &[Trade], market_info: &MarketInfo) {
        for trade in trades {

            let buyer_id = trade.buyer_user_id;  // Now using User ID
            let seller_id = trade.seller_user_id; 
            let quote_id = market_info.quote_currency.id;
            let base_id  = market_info.base_currency.id;

             // FIXED: Correct calculation for trade cost
            let cost = self.safe_multiply_divide(trade.price, trade.quantity, market_info.base_currency.decimals).unwrap();
            // Buyer: spend quote (locked if resting limit or immediate if market), credit base
            let buyer_qoute_locked_now = self.balances.get(&buyer_id)
                .and_then(|ub| ub.token_balances.get(&quote_id))
                .map(|tb| tb.locked)
                .unwrap_or(0);

            let buyer_use_locked = buyer_qoute_locked_now.min(cost);

            self.update_user_balance(buyer_id, quote_id, 0, -buyer_use_locked).await;
            self.update_user_balance(buyer_id, base_id, trade.quantity, 0).await;

            // Seller: spend base (locked if resting limit), credit quote

            let seller_base_locked_now = self.balances
                .get(&seller_id)
                .and_then(|ub| ub.token_balances.get(&base_id))
                .map(|tb| tb.locked)
                .unwrap_or(0);

            let seller_use_locked = seller_base_locked_now.min(trade.quantity);

            self.update_user_balance(seller_id, base_id, 0, -seller_use_locked).await;
            self.update_user_balance(seller_id, quote_id, cost, 0).await;
            

            tracing::info!("💰 Updated balances for trade: {} {} @ {} between users {} and {}", 
                trade.quantity, 
                market_info.base_currency.symbol, 
                trade.price,
                buyer_id,
                seller_id
            );
        }
    }

    // KEEP THIS FUNCTION
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
        let balance_data = serde_json::json!({
            "user_id": user_id,
            "token_id": token_id,
            "available": token_balance.available,
            "locked": token_balance.locked
        });

        // Send to db-updater queue
        let mut conn = self.redis_manager.clone();
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("balance_updated")
            .arg("data")
            .arg(balance_data.to_string())
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
    
    // KEEP THIS FUNCTION
    async fn queue_order_created(&mut self, order: &Order) {
        let mut conn = self.redis_manager.clone();
        let order_json = serde_json::to_string(&order).unwrap();
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("order_created")
            .arg("data")
            .arg(order_json)
            .query_async(&mut conn)
            .await;
        tracing::info!("📤 Queued order_created for {}", order.id);
    }
    // KEEP THIS FUNCTION
    /// Queue updates for db-updater service
    async fn queue_db_updates(&mut self, order: &Order, matched_orders: &[Order], trades: &[Trade]) {
        let mut conn = self.redis_manager.clone();
        
        // Queue order update
        let order_json = serde_json::to_string(&order).unwrap();
        let _: Result<String, _> = redis::cmd("XADD")
            .arg("db_update_queue")
            .arg("*")
            .arg("type")
            .arg("order_updated")
            .arg("data")
            .arg(order_json)
            .query_async(&mut conn)
            .await; 

        // Queue matched orders updates
        for maker in matched_orders {
            let maker_json = serde_json::to_string(&maker).unwrap();
            let _: Result<String, _> = redis::cmd("XADD")
                .arg("db_update_queue")
                .arg("*")
                .arg("type")
                .arg("order_updated")
                .arg("data")
                .arg(maker_json)
                .query_async(&mut conn)
                .await;
        }

        // Queue trade events
        for trade in trades {
            let trade_json = serde_json::to_string(&trade).unwrap();
            let _: Result<String, _> = redis::cmd("XADD")
                .arg("db_update_queue")
                .arg("*")
                .arg("type")
                .arg("trade_executed")
                .arg("data")
                .arg(trade_json)
                .query_async(&mut conn)
                .await;
        }
        
        tracing::info!("📤 Queued {} DB updates", 1 + matched_orders.len() + trades.len());
    }
    
    // /// Publish real-time market events
    // async fn publish_market_events(&mut self, order: &Order, trades: &[Trade]) {
    //     let mut conn = self.redis_manager.clone();
        
    //     // Publish orderbook update
    //     if let Some(orderbook) = self.orderbooks.get(&order.market_id) {
    //         let snapshot = self.create_orderbook_snapshot(orderbook);
    //         let event = MarketEvent::OrderBookUpdate {
    //             market_id: order.market_id,
    //             orderbook: snapshot,
    //         };
            
    //         let channel = format!("market_events:{}", order.market_id);
    //         let message = serde_json::to_string(&event).unwrap();
            
    //         // Now this will work because AsyncCommands is imported
    //         let _: Result<(), _> = conn.publish(channel, message).await;
    //     }
        
    //     // Publish ticker update
    //     if let Some(ticker) = self.tickers.get(&order.market_id) {
    //         let event = MarketEvent::TickerUpdate(ticker.clone());
    //         let channel = format!("ticker_events:{}", order.market_id);
    //         let message = serde_json::to_string(&event).unwrap();
            
    //         let _: Result<(), _> = conn.publish(channel, message).await;
    //     }
        
    //     // Publish trade events
    //     for trade in trades {
    //         let event = MarketEvent::TradeUpdate(trade.clone());
    //         let channel = format!("trade_events:{}", order.market_id);
    //         let message = serde_json::to_string(&event).unwrap();
            
    //         let _: Result<(), _> = conn.publish(channel, message).await;
    //     }
        
    //     tracing::info!("📡 Published market events for {} trades", trades.len());
    // }
    
    // KEEP THIS FUNCTION
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
    
    // KEEP THIS FUNCTION
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
    
    fn calculate_average_price(&self, trades: &[Trade], market_id: Uuid) -> Option<i64> {
        if trades.is_empty() {
            return None;
        }
        let market = self.markets.get(&market_id)?;
        let mut total_value = 0i64;
        let mut total_quantity = 0i64;

        // Calculate total value using safe_multiply_divide for each trade
        for trade in trades {
            match self.safe_multiply_divide(trade.price, trade.quantity, market.base_currency.decimals) {
                Ok(value) => total_value += value,
                Err(e) => {
                    tracing::error!("Error calculating trade value for average price: {}", e);
                    return None; // If any calculation fails, we can't compute average
                }
            }
            total_quantity += trade.quantity;
        }
        
        if total_quantity == 0 {
            return None;
        }
        
        // Calculate average price: total_value / total_quantity
        // But we need to reverse the division we did in safe_multiply_divide
        // So we multiply by the base_multiplier again
        let decimals = market.base_currency.decimals;
        if decimals > 18 {
            tracing::error!("Decimals {} exceeds maximum supported (18)", decimals);
            return None;
        }
        
        let total_value_u256 = U256::from(total_value as u64);
        let total_quantity_u256 = U256::from(total_quantity as u64);
        let multiplier_u256 = U256::from(10u64).pow(U256::from(decimals as u64));
        
        let result = (total_value_u256 * multiplier_u256) / total_quantity_u256;
        
        if result > U256::from(i64::MAX as u64) {
            tracing::error!("Average price calculation exceeds i64::MAX");
            return None;
        }
        
        Some(result.as_u64() as i64)
    }
    
    // fn create_orderbook_snapshot(&self, orderbook: &OrderBook) -> OrderBookSnapshot {
    //     let bids: Vec<(i64, i64)> = orderbook.bids.iter()
    //         .map(|(&price, orders)| {
    //             let total_quantity = orders.iter().map(|o| o.quantity - o.filled_quantity).sum();
    //             (price, total_quantity)
    //         })
    //         .collect();
            
    //     let asks: Vec<(i64, i64)> = orderbook.asks.iter()
    //         .map(|(&price, orders)| {
    //             let total_quantity = orders.iter().map(|o| o.quantity - o.filled_quantity).sum();
    //             (price, total_quantity)
    //         })
    //         .collect();
            
    //     OrderBookSnapshot {
    //         bids,
    //         asks,
    //         timestamp: orderbook.last_updated,
    //     }
    // }

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
    
        let (available_balance, locked_amount) = {
            let user_balance = self.balances.entry(request.user_id).or_insert_with(|| UserBalance {
                user_id: request.user_id,
                token_balances: HashMap::new(),
            });
            let token_balance = user_balance.token_balances.entry(request.token_id).or_insert_with(|| TokenBalance {
                available: 0,
                locked: 0,
            });
            (token_balance.available, token_balance.locked)
        };

        let locked_amount = {
            let user_balance = self.balances.get(&request.user_id).unwrap();
            let tb = user_balance.token_balances.get(&request.token_id).unwrap();
            tb.locked
        };
        
        self.update_user_balance(
            request.user_id, 
            request.token_id, 
            request.amount, 
            0
        ).await;
        
        let new_balance = {
            let user_balance = self.balances.get(&request.user_id).unwrap();
            let tb = user_balance.token_balances.get(&request.token_id).unwrap();
            tb.available
        };
        
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
                    let available_balance = token_balance.available;
                    
                    if available_balance >= request.amount {
                        Some(())
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
        }; 
    
        match withdrawal_result {
            Some(()) => {
                // Queue database update (now safe because previous borrow is released)
                self.update_user_balance(
                    request.user_id, 
                    request.token_id, 
                    -request.amount, 
                    0
                ).await;

                let new_balance = {
                    let user_balance = self.balances.get(&request.user_id).unwrap();
                    let tb = user_balance.token_balances.get(&request.token_id).unwrap();
                    tb.available
                };
                
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

    // Add this helper function
    fn safe_multiply_divide(&self, price: i64, quantity: i64, decimals: i32) -> Result<i64, String> {
        
        if price < 0 || quantity < 0 {
            return Err("Price and quantity must be non-negative".to_string());
        }
        
        if decimals > 18 {
            return Err(format!("Decimals {} exceeds maximum supported (18)", decimals));
        }
        let price_u256 = U256::from(price as u64);
        let quantity_u256 = U256::from(quantity as u64);
        let divisor_u256 = U256::from(10u64).pow(U256::from(decimals as u64));
        
        let result = (price_u256 * quantity_u256) / divisor_u256;
        
        if result > U256::from(i64::MAX as u64) {
            return Err(format!("Result {} exceeds i64::MAX", result));
        }
        
        Ok(result.as_u64() as i64)
    }

    async fn validate_and_lock_order_balance(&mut self, order: &Order) -> Result<Reservation, String> {
        // Market
        let market = self.markets.get(&order.market_id)
            .ok_or_else(|| format!("Market not found: {}", order.market_id))?
            .clone();

        // User balance presence
        let user_balance = self.balances.get(&order.user_id)
            .ok_or_else(|| format!("No balance found for user: {}", order.user_id))?;

        match order.order_type {
            OrderType::Buy => {
                // Determine required quote amount
                let required_quote = match order.order_kind {
                    OrderKind::Limit => {
                        let price = order.price.ok_or_else(|| "Limit buy requires price".to_string())?;
                        self.safe_multiply_divide(price, order.quantity, market.base_currency.decimals)?
                    }
                    OrderKind::Market => {
                        // Estimate cost for market buy; if no liquidity, reject
                        let est = self.estimate_market_buy_price(order.market_id, order.quantity)
                            .ok_or_else(|| format!("No liquidity available for market buy in {}", market.symbol))?;
                        self.safe_multiply_divide(est, order.quantity, market.base_currency.decimals)?
                    }
                };

                // Check available quote balance
                let quote_id = market.quote_currency.id;
                let has = user_balance.token_balances.get(&quote_id).map(|b| b.available).unwrap_or(0);
                if has < required_quote {
                    return Err(format!(
                        "Insufficient {} balance for BUY in {}. Required: {}, Available: {}",
                        market.quote_currency.symbol, market.symbol, required_quote, has
                    ));
                }

                // Lock quote
                self.lock(order.user_id, quote_id, required_quote).await
                    .map_err(|e| e.to_string())?;

                Ok(Reservation { token_id: quote_id, amount: required_quote })
            }

            OrderType::Sell => {
                // Determine required base amount
                let required_base = match order.order_kind {
                    OrderKind::Limit | OrderKind::Market => order.quantity,
                };

                let base_id = market.base_currency.id;
                let has = user_balance.token_balances.get(&base_id).map(|b| b.available).unwrap_or(0);
                if has < required_base {
                    return Err(format!(
                        "Insufficient {} balance for SELL in {}. Required: {}, Available: {}",
                        market.base_currency.symbol, market.symbol, required_base, has
                    ));
                }

                // Lock base
                self.lock(order.user_id, base_id, required_base).await
                    .map_err(|e| e.to_string())?;

                Ok(Reservation { token_id: base_id, amount: required_base })
            }
        }
    }
    //KEEP THIS FUNCTION
    fn estimate_market_buy_price(&self, market_id: Uuid, quantity: i64) -> Option<i64> {
        let orderbook = self.orderbooks.get(&market_id)?;
        let market = self.markets.get(&market_id)?;
        let mut remaining_quantity = quantity;
        let mut total_cost = 0i64;
        
        // Calculate cost by walking through asks (ascending price order)
        for (&price, orders) in &orderbook.asks {
            if remaining_quantity <= 0 { break; }
            
            let available_at_price: i64 = orders.iter()
                .map(|o| o.quantity - o.filled_quantity)
                .sum();
            
            let quantity_to_buy = remaining_quantity.min(available_at_price);
            
            // FIX: Use safe_multiply_divide to avoid overflow
            match self.safe_multiply_divide(price, quantity_to_buy, market.base_currency.decimals) {
                Ok(cost) => total_cost += cost,
                Err(e) => {
                    tracing::error!("Error calculating cost in estimate_market_buy_price: {}", e);
                    return None;
                }
            }
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
            // FIX: Use U256 for the reverse calculation to avoid overflow
            let decimals = market.base_currency.decimals;
            if decimals > 18 {
                tracing::error!("Decimals {} exceeds maximum supported (18)", decimals);
                return None;
            }
            
            let total_cost_u256 = U256::from(total_cost as u64);
            let quantity_u256 = U256::from(quantity as u64);
            let multiplier_u256 = U256::from(10u64).pow(U256::from(decimals as u64));
            
            let result = (total_cost_u256 * multiplier_u256) / quantity_u256;
            
            if result > U256::from(i64::MAX as u64) {
                tracing::error!("Average price calculation exceeds i64::MAX");
                return None;
            }
            
            Some(result.as_u64() as i64)
        }
    }

    //KEEP THIS FUNCTION
    pub async fn process_cancel_order(
        &mut self,
        req: crate::redis_manager::CancelOrderRequest
    ) -> crate::redis_manager::OrderResponse {
        tracing::info!("🔄 Processing cancel order: {}", req.request_id);

        let maybe_order = self.remove_order_from_orderbook(req.market_id, req.order_id);

        match maybe_order {
            Some(mut order) => {
                // Auth check
                if order.user_id != req.user_id {
                    return crate::redis_manager::OrderResponse {
                        request_id: req.request_id,
                        success: false,
                        status: "REJECTED".to_string(),
                        order_id: Some(order.id),
                        message: "Order does not belong to user".to_string(),
                        filled_quantity: Some(order.filled_quantity),
                        remaining_quantity: Some(order.quantity - order.filled_quantity),
                        average_price: None,
                        trades: None,
                    }
                }

                // Update order status
                order.status = OrderStatus::Cancelled;
                // unlock remaining reservation for LIMIT orders
                let remaining = order.quantity - order.filled_quantity;
                match order.order_type {
                    OrderType::Buy => {
                        if matches!(order.order_kind, OrderKind::Limit) {
                            if let Some(price) = order.price {
                                // unlock remaining quote = remaining * price
                                let market = &self.markets[&order.market_id];
                                match self.safe_multiply_divide(price, remaining, market.base_currency.decimals) {
                                    Ok(quote_amount) => {
                                        let quote_id = market.quote_currency.id;
                                        self.unlock(order.user_id, quote_id, quote_amount).await;
                                    }
                                    Err(e) => {
                                        tracing::error!("Error calculating unlock amount for cancelled buy order: {}", e);
                                        // You might want to handle this error more gracefully
                                    }
                                }
                            }
                        }
                    }
                    OrderType::Sell => {
                        if matches!(order.order_kind, OrderKind::Limit) {
                            // unlock remaining base = remaining
                            let base_id = self.markets[&order.market_id].base_currency.id;
                            self.unlock(order.user_id, base_id, remaining).await;
                        }
                    }
}
                self.queue_db_updates(&order, &[], &[]).await;
                self.publish_depth(req.market_id).await;
                crate::redis_manager::OrderResponse {
                    request_id: req.request_id,
                    success: true,
                    status: "CANCELLED".to_string(),
                    order_id: Some(order.id),
                    message: "Order cancelled successfully".to_string(),
                    filled_quantity: Some(order.filled_quantity),
                    remaining_quantity: Some(order.quantity - order.filled_quantity),
                    average_price: None,
                    trades: Some(Vec::new()),
                }
            }
            None => {
                crate::redis_manager::OrderResponse {
                    request_id: req.request_id,
                    success: false,
                    status: "REJECTED".to_string(),
                    order_id: None,
                    message: "Order not found".to_string(),
                    filled_quantity: None,
                    remaining_quantity: None,
                    average_price: None,
                    trades: None,
                }
            }
        }
    }

    fn remove_order_from_orderbook(&mut self, market_id: Uuid, order_id: Uuid) -> Option<Order> {
        let orderbook = self.orderbooks.get_mut(&market_id)?;
        
        for (_price, queue) in orderbook.bids.iter_mut() {
            if let Some(pos) = queue.iter().position(|o| o.id == order_id) {
                return Some(queue.remove(pos).unwrap());
            }
        }
        
        for (_price, queue) in orderbook.asks.iter_mut() {
            if let Some(pos) = queue.iter().position(|o| o.id == order_id) {
                return Some(queue.remove(pos).unwrap());
            }
        }
        None
    }
    // Free = available - locked
    fn free_amount(&self, user: Uuid, token: Uuid) -> i64 {
        self.balances
            .get(&user)
            .and_then(|ub| ub.token_balances.get(&token))
            .map(|tb| tb.available)
            .unwrap_or(0)
    }

    // Try to lock funds: available unchanged, locked += amount
    async fn lock(&mut self, user: Uuid, token: Uuid, amount: i64) -> Result<(), &'static str> {
        if amount <= 0 { return Ok(()); }
        if self.free_amount(user, token) < amount {
            return Err("insufficient free balance to lock");
        }
        // locked += amount
        self.update_user_balance(user, token, -amount, amount).await;
        Ok(())
    }

    // Unlock funds: locked -= amount (available unchanged)
    async fn unlock(&mut self, user: Uuid, token: Uuid, amount: i64) {
        if amount <= 0 { return; }
        // locked -= amount
        self.update_user_balance(user, token, amount, -amount).await;
    }
}
