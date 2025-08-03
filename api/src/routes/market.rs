use actix_web::{web::Json, HttpRequest, post, delete, get, put, Responder, HttpResponse, HttpMessage};
use diesel::prelude::*;
use database::{
    establish_connection,
    Token,
    NewMarket,
    Market,
    schema::{markets, tokens},
};
use crate::jwt::Claims;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Add this new request struct after the existing ones
#[derive(Deserialize, Serialize, Debug)]
pub struct CreateMarketRequest {
    symbol: String,
    base_currency_symbol: String,
    quote_currency_symbol: String,
    min_order_size: i64,
    tick_size: i64,
}

#[derive(Deserialize)]
pub struct UpdateMarketRequest {
    pub symbol: Option<String>,
    pub base_currency_symbol: Option<String>,
    pub quote_currency_symbol: Option<String>,
    pub min_order_size: Option<i64>,
    pub tick_size: Option<i64>,
    pub is_active: Option<bool>,
}

// Add this response struct for markets with token details
#[derive(Serialize)]
pub struct MarketWithTokens {
    pub id: Uuid,
    pub symbol: String,
    pub base_currency: Token,
    pub quote_currency: Token,
    pub min_order_size: i64,
    pub tick_size: i64,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
struct ErrorResponse {
    status: bool,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct SuccessResponse {
    status: bool,
    message: String,
    data: Option<serde_json::Value>,
}

impl ErrorResponse {
    fn new(message: &str) -> Self {
        Self {
            status: false,
            message: message.to_string(),
            data: None,
        }
    }
}

impl SuccessResponse {
    fn new_single(status: bool, message: String, data: Option<MarketWithTokens>) -> Self {
        Self {
            status,
            message,
            data: data.map(|market| serde_json::to_value(market).unwrap()),
        }
    }

    fn new_multiple(status: bool, message: String, data: Vec<MarketWithTokens>) -> Self {
        Self {
            status,
            message,
            data: Some(serde_json::to_value(data).unwrap()),
        }
    }
}

#[post("/markets")]
pub async fn create_market(body: Json<CreateMarketRequest>, req: HttpRequest) -> impl Responder {
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let mut connection = establish_connection();

    // Check if market symbol already exists
    if let Ok(_) = markets::table
        .filter(markets::symbol.eq(&body.symbol))
        .first::<Market>(&mut connection) {
        let response = ErrorResponse::new("Market already exists");
        return HttpResponse::BadRequest().json(response);
    }

    // Get base currency token
    let base_token = match tokens::table
        .filter(tokens::symbol.eq(&body.base_currency_symbol))
        .filter(tokens::is_active.eq(true))
        .first::<Token>(&mut connection) {
        Ok(token) => token,
        Err(_) => {
            let response = ErrorResponse::new("Base currency token not found or inactive");
            return HttpResponse::BadRequest().json(response);
        }
    };

    // Get quote currency token
    let quote_token = match tokens::table
        .filter(tokens::symbol.eq(&body.quote_currency_symbol))
        .filter(tokens::is_active.eq(true))
        .first::<Token>(&mut connection) {
        Ok(token) => token,
        Err(_) => {
            let response = ErrorResponse::new("Quote currency token not found or inactive");
            return HttpResponse::BadRequest().json(response);
        }
    };

    let new_market = NewMarket {
        symbol: body.symbol.clone(),
        base_currency_id: base_token.id,
        quote_currency_id: quote_token.id,
        min_order_size: body.min_order_size,
        tick_size: body.tick_size,
        is_active: Some(true),
    };

    match diesel::insert_into(markets::table)
        .values(&new_market)
        .get_result::<Market>(&mut connection) {
        Ok(market) => {
            let market_response = MarketWithTokens {
                id: market.id,
                symbol: market.symbol,
                base_currency: base_token,
                quote_currency: quote_token,
                min_order_size: market.min_order_size,
                tick_size: market.tick_size,
                is_active: market.is_active,
                created_at: market.created_at,
            };
            let response = SuccessResponse::new_single(
                true,
                "Market created successfully".to_string(),
                Some(market_response)
            );
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            println!("Error creating market: {:?}", e);
            let response = ErrorResponse::new("Error creating market");
            HttpResponse::InternalServerError().json(response)
        }
    }
}