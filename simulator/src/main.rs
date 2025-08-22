// simulator/src/main.rs
use rand::{rngs::StdRng, Rng, SeedableRng};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, Instant};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
struct StartMsg {
    market_id: Uuid,
    base_token_id: Uuid,
    quote_token_id: Uuid,
    users: usize,
    base_deposit: f64,
    quote_deposit: f64,
    order_rate_ms: u64,
    min_qty: f64,
    max_qty: f64,
    start_mid: f64,
    tick: f64,
}

#[derive(Clone)]
struct UserCtx { email: String, token: String }

#[derive(Serialize)]
struct Signup { email: String, password: String }

#[derive(Deserialize)]
struct LoginResp { token: String }

#[derive(Serialize)]
struct DepositReq { 
    token_id: Uuid, 
    amount: f64  // Change to f64 if your API expects decimal deposits
}

#[derive(Serialize)]
struct CreateOrderReq {
    market_id: Uuid,
    order_type: String, // "Buy" | "Sell"
    order_kind: String, // "Limit"
    price: Option<f64>,
    quantity: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into());
    let api = std::env::var("API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    control_loop(redis_url, api).await
}

async fn control_loop(redis_url: String, api: String) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;

    // Create consumer group once
    let _: Result<(), _> = redis::cmd("XGROUP")
        .arg("CREATE").arg("simulator_control").arg("simgrp").arg("$").arg("MKSTREAM")
        .query_async(&mut conn).await;

    let consumer = format!("sim-{}", Uuid::new_v4());
    loop {
        let v: redis::Value = redis::cmd("XREADGROUP")
            .arg("GROUP").arg("simgrp").arg(&consumer)
            .arg("BLOCK").arg(0)
            .arg("COUNT").arg(1)
            .arg("STREAMS").arg("simulator_control").arg(">")
            .query_async(&mut conn).await?;

        if let Some((stream, entries)) = parse_stream(v) {
            for (id, fields) in entries {
                if fields.get("type").map(|s| s.as_str()) == Some("start") {
                    if let Some(data) = fields.get("data") {
                        if let Ok(cfg) = serde_json::from_str::<StartMsg>(data) {
                            if let Err(e) = run_sim_for_10s(api.clone(), cfg).await {
                                eprintln!("sim error: {e:?}");
                            }
                        }
                    }
                }
                let _: () = redis::cmd("XACK").arg(&stream).arg("simgrp").arg(id)
                    .query_async(&mut conn).await.unwrap_or(());
            }
        }
    }
}

fn parse_stream(v: redis::Value) -> Option<(String, Vec<(String, std::collections::HashMap<String, String>)>)> {
    use redis::Value::*;
    if let Bulk(streams) = v {
        if let Some(Bulk(stream)) = streams.get(0) {
            if let (Some(Data(key)), Some(Bulk(entries))) = (stream.get(0), stream.get(1)) {
                let stream_key = String::from_utf8_lossy(key).to_string();
                let mut out = Vec::new();
                for e in entries {
                    if let Bulk(parts) = e {
                        let id = if let Some(Data(i)) = parts.get(0) { String::from_utf8_lossy(i).to_string() } else { continue };
                        let mut map = std::collections::HashMap::new();
                        if let Some(Bulk(kvs)) = parts.get(1) {
                            let mut it = kvs.iter();
                            while let (Some(Data(k)), Some(Data(v))) = (it.next(), it.next()) {
                                map.insert(String::from_utf8_lossy(k).to_string(), String::from_utf8_lossy(v).to_string());
                            }
                        }
                        out.push((id, map));
                    }
                }
                return Some((stream_key, out));
            }
        }
    }
    None
}

async fn run_sim_for_10s(api: String, cfg: StartMsg) -> anyhow::Result<()> {
    let client = Client::builder().build()?;
    // Users
    let mut users = Vec::new();
    for i in 0..cfg.users {
        let email = format!("demo+bot{:03}@example.com", i);
        let token = ensure_user(&client, &api, &email, "password").await?;
        // Deposit both sides to simplify
        deposit(&client, &api, &token, cfg.base_token_id, cfg.base_deposit).await?;
        deposit(&client, &api, &token, cfg.quote_token_id, cfg.quote_deposit).await?;
        users.push(UserCtx { email, token });
    }

    // Run for 10 seconds
    let end = Instant::now() + Duration::from_secs(5);
    let mut rng = StdRng::from_seed([0x42; 32]);
    let mut mid = cfg.start_mid;

    let min_price = mid - cfg.tick * 5.0;
    let max_price = mid + cfg.tick * 5.0;

    while Instant::now() < end {
        // drift mid
        let drift = rng.random_range(-5.0..=5.0);
        mid = (mid + drift).max(cfg.tick);

        // Round mid to 2 decimal places
        mid = (mid * 100.0).round() / 100.0;

        // pick user and params
        let u = &users[rng.random_range(0..users.len())];
        let is_buy = rng.random_bool(0.5);
        let is_limit = true;
        let qty = rng.random_range(cfg.min_qty..=cfg.max_qty);
        
        let price = if is_limit {
            // Generate spread around current mid price
            let spread_range = rng.random_range(0.01..=2.0); // 1 cent to $2 spread
            let raw_price = if is_buy { 
                mid - spread_range  // Buy below mid
            } else { 
                mid + spread_range  // Sell above mid
            };
            
            // Round to 2 decimal places and ensure minimum price
            let rounded_price = ((raw_price * 100.0).round() / 100.0).max(0.01);
            Some(rounded_price)
        } else { 
            None 
        };

        let req = CreateOrderReq {
            market_id: cfg.market_id,
            order_type: if is_buy { "Buy".into() } else { "Sell".into() },
            order_kind: if is_limit { "Limit".into() } else { "Market".into() },
            price,
            quantity: qty,
        };
        let _ = place_order(&client, &api, &u.token, &req).await;
        sleep(Duration::from_millis(cfg.order_rate_ms)).await;
    }
    Ok(())
}

async fn ensure_user(client: &Client, api: &str, email: &str, pass: &str) -> anyhow::Result<String> {
    let _ = client.post(format!("{api}/signup")).json(&Signup { email: email.into(), password: pass.into() }).send().await;
    let r = client.post(format!("{api}/login")).json(&Signup { email: email.into(), password: pass.into() }).send().await?;
    if r.status() != StatusCode::OK { anyhow::bail!("login failed {}", r.status()) }
    Ok(r.json::<LoginResp>().await?.token)
}
async fn deposit(client: &Client, api: &str, jwt: &str, token_id: Uuid, amount: f64) -> anyhow::Result<()> {
    let r = client.post(format!("{api}/user/deposit")).bearer_auth(jwt).json(&DepositReq { token_id, amount }).send().await?;
    if !r.status().is_success() { anyhow::bail!("deposit failed: {}", r.text().await?) }
    Ok(())
}
async fn place_order(client: &Client, api: &str, jwt: &str, req: &CreateOrderReq) -> anyhow::Result<()> {
    let r = client.post(format!("{api}/user/orders")).bearer_auth(jwt).json(req).send().await?;
    if !r.status().is_success() { eprintln!("order failed: {}", r.text().await?); }
    Ok(())
}