use actix_web::{post, web::Json, HttpRequest, HttpResponse, HttpMessage, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::jwt::Claims;
use crate::redis_manager::{
    get_redis_manager, EngineMessage, EngineProcessingResult, EngineResponse,
    OrderRequest
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
    println!("Creating order: {:?}", body);

    // Create order request
    let order_request = OrderRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        market_id: body.market_id,
        order_type: body.order_type,
        order_kind: body.order_kind,
        price: body.price,
        quantity: body.quantity,
        timestamp: Utc::now().timestamp_millis(),
    };
    
    // Send to engine and wait for response
    let redis_manager = get_redis_manager().await;
    
    match redis_manager.send_and_wait(EngineMessage::Order(order_request), 5).await {
        EngineProcessingResult::Success(EngineResponse::Order(response)) => {
            HttpResponse::Ok().json(response)
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