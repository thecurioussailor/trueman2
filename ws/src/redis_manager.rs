use redis::aio::ConnectionLike;
use redis::{AsyncCommands, RedisResult};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::types::{Feed, ServerMsg};
use crate::user_manager::{UserManager, SubKey};

fn parse_channel(channel: &str) -> Option<(Feed, Uuid)> {
    // expected: "depth:<uuid>", "ticker:<uuid>", "trades:<uuid>"
    let (pfx, rest) = channel.split_once(':')?;
    let id = Uuid::parse_str(rest).ok()?;
    let feed = match pfx {
        "depth" => Feed::Depth,
        "ticker" => Feed::Ticker,
        "trades" => Feed::Trades,
        _ => return None,
    };
    Some((feed, id))
}

pub async fn run_redis_listener(redis_url: &str, users: UserManager) -> RedisResult<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;
    let mut pubsub = conn.into_pubsub();
    pubsub.psubscribe("depth:*").await?;
    pubsub.psubscribe("ticker:*").await?;
    pubsub.psubscribe("trades:*").await?;

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let channel: String = msg.get_channel_name().into();
        let payload: String = msg.get_payload()?;
        if let Some((feed, market_id)) = parse_channel(&channel) {
            // Try parse JSON; if not JSON, wrap as string
            let json = serde_json::from_str::<serde_json::Value>(&payload)
                .unwrap_or_else(|_| serde_json::Value::String(payload.clone()));
            users
                .broadcast((feed, market_id) as SubKey, ServerMsg::Event { channel, payload: json })
                .await;
        }
    }
    Ok(())
}