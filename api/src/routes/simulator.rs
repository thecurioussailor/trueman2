// api/src/routes/simulator.rs (new)
use actix_web::{post, web::Json, Responder, HttpResponse, HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;
use crate::jwt::Claims;

#[derive(Deserialize, Serialize, Debug)]
pub struct StartSimulatorRequest {
    pub market_id: Uuid,
    pub base_token_id: Uuid,
    pub quote_token_id: Uuid,
    pub users: usize,
    pub base_deposit: f64,
    pub quote_deposit: f64,
    pub order_rate_ms: u64,
    pub min_qty: f64,
    pub max_qty: f64,
    pub start_mid: f64,
    pub tick: f64,
}

#[post("/simulator/start")]
pub async fn start_simulator(req: HttpRequest, body: Json<StartSimulatorRequest>) -> impl Responder {
    // Extract user ID from JWT
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => match Uuid::parse_str(&claims.user_id) {
            Ok(uuid) => uuid,
            Err(_) => return HttpResponse::BadRequest().json("Invalid user ID format"),
        },
        None => return HttpResponse::Unauthorized().json("Authentication required"),
    };

    let msg_json = serde_json::to_string(&*body).unwrap();

    let client = match redis::Client::open("redis://127.0.0.1:6379/") {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(format!("redis error: {e}")),
    };
    let mut conn = match client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(format!("conn error: {e}")),
    };

    let res: Result<String, _> = redis::cmd("XADD")
        .arg("simulator_control")
        .arg("*")
        .arg("type").arg("start")
        .arg("data").arg(msg_json)
        .arg("timestamp").arg(Utc::now().timestamp_millis())
        .query_async(&mut conn)
        .await;

    match res {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({ "queued": true, "id": id })),
        Err(e) => HttpResponse::InternalServerError().json(format!("xadd error: {e}")),
    }
}