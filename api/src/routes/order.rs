use actix_web::{post,get, web::Json, HttpRequest, HttpResponse, HttpMessage, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::jwt::Claims;
use crate::redis_manager::{
    get_redis_manager, EngineMessage, EngineProcessingResult, EngineResponse,
    OrderRequest, CancelOrderRequest,
};
use crate::decimal_utils::{
    DecimalCreateOrderRequest, price_to_atomic_units, quantity_to_atomic_units, 
    ConversionError, quantity_from_atomic_units, price_from_atomic_units
};
use diesel::prelude::*;
use database::{
    establish_connection,
    Order as DbOrder,
    Market,
    schema::{orders, markets},
};

#[derive(Deserialize, Debug)]
pub struct CreateOrderRequest {
    pub market_id: Uuid,
    pub order_type: String, // "Buy" or "Sell"
    pub order_kind: String, // "Market" or "Limit"
    pub price: Option<i64>,
    pub quantity: i64,
}

#[derive(Serialize)]
pub struct OrderResult {
    pub success: bool,
    pub message: String,
    pub request_id: String,
    pub status: String,
    pub order_id: Option<Uuid>,
    pub filled_quantity: Option<i64>,
    pub trades: Option<Vec<serde_json::Value>>,
}

// Enhanced order result with decimal conversion
#[derive(Serialize)]
pub struct DecimalOrderResult {
    pub success: bool,
    pub message: String,
    pub request_id: String,
    pub status: String,
    pub order_id: Option<Uuid>,
    pub filled_quantity: Option<f64>,        // Decimal quantity
    pub filled_quantity_atomic: Option<i64>, // Atomic quantity for debugging
    pub trades: Option<Vec<serde_json::Value>>,
}


#[derive(Deserialize, Debug)]
pub struct CancelOrderBody {
    pub order_id: Uuid,
    pub market_id: Uuid,
}

#[post("/orders")]
pub async fn create_order(req: HttpRequest, body: Json<DecimalCreateOrderRequest>) -> impl Responder {
    // Extract user ID from JWT
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let body = body.into_inner();
    println!("Creating order: {:?}", body);

    if body.quantity <= 0.0 {
        return HttpResponse::BadRequest().json("Invalid quantity: Quantity must be greater than 0");
    }

    if body.order_kind == "Limit" {
        match body.price {
            Some(price) if price <= 0.0 => {
                return HttpResponse::BadRequest().json("Invalid price: must be positive for limit orders");
            }
            None => {
                return HttpResponse::BadRequest().json("Price required for limit orders");
            }
            _ => {} // Valid price
        }
    }

    // Convert decimal price to atomic units (quote token) if provided
    let atomic_price = if let Some(price) = body.price {
        match price_to_atomic_units(price, body.market_id) {
            Ok(p) => Some(p),
            Err(ConversionError::MarketNotFound) => {
                return HttpResponse::BadRequest().json("Market not found");
            },
            Err(ConversionError::InvalidAmount) => {
                return HttpResponse::BadRequest().json("Invalid price");
            },
            Err(ConversionError::Overflow) => {
                return HttpResponse::BadRequest().json("Price too large");
            },
            Err(e) => {
                return HttpResponse::InternalServerError().json(format!("Price conversion error: {}", e));
            }
        }
    } else {
        None
    };
    println!("atomic_price: {:?}", atomic_price);
    // Convert decimal quantity to atomic units (base token)
    let atomic_quantity = match quantity_to_atomic_units(body.quantity, body.market_id) {
        Ok(qty) => qty,
        Err(ConversionError::MarketNotFound) => {
            return HttpResponse::BadRequest().json("Market not found");
        },
        Err(ConversionError::InvalidAmount) => {
            return HttpResponse::BadRequest().json("Invalid quantity");
        },
        Err(ConversionError::Overflow) => {
            return HttpResponse::BadRequest().json("Quantity too large");
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(format!("Quantity conversion error: {}", e));
        }
    };
    println!("atomic_quantity: {:?}", atomic_quantity);
    // Create order request
    let order_request = OrderRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        market_id: body.market_id,
        order_type: body.order_type,
        order_kind: body.order_kind,
        price: atomic_price,
        quantity: atomic_quantity,
        timestamp: Utc::now().timestamp_millis(),
    };
    
    // Send to engine and wait for response
    let redis_manager = get_redis_manager().await;
    
    match redis_manager.send_and_wait(EngineMessage::Order(order_request), 5).await {
        EngineProcessingResult::Success(EngineResponse::Order(response)) => {
            // Convert filled_quantity back to decimal for response
            let filled_quantity_decimal = if let Some(filled_atomic) = response.filled_quantity {
                match quantity_from_atomic_units(filled_atomic, body.market_id) {
                    Ok(decimal_quantity) => {
                        // Calculate decimal filled quantity
                        Some(decimal_quantity)
                    }
                    Err(e) => {
                        println!("Error converting filled quantity to decimal: {}", e);
                        None
                    }
                }
            } else {
                None
            };

                // Convert remaining_quantity back to decimal
            let remaining_quantity_decimal = if let Some(remaining_atomic) = response.remaining_quantity {
                match quantity_from_atomic_units(remaining_atomic, body.market_id) {
                    Ok(decimal_qty) => Some(decimal_qty),
                    Err(e) => {
                        println!("Error converting remaining quantity: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Convert average_price back to decimal
            let average_price_decimal = if let Some(avg_price_atomic) = response.average_price {
                match price_from_atomic_units(avg_price_atomic, body.market_id) {
                    Ok(decimal_price) => Some(decimal_price),
                    Err(e) => {
                        println!("Error converting average price: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Convert trade data back to decimal
            let trades_decimal = response.trades.map(|trades| {
                trades.into_iter().map(|trade| {
                    // Convert trade price and quantity to decimal
                    let trade_price_decimal = price_from_atomic_units(trade.price, body.market_id)
                        .unwrap_or(trade.price as f64); // Fallback to atomic if conversion fails
                    let trade_quantity_decimal = quantity_from_atomic_units(trade.quantity, body.market_id)
                        .unwrap_or(trade.quantity as f64); // Fallback to atomic if conversion fails
            
                    serde_json::json!({
                        "trade_id": trade.trade_id,
                        "price": trade_price_decimal,           // Decimal price (e.g., 50.0 USDC)
                        "price_atomic": trade.price,            // Atomic price for debugging
                        "quantity": trade_quantity_decimal,     // Decimal quantity (e.g., 1.0 SOL)  
                        "quantity_atomic": trade.quantity,      // Atomic quantity for debugging
                        "timestamp": trade.timestamp
                    })
                }).collect::<Vec<serde_json::Value>>()  // <-- Add explicit type annotation here
            });

            HttpResponse::Ok().json(serde_json::json!({
                "success": response.success,
                "message": response.message,
                "request_id": response.request_id,
                "status": response.status,
                "order_id": response.order_id,
                
                // Quantities in decimal format
                "filled_quantity": filled_quantity_decimal,
                "remaining_quantity": remaining_quantity_decimal,
                "average_price": average_price_decimal,
                
                // Atomic values for debugging
                "filled_quantity_atomic": response.filled_quantity,
                "remaining_quantity_atomic": response.remaining_quantity,
                "average_price_atomic": response.average_price,
                
                // Trades with both decimal and atomic values
                "trades": trades_decimal
            }))
        }
        EngineProcessingResult::Timeout => {
            HttpResponse::Ok().json("Order is being processed")
        }
        EngineProcessingResult::Error(e) => {
            HttpResponse::BadRequest().json(format!("Order failed: {}", e))
        }
        _ => {
            HttpResponse::InternalServerError().json("Unexpected response type")
        }
    }
}

#[post("/orders/cancel")]
pub async fn cancel_order(req: HttpRequest, body: Json<CancelOrderBody>) -> impl Responder {
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let body = body.into_inner();

    let cancel_req = CancelOrderRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        order_id: body.order_id,
        market_id: body.market_id,
        timestamp: Utc::now().timestamp_millis(),
    };
    
    let redis_manager = get_redis_manager().await;
    
    match redis_manager.send_and_wait(EngineMessage::CancelOrder(cancel_req), 5).await {
        EngineProcessingResult::Success(EngineResponse::Order(response)) => {
            HttpResponse::Ok().json(response)
        }
        EngineProcessingResult::Timeout => {
            HttpResponse::Ok().json("Cancel order is being processed")
        }
        EngineProcessingResult::Error(e) => {
            HttpResponse::BadRequest().json(format!("Cancel order failed: {}", e))
        }
        _ => {
            HttpResponse::InternalServerError().json("Unexpected response type")
        }
    }
}

#[get("/orders")]
pub async fn get_orders(req: HttpRequest) -> impl Responder {
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let mut conn = establish_connection();

    #[derive(Serialize)]
    struct OrderWithMarket {
        id: Uuid,
        user_id: Uuid,
        market: Market,
        order_type: String,
        order_kind: String,
        price: Option<f64>,        // Decimal price
        price_atomic: Option<i64>, // Atomic price for debugging
        quantity: f64,             // Decimal quantity  
        quantity_atomic: i64,      // Atomic quantity for debugging
        filled_quantity: f64,      // Decimal filled quantity
        filled_quantity_atomic: i64, // Atomic filled quantity for debugging
        status: String,
        created_at: chrono::NaiveDateTime,
        updated_at: chrono::NaiveDateTime,
    }

    let result = orders::table
        .inner_join(markets::table.on(markets::id.eq(orders::market_id)))
        .filter(orders::user_id.eq(user_id))
        .select((DbOrder::as_select(), Market::as_select()))
        .order(orders::created_at.desc())
        .load::<(DbOrder, Market)>(&mut conn);

    match result {
        Ok(rows) => {
            let data: Vec<OrderWithMarket> = rows
                .into_iter()
                .map(|(o, m)| {
                    // Convert price from atomic to decimal (if exists)
                    let price_decimal = if let Some(atomic_price) = o.price {
                        price_from_atomic_units(atomic_price, m.id).ok()
                    } else {
                        None
                    };

                    // Convert quantity from atomic to decimal
                    let quantity_decimal = quantity_from_atomic_units(o.quantity, m.id)
                        .unwrap_or(o.quantity as f64); // Fallback to atomic if conversion fails

                    // Convert filled_quantity from atomic to decimal
                    let filled_quantity_decimal = quantity_from_atomic_units(o.filled_quantity, m.id)
                        .unwrap_or(o.filled_quantity as f64); // Fallback to atomic if conversion fails
                    OrderWithMarket {
                        id: o.id,
                        user_id: o.user_id,
                        market: m,
                        order_type: o.order_type,
                        order_kind: o.order_kind,
                        // Decimal values (user-friendly)
                        price: price_decimal,
                        quantity: quantity_decimal,
                        filled_quantity: filled_quantity_decimal,
                        
                        // Atomic values (for debugging)
                        price_atomic: o.price,
                        quantity_atomic: o.quantity,
                        filled_quantity_atomic: o.filled_quantity,
                        status: o.status,
                        created_at: o.created_at,
                        updated_at: o.updated_at,
                    }
                })
                .collect();
            HttpResponse::Ok().json(data)
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Error fetching orders: {}", e)),
    }
}