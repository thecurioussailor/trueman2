use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::trading_engine::MarketInfo;

#[derive(Debug)]
pub enum ConversionError {
    TokenNotFound,
    MarketNotFound,
    InvalidAmount,
    Overflow,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConversionError::TokenNotFound => write!(f, "Token not found in registry"),
            ConversionError::MarketNotFound => write!(f, "Market not found in registry"),
            ConversionError::InvalidAmount => write!(f, "Invalid amount provided"),
            ConversionError::Overflow => write!(f, "Amount overflow during conversion"),
        }
    }
}

/// Convert atomic units back to decimal for a specific token using decimals directly
/// Example: 1500000000 lamports (with 9 decimals) -> 1.5 SOL
pub fn from_atomic_units_with_decimals(atomic_amount: i64, decimals: i32) -> f64 {
    let divisor = 10_f64.powi(decimals);
    atomic_amount as f64 / divisor
}

/// Convert atomic price back to decimal for a market using MarketInfo
/// Example: atomic price in USDC -> 50.0 USDC per SOL
pub fn price_from_atomic_units(atomic_price: i64, market_info: &MarketInfo) -> f64 {
    // Price is in quote token units
    from_atomic_units_with_decimals(atomic_price, market_info.quote_currency.decimals)
}

/// Convert atomic quantity back to decimal for a market using MarketInfo
/// Example: atomic quantity in SOL -> 1.0 SOL
pub fn quantity_from_atomic_units(atomic_quantity: i64, market_info: &MarketInfo) -> f64 {
    // Quantity is in base token units
    from_atomic_units_with_decimals(atomic_quantity, market_info.base_currency.decimals)
}

/// Format price to tick size precision
pub fn format_price_to_tick_precision(price: f64, tick_size: i64, quote_decimals: i32) -> f64 {
    let tick_size_decimal = tick_size as f64 / 10_f64.powi(quote_decimals);
    let decimal_places = (-tick_size_decimal.log10().floor()) as usize;
    let multiplier = 10_f64.powi(decimal_places as i32);
    (price * multiplier).round() / multiplier
}

/// Format quantity to base currency precision based on min_order_size
pub fn format_quantity_to_precision(quantity: f64, min_order_size: i64, base_decimals: i32) -> f64 {
    // Calculate how many decimal places the min_order_size represents
    let min_order_decimal = (min_order_size as f64) / 10_f64.powi(base_decimals);
    
    // Get the number of decimal places needed
    let decimal_places = if min_order_decimal < 1.0 {
        (-min_order_decimal.log10().floor()) as i32
    } else {
        0
    };
    
    // Format to that precision
    let multiplier = 10_f64.powi(decimal_places);
    (quantity * multiplier).round() / multiplier
}

/// Enhanced DepthUpdate struct with both decimal and atomic values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedDepthUpdate {
    pub market_id: Uuid,
    pub seq: u64,
    pub ts: i64,
    pub bids: Vec<(f64, f64)>,        // Decimal values (price, quantity)
    pub asks: Vec<(f64, f64)>,        // Decimal values (price, quantity)
    pub bids_atomic: Vec<(i64, i64)>, // Atomic values for debugging
    pub asks_atomic: Vec<(i64, i64)>, // Atomic values for debugging
}

/// Enhanced MarketTicker struct with both decimal and atomic values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMarketTicker {
    pub market_id: Uuid,
    pub last_price: f64,              // Decimal price (e.g., 50.0 USDC)
    pub last_price_atomic: i64,       // Atomic price for debugging
    pub volume_24h: f64,              // Decimal volume (e.g., 10.5 SOL)
    pub volume_24h_atomic: i64,       // Atomic volume for debugging
    pub high_24h: f64,                // Decimal high
    pub high_24h_atomic: i64,         // Atomic high for debugging
    pub low_24h: f64,                 // Decimal low
    pub low_24h_atomic: i64,          // Atomic low for debugging
    pub change_24h: f64,
    pub timestamp: i64,
}

/// Convert atomic MarketTicker to enhanced version with decimal values
pub fn convert_ticker_to_decimal(atomic_ticker: &crate::trading_engine::MarketTicker, market_info: &MarketInfo) -> EnhancedMarketTicker {
    let raw_price = price_from_atomic_units(atomic_ticker.last_price, market_info);
    let raw_volume = quantity_from_atomic_units(atomic_ticker.volume_24h, market_info);
    let raw_high = price_from_atomic_units(atomic_ticker.high_24h, market_info);
    let raw_low = price_from_atomic_units(atomic_ticker.low_24h, market_info);

    EnhancedMarketTicker {
        market_id: atomic_ticker.market_id,
        last_price: format_price_to_tick_precision(raw_price, market_info.tick_size, market_info.quote_currency.decimals),
        last_price_atomic: atomic_ticker.last_price,
        volume_24h: format_quantity_to_precision(raw_volume, market_info.min_order_size, market_info.base_currency.decimals),
        volume_24h_atomic: atomic_ticker.volume_24h,
        high_24h: format_price_to_tick_precision(raw_high, market_info.tick_size, market_info.quote_currency.decimals),
        high_24h_atomic: atomic_ticker.high_24h,
        low_24h: format_price_to_tick_precision(raw_low, market_info.tick_size, market_info.quote_currency.decimals),
        low_24h_atomic: atomic_ticker.low_24h,
        change_24h: atomic_ticker.change_24h,
        timestamp: atomic_ticker.timestamp,
    }
}

/// Enhanced Trade struct with both decimal and atomic values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedTrade {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub price: f64,              // Decimal price (e.g., 4021.0 USDC)
    pub price_atomic: i64,       // Atomic price for debugging
    pub quantity: f64,           // Decimal quantity (e.g., 0.001 ETH)
    pub quantity_atomic: i64,    // Atomic quantity for debugging
    pub timestamp: i64,
}

/// Convert atomic Trade to enhanced version with decimal values
pub fn convert_trade_to_decimal(atomic_trade: &crate::trading_engine::Trade, market_info: &MarketInfo) -> EnhancedTrade {
    let raw_price = price_from_atomic_units(atomic_trade.price, market_info);
    let raw_quantity = quantity_from_atomic_units(atomic_trade.quantity, market_info);
    EnhancedTrade {
        id: atomic_trade.id,
        market_id: atomic_trade.market_id,
        buyer_order_id: atomic_trade.buyer_order_id,
        seller_order_id: atomic_trade.seller_order_id,
        buyer_user_id: atomic_trade.buyer_user_id,
        seller_user_id: atomic_trade.seller_user_id,
        price: format_price_to_tick_precision(raw_price, market_info.tick_size, market_info.quote_currency.decimals),
        price_atomic: atomic_trade.price,
        quantity: format_quantity_to_precision(raw_quantity, market_info.min_order_size, market_info.base_currency.decimals),
        quantity_atomic: atomic_trade.quantity,
        timestamp: atomic_trade.timestamp,
    }
}