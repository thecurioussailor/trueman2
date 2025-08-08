use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::middleware::Logger;

pub mod routes;
pub mod jwt;
pub mod redis_manager;

use routes::{
    auth::{login, signup, admin_login},
    token::{create_token, get_tokens, update_token, delete_token, get_public_tokens},
    market::{create_market, get_markets, update_market, delete_market, get_public_markets},
    balance::{get_user_balance, deposit_funds, withdraw_funds},
    order::{create_order, cancel_order, get_orders},
    trade::get_trades,
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
            .service(get_public_tokens)
            .service(get_public_markets)
            // Protected routes at root level
            .service(
                web::scope("/user")
                            .wrap(HttpAuthentication::bearer(user_auth))
                            .service(get_user_profile)
                            .service(get_user_balance)
                            .service(deposit_funds)
                            .service(withdraw_funds)
                            .service(create_order)
                            .service(cancel_order)
                            .service(get_orders)
                            .service(get_trades)
            )
            .service(
                web::scope("/admin")
                    .wrap(HttpAuthentication::bearer(admin_auth))
                    .service(admin_dashboard)
                    .service(create_token)
                    .service(get_tokens)
                    .service(update_token)
                    .service(delete_token)
                    .service(create_market)
                    .service(get_markets)
                    .service(update_market)
                    .service(delete_market)
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}