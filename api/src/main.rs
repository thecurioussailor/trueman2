use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::middleware::Logger;

pub mod routes;
pub mod jwt;

use routes::{
    auth::{login, signup, admin_login},
    token::{create_token, get_tokens, update_token, delete_token},
    market::{create_market},
    
};
use routes::test::{get_user_profile, admin_dashboard};
use jwt::{admin_auth, user_auth};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(signup)
            .service(login)
            .service(admin_login)
            .service(
                web::scope("/admin")
                    .wrap(HttpAuthentication::bearer(admin_auth))
                    .service(get_user_profile)
                    .service(admin_dashboard)
                    .service(create_token)
                    .service(get_tokens)
                    .service(update_token)
                    .service(delete_token)
                    .service(create_market)
                    // .service(get_markets)
                    // .service(update_market)
                    // .service(delete_market)
            )
            .service(
                web::scope("/user")
                    .wrap(HttpAuthentication::bearer(user_auth))
                    .service(get_user_profile)
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}