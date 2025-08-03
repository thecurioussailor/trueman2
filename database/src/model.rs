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