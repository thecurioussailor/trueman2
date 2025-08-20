use actix_web::{get, HttpRequest,web,  HttpResponse, HttpMessage, Responder};
use diesel::prelude::*;
use database::{
    establish_connection, schema::trades::{self, dsl as trades_dsl}, Trade as DbTrade
};
use uuid::Uuid;
use crate::jwt::Claims;
use serde::{Serialize, Deserialize};
use crate::decimal_utils::{price_from_atomic_units, quantity_from_atomic_units};

#[derive(Deserialize)]
pub struct TradesQuery {
    pub market_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Enhanced trade response with decimal conversion
#[derive(Serialize)]
pub struct DecimalTrade {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub price: f64,              // Decimal price (e.g., 50.0 USDC)
    pub price_atomic: i64,       // Atomic price for debugging
    pub quantity: f64,           // Decimal quantity (e.g., 1.0 SOL)
    pub quantity_atomic: i64,    // Atomic quantity for debugging
    pub created_at: chrono::NaiveDateTime,
}

#[get("/trades")]
pub async fn get_trades(req: HttpRequest, query: web::Query<TradesQuery>) -> impl Responder {
    let uid = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let mut conn = establish_connection();
    use trades_dsl::{trades, buyer_user_id, seller_user_id, market_id, created_at};

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0);

    let mut q = trades
        .filter(buyer_user_id.eq(uid).or(seller_user_id.eq(uid)))
        .into_boxed();

    if let Some(mid) = query.market_id {
        q = q.filter(market_id.eq(mid));
    }

    match q
        .select(DbTrade::as_select())
        .order(created_at.desc())
        .limit(limit)
        .offset(offset)
        .load::<DbTrade>(&mut conn) {
        Ok(rows) => {
            let decimal_trades: Vec<DecimalTrade> = rows
                .into_iter()
                .map(|trade| {
                    // Convert price from atomic to decimal
                    let price_decimal = price_from_atomic_units(trade.price, trade.market_id)
                        .unwrap_or(trade.price as f64); // Fallback to atomic if conversion fails

                    // Convert quantity from atomic to decimal
                    let quantity_decimal = quantity_from_atomic_units(trade.quantity, trade.market_id)
                        .unwrap_or(trade.quantity as f64); // Fallback to atomic if conversion fails

                    DecimalTrade {
                        id: trade.id,
                        market_id: trade.market_id,
                        buyer_order_id: trade.buyer_order_id,
                        seller_order_id: trade.seller_order_id,
                        buyer_user_id: trade.buyer_user_id,
                        seller_user_id: trade.seller_user_id,
                        
                        // Decimal values (user-friendly)
                        price: price_decimal,
                        quantity: quantity_decimal,
                        
                        // Atomic values (for debugging)
                        price_atomic: trade.price,
                        quantity_atomic: trade.quantity,
                        
                        created_at: trade.created_at,
                    }
                })
                .collect(); 
            HttpResponse::Ok().json(decimal_trades)
        },
        Err(e) => HttpResponse::InternalServerError().json(format!("DB error: {}", e)),
    }
}