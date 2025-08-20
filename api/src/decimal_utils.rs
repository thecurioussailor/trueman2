use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::registry;
use crate::redis_manager::UserTokenBalance;
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



#[derive(Deserialize, Debug)]
pub struct DecimalCreateOrderRequest {
    pub market_id: Uuid,
    pub order_type: String, // "Buy" or "Sell"
    pub order_kind: String, // "Market" or "Limit"
    pub price: Option<f64>,    // Decimal price (e.g., 150.25 USDC per SOL)
    pub quantity: f64,         // Decimal quantity (e.g., 1.5 SOL)
}

// Response structures that return decimal amounts
#[derive(Serialize, Debug)]
pub struct DecimalUserTokenBalance {
    pub token_id: Uuid,
    pub token_symbol: String, // e.g., "USDC", "SOL"
    pub available: f64,       // Decimal amount (e.g., 1.5)
    pub locked: f64,          // Decimal amount (e.g., 0.25)
}

#[derive(Serialize, Debug)]
pub struct DecimalBalanceResponse {
    pub balances: Vec<DecimalUserTokenBalance>,
}

/// Convert a decimal amount to atomic units for a specific token
/// Example: 1.5 SOL (with 9 decimals) -> 1500000000 lamports
pub fn to_atomic_units(amount: f64, token_id: Uuid) -> Result<i64, ConversionError> {
    if amount < 0.0 || !amount.is_finite() {
        return Err(ConversionError::InvalidAmount);
    }

    let decimals = registry::get_token_decimals(token_id)
        .ok_or(ConversionError::TokenNotFound)?;
    println!("decimals: {:?}", decimals);
    let multiplier = 10_f64.powi(decimals as i32);
    let atomic_amount = amount * multiplier;
    
    // Check for overflow
    if atomic_amount > i64::MAX as f64 {
        return Err(ConversionError::Overflow);
    }
    
    Ok(atomic_amount.round() as i64)
}

/// Convert atomic units back to decimal for a specific token
/// Example: 1500000000 lamports (with 9 decimals) -> 1.5 SOL
pub fn from_atomic_units(atomic_amount: i64, token_id: Uuid) -> Result<f64, ConversionError> {
    let decimals = registry::get_token_decimals(token_id)
        .ok_or(ConversionError::TokenNotFound)?;
    
    let divisor = 10_f64.powi(decimals as i32);
    Ok(atomic_amount as f64 / divisor)
}

/// Convert price from decimal to atomic units for a market
/// Price is always in quote token units per base token
/// Example: For SOL-USDC market, price 150.25 USDC -> atomic units in quote token (USDC)
pub fn price_to_atomic_units(price: f64, market_id: Uuid) -> Result<i64, ConversionError> {
    if price <= 0.0 || !price.is_finite() {
        return Err(ConversionError::InvalidAmount);
    }

    let market_meta = registry::get_market_meta(market_id)
        .ok_or(ConversionError::MarketNotFound)?;
    println!("market_meta: {:?}", market_meta);
    // Convert using the quote token's decimals
    to_atomic_units(price, market_meta.quote_token_id)
}

/// Convert quantity from decimal to atomic units for a market (base token)
/// Example: For SOL-USDC market, quantity 1.5 SOL -> atomic units in base token (SOL)
pub fn quantity_to_atomic_units(quantity: f64, market_id: Uuid) -> Result<i64, ConversionError> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err(ConversionError::InvalidAmount);
    }

    let market_meta = registry::get_market_meta(market_id)
        .ok_or(ConversionError::MarketNotFound)?;
    
    // Convert using the base token's decimals
    to_atomic_units(quantity, market_meta.base_token_id)
}

/// Convert atomic price back to decimal for a market
/// Example: atomic price in USDC -> 150.25 USDC per SOL
pub fn price_from_atomic_units(atomic_price: i64, market_id: Uuid) -> Result<f64, ConversionError> {
    let market_meta = registry::get_market_meta(market_id)
        .ok_or(ConversionError::MarketNotFound)?;
    
    // Convert using the quote token's decimals
    from_atomic_units(atomic_price, market_meta.quote_token_id)
}

/// Convert atomic quantity back to decimal for a market
/// Example: atomic quantity in SOL -> 1.5 SOL
pub fn quantity_from_atomic_units(atomic_quantity: i64, market_id: Uuid) -> Result<f64, ConversionError> {
    let market_meta = registry::get_market_meta(market_id)
        .ok_or(ConversionError::MarketNotFound)?;
    
    // Convert using the base token's decimals
    from_atomic_units(atomic_quantity, market_meta.base_token_id)
}

/// Convert atomic balance data to decimal format with token symbols
pub fn convert_balances_to_decimal(atomic_balances: Vec<UserTokenBalance>) -> Result<DecimalBalanceResponse, ConversionError> {
    let mut decimal_balances = Vec::new();
    
    for balance in atomic_balances {
        let available_decimal = from_atomic_units(balance.available, balance.token_id)?;
        let locked_decimal = from_atomic_units(balance.locked, balance.token_id)?;
        
        // Get token symbol from registry (you might need to add this to your registry)
        // For now, we'll use the token_id as a fallback
        let token_symbol = get_token_symbol(balance.token_id).unwrap_or_else(|| balance.token_id.to_string());
        
        decimal_balances.push(DecimalUserTokenBalance {
            token_id: balance.token_id,
            token_symbol,
            available: available_decimal,
            locked: locked_decimal,
        });
    }
    
    Ok(DecimalBalanceResponse {
        balances: decimal_balances,
    })
}

/// Get token symbol from registry - you may need to extend your registry for this
/// For now, this is a placeholder that returns common token symbols
fn get_token_symbol(token_id: Uuid) -> Option<String> {
    // This is a placeholder - you should extend your registry to include token symbols
    // For now, return None so it falls back to token_id
    None
    
    // In a real implementation, you might do:
    // registry::get_token_symbol(token_id)
}