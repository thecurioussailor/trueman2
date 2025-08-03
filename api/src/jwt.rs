use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, Header, EncodingKey, DecodingKey, Validation};
use actix_web::{
    dev::ServiceRequest,
    Error,
    HttpMessage,
    error::{ErrorUnauthorized, ErrorForbidden},
};
#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub user_id: String,
    pub email: String,
    pub is_admin: bool,
    pub exp: usize,
}

const JWT_SECRET: &[u8] = b"secret";

pub fn create_jwt(user_id: String, email: String, is_admin: bool) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(1))
        .expect("Invalid expiration date");

    let claims = Claims { 
        user_id, 
        email,
        is_admin,
        exp: expiration.timestamp() as usize 
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))?;

    Ok(token)
}

pub fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
         &DecodingKey::from_secret(JWT_SECRET), 
         &Validation::default())?;
    Ok(token_data.claims)
}

// User middleware - just verify JWT, any authenticated user can access
pub async fn user_auth(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    match verify_jwt(credentials.token()) {
        Ok(token_data) => {
            // Add claims to request extensions so handlers can access them
            req.extensions_mut().insert(token_data);
            Ok(req)
        }
        Err(e) => {
            println!("JWT verification failed: {:?}", e);
            Err((ErrorUnauthorized("Invalid or expired token"), req))
        }
    }
}

// Admin middleware - verify JWT AND check if user is admin
pub async fn admin_auth(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    match verify_jwt(credentials.token()) {
        Ok(token_data) => {
            if token_data.is_admin {
                // Add claims to request extensions so handlers can access them
                req.extensions_mut().insert(token_data);
                Ok(req)
            } else {
                println!("Non-admin user attempted to access admin endpoint: {}", token_data.email);
                Err((ErrorForbidden("Admin access required"), req))
            }
        }
        Err(e) => {
            println!("JWT verification failed: {:?}", e);
            Err((ErrorUnauthorized("Invalid or expired token"), req))
        }
    }
}