use actix_web::{post, web::Json, HttpRequest, HttpResponse, HttpMessage, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::jwt::Claims;
use crate::redis_manager::{OrderRequest, OrderProcessingResult, get_redis_manager};
use database::{OrderType, OrderKind};

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

#[post("/orders")]
pub async fn create_order(req: HttpRequest, body: Json<CreateOrderRequest>) -> impl Responder {
    // Extract user ID from JWT
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let body = body.into_inner();
    println!("body: {:?}", body);

    // Validate order data
    if body.quantity <= 0 {
        return HttpResponse::BadRequest().json("Order quantity must be positive");
    }

    let order_type = match body.order_type.as_str() {
        "Buy" => OrderType::Buy,
        "Sell" => OrderType::Sell,
        _ => return HttpResponse::BadRequest().json("Invalid order type"),
    };

    let order_kind = match body.order_kind.as_str() {
        "Market" => OrderKind::Market,
        "Limit" => OrderKind::Limit,
        _ => return HttpResponse::BadRequest().json("Invalid order kind"),
    };

    // Validate limit orders have price
    if matches!(order_kind, OrderKind::Limit) && body.price.is_none() {
        return HttpResponse::BadRequest().json("Limit orders must have a price");
    }

    // Create order request
    let request_id = Uuid::new_v4().to_string();
    let order_request = OrderRequest {
        request_id: request_id.clone(),
        user_id,
        market_id: body.market_id,
        order_type: order_type.to_string(),
        order_kind: order_kind.to_string(),
        price: body.price,
        quantity: body.quantity,
        timestamp: Utc::now().timestamp_millis(),
    };

    // Send to engine and wait for response
    let redis_manager = get_redis_manager().await;
    match redis_manager.send_and_wait(order_request, 5).await {
        OrderProcessingResult::Success(response) => {
            let result = OrderResult {
                success: response.success,
                message: response.message,
                request_id: response.request_id,
                status: response.status,
                order_id: response.order_id,
                filled_quantity: response.filled_quantity,
                trades: response.trades.map(|trades| 
                    trades.into_iter()
                        .map(|t| serde_json::to_value(t).unwrap())
                        .collect()
                ),
            };
            
            if response.success {
                HttpResponse::Ok().json(result)
            } else {
                HttpResponse::BadRequest().json(result)
            }
        }
        OrderProcessingResult::Timeout => {
            let result = OrderResult {
                success: true,
                message: "Order submitted successfully and is being processed".to_string(),
                request_id,
                status: "PROCESSING".to_string(),
                order_id: None,
                filled_quantity: None,
                trades: None,
            };
            HttpResponse::Ok().json(result)
        }
        OrderProcessingResult::Error(error) => {
            let result = OrderResult {
                success: false,
                message: format!("Order failed: {}", error),
                request_id,
                status: "REJECTED".to_string(),
                order_id: None,
                filled_quantity: None,
                trades: None,
            };
            HttpResponse::BadRequest().json(result)
        }
    }
}