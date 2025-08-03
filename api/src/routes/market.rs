use actix_web::{web::Json, HttpRequest, post, delete, get, put, Responder, HttpResponse, HttpMessage, web::Path};
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
    base_currency_id: Uuid,
    quote_currency_id: Uuid,
    min_order_size: i64,
    tick_size: i64,
}

#[derive(Deserialize)]
pub struct UpdateMarketRequest {
    pub symbol: Option<String>,
    pub base_currency_id: Option<Uuid>,
    pub quote_currency_id: Option<Uuid>,
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
        .filter(tokens::id.eq(&body.base_currency_id))
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
        .filter(tokens::id.eq(&body.quote_currency_id))
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

#[get("/markets")]
pub async fn get_markets(req: HttpRequest) -> impl Responder {
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let mut connection = establish_connection();

    let markets_result = markets::table
        .select(Market::as_select())
        .load::<Market>(&mut connection);

        match markets_result {
            Ok(markets) => {
                let mut markets_with_tokens = Vec::new();
                
                for market in markets {
                    // Get base token
                    let base_token = match tokens::table
                        .filter(tokens::id.eq(market.base_currency_id))
                        .first::<Token>(&mut connection) {
                        Ok(token) => token,
                        Err(_) => continue, // Skip if token not found
                    };
    
                    // Get quote token
                    let quote_token = match tokens::table
                        .filter(tokens::id.eq(market.quote_currency_id))
                        .first::<Token>(&mut connection) {
                        Ok(token) => token,
                        Err(_) => continue, // Skip if token not found
                    };
    
                    markets_with_tokens.push(MarketWithTokens {
                        id: market.id,
                        symbol: market.symbol,
                        base_currency: base_token,
                        quote_currency: quote_token,
                        min_order_size: market.min_order_size,
                        tick_size: market.tick_size,
                        is_active: market.is_active,
                        created_at: market.created_at,
                    });
                }
                
                let response = SuccessResponse::new_multiple(
                    true,
                    "Markets fetched successfully".to_string(),
                    markets_with_tokens
                );
                HttpResponse::Ok().json(response)
            }
            Err(e) => {
                println!("Error fetching markets: {:?}", e);
                let response = ErrorResponse::new("Error fetching markets");
                HttpResponse::InternalServerError().json(response)
            }
        }
}

#[get("/markets")]
pub async fn get_public_markets() -> impl Responder {
    let mut connection = establish_connection();

    let markets_result = markets::table
        .select(Market::as_select())
        .load::<Market>(&mut connection);

        match markets_result {
            Ok(markets) => {
                let mut markets_with_tokens = Vec::new();
                
                for market in markets {
                    // Get base token
                    let base_token = match tokens::table
                        .filter(tokens::id.eq(market.base_currency_id))
                        .filter(tokens::is_active.eq(true))
                        .first::<Token>(&mut connection) {
                        Ok(token) => token,
                        Err(_) => continue, // Skip if token not found
                    };
    
                    // Get quote token
                    let quote_token = match tokens::table
                        .filter(tokens::id.eq(market.quote_currency_id))
                        .first::<Token>(&mut connection) {
                        Ok(token) => token,
                        Err(_) => continue, // Skip if token not found
                    };
    
                    markets_with_tokens.push(MarketWithTokens {
                        id: market.id,
                        symbol: market.symbol,
                        base_currency: base_token,
                        quote_currency: quote_token,
                        min_order_size: market.min_order_size,
                        tick_size: market.tick_size,
                        is_active: market.is_active,
                        created_at: market.created_at,
                    });
                }
                
                let response = SuccessResponse::new_multiple(
                    true,
                    "Markets fetched successfully".to_string(),
                    markets_with_tokens
                );
                HttpResponse::Ok().json(response)
            }
            Err(e) => {
                println!("Error fetching markets: {:?}", e);
                let response = ErrorResponse::new("Error fetching markets");
                HttpResponse::InternalServerError().json(response)
            }
        }
}

#[put("/markets/{id}")]
pub async fn update_market(
    path: Path<Uuid>, 
    body: Json<UpdateMarketRequest>, 
    req: HttpRequest
) -> impl Responder {
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let market_id = path.into_inner();
    let mut connection = establish_connection();

    // Check if market exists
    let existing_market = match markets::table
        .filter(markets::id.eq(market_id))
        .first::<Market>(&mut connection) {
        Ok(market) => market,
        Err(_) => {
            let response = ErrorResponse::new("Market not found");
            return HttpResponse::NotFound().json(response);
        }
    };

    // If updating symbol, check if new symbol is already taken
    if let Some(ref new_symbol) = body.symbol {
        if new_symbol != &existing_market.symbol {
            if let Ok(_) = markets::table
                .filter(markets::symbol.eq(new_symbol))
                .first::<Market>(&mut connection) {
                let response = ErrorResponse::new("Market with this symbol already exists");
                return HttpResponse::BadRequest().json(response);
            }
        }
    }

    // Validate token IDs if provided
    let mut base_currency_id = existing_market.base_currency_id;
    let mut quote_currency_id = existing_market.quote_currency_id;

    if let Some(new_base_id) = body.base_currency_id {
        match tokens::table
            .filter(tokens::id.eq(new_base_id))
            .filter(tokens::is_active.eq(true))
            .first::<Token>(&mut connection) {
            Ok(_) => base_currency_id = new_base_id,
            Err(_) => {
                let response = ErrorResponse::new("Base currency token not found or inactive");
                return HttpResponse::BadRequest().json(response);
            }
        }
    }

    if let Some(new_quote_id) = body.quote_currency_id {
        match tokens::table
            .filter(tokens::id.eq(new_quote_id))
            .filter(tokens::is_active.eq(true))
            .first::<Token>(&mut connection) {
            Ok(_) => quote_currency_id = new_quote_id,
            Err(_) => {
                let response = ErrorResponse::new("Quote currency token not found or inactive");
                return HttpResponse::BadRequest().json(response);
            }
        }
    }

    // Apply updates
    let symbol = body.symbol.as_ref().unwrap_or(&existing_market.symbol);
    let min_order_size = body.min_order_size.unwrap_or(existing_market.min_order_size);
    let tick_size = body.tick_size.unwrap_or(existing_market.tick_size);
    let is_active = body.is_active.unwrap_or(existing_market.is_active);

    let result = diesel::update(markets::table.filter(markets::id.eq(market_id)))
        .set((
            markets::symbol.eq(symbol),
            markets::base_currency_id.eq(base_currency_id),
            markets::quote_currency_id.eq(quote_currency_id),
            markets::min_order_size.eq(min_order_size),
            markets::tick_size.eq(tick_size),
            markets::is_active.eq(is_active),
        ))
        .get_result::<Market>(&mut connection);

    match result {
        Ok(updated_market) => {
            // Get token details for response
            let base_token = tokens::table
                .filter(tokens::id.eq(updated_market.base_currency_id))
                .first::<Token>(&mut connection)
                .unwrap();
            
            let quote_token = tokens::table
                .filter(tokens::id.eq(updated_market.quote_currency_id))
                .first::<Token>(&mut connection)
                .unwrap();

            let market_response = MarketWithTokens {
                id: updated_market.id,
                symbol: updated_market.symbol,
                base_currency: base_token,
                quote_currency: quote_token,
                min_order_size: updated_market.min_order_size,
                tick_size: updated_market.tick_size,
                is_active: updated_market.is_active,
                created_at: updated_market.created_at,
            };

            let response = SuccessResponse::new_single(
                true,
                "Market updated successfully".to_string(),
                Some(market_response)
            );
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            println!("Error updating market: {:?}", e);
            let response = ErrorResponse::new("Error updating market");
            HttpResponse::InternalServerError().json(response)
        }
    }
}

#[delete("/markets/{id}")]
pub async fn delete_market(path: Path<Uuid>, req: HttpRequest) -> impl Responder {
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let market_id = path.into_inner();
    let mut connection = establish_connection();

    // Check if market exists
    match markets::table
        .filter(markets::id.eq(market_id))
        .first::<Market>(&mut connection) {
        Ok(_) => {
            // Deactivate market instead of hard delete
            match diesel::update(markets::table.filter(markets::id.eq(market_id)))
                .set(markets::is_active.eq(false))
                .get_result::<Market>(&mut connection) {
                Ok(updated_market) => {
                    // Get token details for response
                    let base_token = tokens::table
                        .filter(tokens::id.eq(updated_market.base_currency_id))
                        .first::<Token>(&mut connection)
                        .unwrap();
                    
                    let quote_token = tokens::table
                        .filter(tokens::id.eq(updated_market.quote_currency_id))
                        .first::<Token>(&mut connection)
                        .unwrap();

                    let market_response = MarketWithTokens {
                        id: updated_market.id,
                        symbol: updated_market.symbol,
                        base_currency: base_token,
                        quote_currency: quote_token,
                        min_order_size: updated_market.min_order_size,
                        tick_size: updated_market.tick_size,
                        is_active: updated_market.is_active,
                        created_at: updated_market.created_at,
                    };

                    let response = SuccessResponse::new_single(
                        true,
                        "Market deactivated successfully".to_string(),
                        Some(market_response)
                    );
                    HttpResponse::Ok().json(response)
                }
                Err(e) => {
                    println!("Error deactivating market: {:?}", e);
                    let response = ErrorResponse::new("Deactivation failed");
                    HttpResponse::InternalServerError().json(response)
                }
            }
        }
        Err(_) => {
            let response = ErrorResponse::new("Market not found");
            HttpResponse::NotFound().json(response)
        }
    }
}