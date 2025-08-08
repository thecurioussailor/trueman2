use actix_web::{get, HttpRequest,web,  HttpResponse, HttpMessage, Responder};
use diesel::prelude::*;
use database::{
    establish_connection, schema::trades::{self, dsl as trades_dsl}, Trade as DbTrade
};
use uuid::Uuid;
use crate::jwt::Claims;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TradesQuery {
    pub market_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[get("/trades")]
pub async fn get_trades(req: HttpRequest, query: web::Query<TradesQuery>) -> impl Responder {
    let uid = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let mut conn = establish_connection();
    use trades_dsl::{trades, buyer_user_id, seller_user_id, market_id, created_at};

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0);

    let mut q = trades
        .filter(buyer_user_id.eq(uid).or(seller_user_id.eq(uid)))
        .into_boxed();

    if let Some(mid) = query.market_id {
        q = q.filter(market_id.eq(mid));
    }

    match q
        .select(DbTrade::as_select())
        .order(created_at.desc())
        .limit(limit)
        .offset(offset)
        .load::<DbTrade>(&mut conn) {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => HttpResponse::InternalServerError().json(format!("DB error: {}", e)),
    }
}