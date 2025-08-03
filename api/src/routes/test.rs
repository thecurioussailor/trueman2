use actix_web::{get, HttpResponse, Responder, HttpRequest, HttpMessage};
use serde::Serialize;
use crate::jwt::Claims;

#[derive(Serialize)]
struct UserInfoResponse {
    message: String,
    user_id: String,
    email: String,
    is_admin: bool,
}

// Protected route for any authenticated user
#[get("/profile")]
pub async fn get_user_profile(req: HttpRequest) -> impl Responder {
    // Get claims from request extensions (set by user_auth middleware)
    match req.extensions().get::<Claims>() {
        Some(claims) => {
            HttpResponse::Ok().json(UserInfoResponse {
                message: "User profile retrieved successfully".to_string(),
                user_id: claims.user_id.clone(),
                email: claims.email.clone(),
                is_admin: claims.is_admin,
            })
        }
        None => HttpResponse::Unauthorized().json("Authentication required"),
    }
}

// Protected route for admin users only
#[get("/dashboard")]
pub async fn admin_dashboard(req: HttpRequest) -> impl Responder {
    // Get claims from request extensions (set by admin_auth middleware)
    match req.extensions().get::<Claims>() {
        Some(claims) => {
            HttpResponse::Ok().json(UserInfoResponse {
                message: format!("Welcome to admin dashboard, {}!", claims.email),
                user_id: claims.user_id.clone(),
                email: claims.email.clone(),
                is_admin: claims.is_admin,
            })
        }
        None => HttpResponse::Unauthorized().json("{Error: Authentication required}"),
    }
}