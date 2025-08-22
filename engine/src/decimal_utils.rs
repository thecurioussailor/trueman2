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
    EnhancedMarketTicker {
        market_id: atomic_ticker.market_id,
        last_price: price_from_atomic_units(atomic_ticker.last_price, market_info),
        last_price_atomic: atomic_ticker.last_price,
        volume_24h: quantity_from_atomic_units(atomic_ticker.volume_24h, market_info),
        volume_24h_atomic: atomic_ticker.volume_24h,
        high_24h: price_from_atomic_units(atomic_ticker.high_24h, market_info),
        high_24h_atomic: atomic_ticker.high_24h,
        low_24h: price_from_atomic_units(atomic_ticker.low_24h, market_info),
        low_24h_atomic: atomic_ticker.low_24h,
        change_24h: atomic_ticker.change_24h,
        timestamp: atomic_ticker.timestamp,
    }
}