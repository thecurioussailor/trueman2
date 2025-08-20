use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::middleware::Logger;
use tracing_subscriber::EnvFilter;

pub mod routes;
pub mod jwt;
pub mod redis_manager;
pub mod registry;
pub mod decimal_utils;
use routes::{
    auth::{login, signup, admin_login},
    token::{create_token, get_tokens, update_token, delete_token, get_public_tokens},
    market::{create_market, get_markets, update_market, delete_market, get_public_markets},
    balance::{get_user_balance, deposit_funds, withdraw_funds},
    order::{create_order, cancel_order, get_orders},
    trade::get_trades,
    simulator::start_simulator,
};
use routes::test::{get_user_profile, admin_dashboard};
use jwt::{admin_auth, user_auth};
use registry::load_registry;
use std::time::Duration;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();    
    // Initial registry load (tokens + markets)
    {
        let mut conn = database::establish_connection();
        load_registry(&mut conn).expect("failed to load registry");
        let (tok_cnt, mkt_cnt) = registry::counts();
        tracing::info!("Registry loaded: {} tokens, {} markets", tok_cnt, mkt_cnt);
    }

    // Periodic refresh (simple loop; swap to LISTEN/NOTIFY or pubsub later)
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(Duration::from_secs(60)).await;
            let mut conn = database::establish_connection();
            if let Err(e) = load_registry(&mut conn) {
                tracing::warn!("Registry refresh failed: {}", e);
            } else {
                let (tok_cnt, mkt_cnt) = registry::counts();
                tracing::info!("Registry refreshed: {} tokens, {} markets", tok_cnt, mkt_cnt);
            }
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
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
                            .service(start_simulator)
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