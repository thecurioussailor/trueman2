use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

// User model for inserting
#[derive(diesel::Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
}

// User model for querying
#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: NaiveDateTime,
}

// Token models
#[derive(diesel::Insertable)]
#[diesel(table_name = crate::schema::tokens)]
pub struct NewToken {
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_active: Option<bool>,
}

#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

// Market models
#[derive(diesel::Insertable, Deserialize)]
#[diesel(table_name = crate::schema::markets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMarket {
    pub symbol: String,
    pub base_currency_id: Uuid,
    pub quote_currency_id: Uuid,
    pub min_order_size: i64,
    pub tick_size: i64,
    pub is_active: Option<bool>,
}

#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::markets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Market {
    pub id: Uuid,
    pub symbol: String,
    pub base_currency_id: Uuid,
    pub quote_currency_id: Uuid,
    pub min_order_size: i64,
    pub tick_size: i64,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

// Response models for API (with joined data)
#[derive(Serialize)]
pub struct MarketResponse {
    pub id: Uuid,
    pub symbol: String,
    pub base_currency: Token,
    pub quote_currency: Token,
    pub min_order_size: i64,
    pub tick_size: i64,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Buy => write!(f, "Buy"),
            OrderType::Sell => write!(f, "Sell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderKind {
    Market,
    Limit,
}

impl std::fmt::Display for OrderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderKind::Market => write!(f, "Market"),
            OrderKind::Limit => write!(f, "Limit"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "Pending"),
            OrderStatus::PartiallyFilled => write!(f, "PartiallyFilled"),
            OrderStatus::Filled => write!(f, "Filled"),
            OrderStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

// Order models
#[derive(diesel::Insertable, Deserialize)]
#[diesel(table_name = crate::schema::orders)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewOrder {
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String, // Will be converted from OrderType enum
    pub order_kind: String, // Will be converted from OrderKind enum
    pub price: Option<i64>, // NULL for market orders
    pub quantity: i64,
    pub filled_quantity: Option<i64>, // Optional since it has a default
    pub status: Option<String>, // Optional since it has a default
}

#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::orders)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub order_type: String,
    pub order_kind: String,
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// Trade models
#[derive(diesel::Insertable, Deserialize)]
#[diesel(table_name = crate::schema::trades)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTrade {
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: i64,
    pub quantity: i64,
}

#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::trades)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Trade {
    pub id: Uuid,
    pub market_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: i64,
    pub quantity: i64,
    pub created_at: NaiveDateTime,
}

// Response models for API (with joined data)
#[derive(Serialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market: Market,
    pub order_type: String,
    pub order_kind: String,
    pub price: Option<i64>,
    pub quantity: i64,
    pub filled_quantity: i64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct TradeResponse {
    pub id: Uuid,
    pub market: Market,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: i64,
    pub quantity: i64,
    pub created_at: NaiveDateTime,
}

// Helper implementations for enum conversions
impl NewOrder {
    pub fn new(
        user_id: Uuid,
        market_id: Uuid,
        order_type: OrderType,
        order_kind: OrderKind,
        price: Option<i64>,
        quantity: i64,
    ) -> Self {
        Self {
            user_id,
            market_id,
            order_type: order_type.to_string(),
            order_kind: order_kind.to_string(),
            price,
            quantity,
            filled_quantity: None, // Will use default (0)
            status: None, // Will use default (PENDING)
        }
    }
}

impl Order {
    pub fn order_type_enum(&self) -> Result<OrderType, String> {
        match self.order_type.as_str() {
            "Buy" => Ok(OrderType::Buy),
            "Sell" => Ok(OrderType::Sell),
            _ => Err(format!("Invalid order type: {}", self.order_type)),
        }
    }

    pub fn order_kind_enum(&self) -> Result<OrderKind, String> {
        match self.order_kind.as_str() {
            "Market" => Ok(OrderKind::Market),
            "Limit" => Ok(OrderKind::Limit),
            _ => Err(format!("Invalid order kind: {}", self.order_kind)),
        }
    }

    pub fn status_enum(&self) -> Result<OrderStatus, String> {
        match self.status.as_str() {
            "Pending" => Ok(OrderStatus::Pending),
            "PartiallyFilled" => Ok(OrderStatus::PartiallyFilled),
            "Filled" => Ok(OrderStatus::Filled),
            "Cancelled" => Ok(OrderStatus::Cancelled),
            _ => Err(format!("Invalid order status: {}", self.status)),
        }
    }
}

// Balance models
#[derive(diesel::Insertable)]
#[diesel(table_name = crate::schema::balances)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewBalance {
    pub user_id: Uuid,
    pub token_id: Uuid,
    pub amount: Option<i64>, // Optional since it has a default of 0
    pub locked_amount: Option<i64>, // Optional since it has a default of 0
}

#[derive(diesel::Queryable, diesel::Selectable, Serialize)]
#[diesel(table_name = crate::schema::balances)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Balance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_id: Uuid,
    pub amount: i64,
    pub locked_amount: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// Response models for API
#[derive(Serialize)]
pub struct BalanceResponse {
    pub token: Token,
    pub amount: i64,
    pub locked_amount: i64,
    pub available_amount: i64, // amount - locked_amount
}

#[derive(Serialize)]
pub struct UserBalancesResponse {
    pub user_id: Uuid,
    pub balances: Vec<BalanceResponse>,
}

// Request models for deposits/withdrawals
#[derive(Deserialize)]
pub struct DepositRequest {
    pub token_id: Uuid,
    pub amount: i64,
}

#[derive(Deserialize)]
pub struct WithdrawRequest {
    pub token_id: Uuid,
    pub amount: i64,
}

#[derive(Serialize)]
pub struct TransactionResponse {
    pub success: bool,
    pub message: String,
    pub new_balance: Option<i64>,
}

// Helper implementations
impl NewBalance {
    pub fn new(user_id: Uuid, token_id: Uuid, amount: i64) -> Self {
        Self {
            user_id,
            token_id,
            amount: Some(amount),
            locked_amount: Some(0),
        }
    }
}

impl Balance {
    pub fn available_amount(&self) -> i64 {
        self.amount - self.locked_amount
    }
    
    pub fn can_withdraw(&self, amount: i64) -> bool {
        self.available_amount() >= amount
    }
    
    pub fn can_lock(&self, amount: i64) -> bool {
        self.available_amount() >= amount
    }
}

impl From<(Balance, Token)> for BalanceResponse {
    fn from((balance, token): (Balance, Token)) -> Self {
        Self {
            token,
            amount: balance.amount,
            locked_amount: balance.locked_amount,
            available_amount: balance.available_amount(),
        }
    }
}