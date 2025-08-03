use actix_web::{delete, get, post, put, web::{Json, Path}, HttpMessage, HttpRequest, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::jwt::Claims;
use diesel::prelude::*;
use uuid::Uuid;
use database::{
    establish_connection,
    NewToken, Token,
    schema::tokens,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateTokenRequest {
    symbol: String,
    name: String,
    decimals: i32,
}
#[derive(Deserialize)]
pub struct UpdateTokenRequest {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<i32>,
    pub is_active: Option<bool>,
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

//Helper function to convert Token to serde_json::Value
impl SuccessResponse {
    // For single token
    fn new_single(status: bool, message: String, data: Option<Token>) -> Self {
        Self {
            status,
            message,
            data: data.map(|token| serde_json::to_value(token).unwrap()),
        }
    }
    
    // For multiple tokens
    fn new_multiple(status: bool, message: String, data: Vec<Token>) -> Self {
        Self {
            status,
            message,
            data: Some(serde_json::to_value(data).unwrap()),
        }
    }
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

#[post("/tokens")]
pub async fn create_token(body: Json<CreateTokenRequest>, req: HttpRequest) -> impl Responder {
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse {
                status: true,
                message: "Unauthorized".to_string(),
                data: None,
            };
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let mut connection = establish_connection();

    if let Ok(_) = tokens::table
        .filter(tokens::symbol.eq(&body.symbol))
        .first::<Token>(&mut connection) {
        let response = ErrorResponse::new("Token already exists");
        return HttpResponse::BadRequest().json(response);
    }

    let new_token = NewToken {
        symbol: body.symbol.clone(),
        name: body.name.clone(),
        decimals: body.decimals,
        is_active: Some(true),
    };

    match diesel::insert_into(tokens::table)
        .values(&new_token)
        .get_result::<Token>(&mut connection) {
            Ok(token) => {
                let response = SuccessResponse::new_single(
                    true,
                    "Token created successfully".to_string(),
                    Some(token),
                );
                HttpResponse::Ok().json(response)
            }
            Err(e) => {
                println!("Error saving new token: {:?}", e);
                HttpResponse::InternalServerError().json("Error saving new token")
            }
        }
        
}

#[get("/tokens")]
pub async fn get_tokens(req: HttpRequest) -> impl Responder {

    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let mut connection = establish_connection();

    match tokens::table
        .select(Token::as_select())
        .load::<Token>(&mut connection) {
            Ok(tokens) => {
                let response = SuccessResponse::new_multiple(
                    true,
                    "Tokens fetched successfully".to_string(),
                    tokens,
                );
                HttpResponse::Ok().json(response)
            }
            Err(e) => {
                println!("Error fetching tokens: {:?}", e);
                let response = ErrorResponse::new("Error fetching tokens");
                HttpResponse::InternalServerError().json(response)
            }
        }
}

#[put("/tokens/{id}")]
pub async fn update_token(path: Path<Uuid>, body: Json<UpdateTokenRequest>,req: HttpRequest) -> impl Responder {

    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let token_id = path.into_inner();
    let mut connection = establish_connection();

    let existing_token = match tokens::table
        .filter(tokens::id.eq(token_id))
        .first::<Token>(&mut connection) {
            Ok(token) => token,
            Err(_) => {
                let response = ErrorResponse::new("Token not found");
                return HttpResponse::NotFound().json(response);
            }
        };

    if let Some(ref new_symbol) = body.symbol {
        if new_symbol.to_lowercase() != existing_token.symbol.to_lowercase() {
            if let Ok(_) = tokens::table
                .filter(tokens::symbol.eq(new_symbol))
                .first::<Token>(&mut connection) {
                    let response = ErrorResponse::new("Token with this symbol already exists");
                    return HttpResponse::BadRequest().json(response);
                }
        }  
    }

    let mut _query = diesel::update(tokens::table.filter(tokens::id.eq(token_id)));

    // Apply updates if provided
    let result = if body.symbol.is_some() || body.name.is_some() || body.decimals.is_some() || body.is_active.is_some() {
        // For simplicity, we'll update all fields. In production, you might want more granular updates
        let updated_token = tokens::table
            .filter(tokens::id.eq(token_id))
            .first::<Token>(&mut connection)
            .unwrap();

        let symbol = body.symbol.as_ref().unwrap_or(&updated_token.symbol);
        let name = body.name.as_ref().unwrap_or(&updated_token.name);
        let decimals = body.decimals.unwrap_or(updated_token.decimals);
        let is_active = body.is_active.unwrap_or(updated_token.is_active);

        diesel::update(tokens::table.filter(tokens::id.eq(token_id)))
            .set((
                tokens::symbol.eq(symbol),
                tokens::name.eq(name),
                tokens::decimals.eq(decimals),
                tokens::is_active.eq(is_active),
            ))
            .get_result::<Token>(&mut connection)
    } else {
        Ok(existing_token)
    };

    match result {
        Ok(updated_token) => {
            let response = SuccessResponse::new_single(
                true,
                "Token updated successfully".to_string(),
                Some(updated_token),
            );
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            println!("Error updating token: {:?}", e);
            let response = ErrorResponse::new("Error updating token");
            HttpResponse::InternalServerError().json(response)
        }
    }
}

#[delete("/tokens/{id}")]
pub async fn delete_token(path: Path<Uuid>, req: HttpRequest) -> impl Responder {
    // Get claims from request extensions (set by admin middleware)
    let _claims = match req.extensions().get::<Claims>() {
        Some(claims) => claims,
        None => {
            let response = ErrorResponse::new("Authentication required");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let token_id = path.into_inner();
    let mut connection = establish_connection();

    // Check if token exists
    match tokens::table
        .filter(tokens::id.eq(token_id))
        .first::<Token>(&mut connection) {
        Ok(_) => {
            // Deactivate token instead of hard delete
            match diesel::update(tokens::table.filter(tokens::id.eq(token_id)))
                .set(tokens::is_active.eq(false))
                .get_result::<Token>(&mut connection) {
                Ok(updated_token) => {
                    let response = SuccessResponse::new_single(
                        true,
                        "Token deactivated successfully".to_string(),
                        Some(updated_token),
                    );
                    HttpResponse::Ok().json(response)
                }
                Err(e) => {
                    println!("Error deactivating token: {:?}", e);
                    let response = ErrorResponse::new("Deactivation failed");
                    HttpResponse::InternalServerError().json(response)
                }
            }
        }
        Err(_) => {
            let response = ErrorResponse::new("Token not found");
            HttpResponse::NotFound().json(response)
        }
    }
}