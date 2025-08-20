pub mod schema;
pub mod model;

use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub use model::{
    NewUser, User,
    NewToken, Token,
    NewMarket, Market,
    MarketResponse,
    OrderType, OrderKind, OrderStatus,
    NewOrder, Order,
    NewTrade, Trade,
    OrderResponse,
    TradeResponse,
    NewBalance, Balance,
    BalanceResponse,
    UserBalancesResponse,
    DecimalDepositRequest,
    DecimalWithdrawRequest,
    TransactionResponse,
};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}