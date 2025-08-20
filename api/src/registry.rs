use dashmap::DashMap;
use diesel::prelude::*;
use once_cell::sync::Lazy;
use uuid::Uuid;
use database::{
    Market,
    schema::{markets, tokens},
};

#[derive(Clone, Debug)]
pub struct MarketMeta {
    pub market_id: Uuid,
    pub symbol: String,

    pub base_token_id: Uuid,
    pub quote_token_id: Uuid,

    pub base_decimals: u32,
    pub quote_decimals: u32,

    // Venue filters in atomic units
    pub tick_size: i64,       // quote atomic increment
    pub min_order_size: i64,  // base atomic minimum
    // Optional: add step_size, min_notional, etc., when you add them to DB
}

pub static TOKENS_DECIMALS: Lazy<DashMap<Uuid, u32>> = Lazy::new(DashMap::new);
pub static MARKETS: Lazy<DashMap<Uuid, MarketMeta>> = Lazy::new(DashMap::new);

/// Load tokens (decimals) and markets (with base/quote decimals) into in-memory caches.
/// Call once at startup, and periodically (e.g., every 30–60s) to refresh.
pub fn load_registry(conn: &mut PgConnection) -> Result<(), diesel::result::Error> {
    // 1) Load active tokens → decimals
    let token_rows: Vec<(Uuid, i32)> = tokens::table
        .filter(tokens::is_active.eq(true))
        .select((tokens::id, tokens::decimals))
        .load(conn)?;

    // Rebuild token cache atomically (simple strategy: clear then insert)
    TOKENS_DECIMALS.clear();
    for (id, dec_i32) in token_rows {
        let d = u32::try_from(dec_i32).unwrap_or(0);
        TOKENS_DECIMALS.insert(id, d);
    }

    // 2) Load active markets; attach decimals from TOKENS_DECIMALS
    let market_rows: Vec<Market> = markets::table
        .filter(markets::is_active.eq(true))
        .select(Market::as_select())
        .load(conn)?;

    MARKETS.clear();
    for m in market_rows {
        // Market is expected to have base_currency_id, quote_currency_id, tick_size, min_order_size
        let base_id = m.base_currency_id;
        let quote_id = m.quote_currency_id;

        // Lookup decimals from token cache; skip markets with unknown tokens
        let base_dec = match TOKENS_DECIMALS.get(&base_id) {
            Some(v) => *v,
            None => continue,
        };
        let quote_dec = match TOKENS_DECIMALS.get(&quote_id) {
            Some(v) => *v,
            None => continue,
        };

        let meta = MarketMeta {
            market_id: m.id,
            symbol: m.symbol.clone(),

            base_token_id: base_id,
            quote_token_id: quote_id,

            base_decimals: base_dec,
            quote_decimals: quote_dec,

            tick_size: m.tick_size,
            min_order_size: m.min_order_size,
        };
        MARKETS.insert(m.id, meta);
    }

    Ok(())
}

/// Get cached token decimals by token_id (None if unknown).
pub fn get_token_decimals(token_id: Uuid) -> Option<u32> {
    TOKENS_DECIMALS.get(&token_id).map(|v| *v)
}

/// Get cached market metadata by market_id (None if unknown).
pub fn get_market_meta(market_id: Uuid) -> Option<MarketMeta> {
    MARKETS.get(&market_id).map(|v| v.clone())
}

/// Convenience: counts (useful for logs/health).
pub fn counts() -> (usize, usize) {
    (TOKENS_DECIMALS.len(), MARKETS.len())
}