use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feed {
    Depth,
    Ticker,
    Trades,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ClientMsg {
    #[serde(rename = "subscribe")]
    Subscribe { market_id: Uuid, feeds: Vec<Feed> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { market_id: Uuid, feeds: Vec<Feed> },
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "info")]
    Info { message: String },
    // Raw pass-through from Redis (already JSON strings), but we wrap with meta
    #[serde(rename = "event")]
    Event { channel: String, payload: serde_json::Value },
}