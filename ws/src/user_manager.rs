use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::types::{Feed, ServerMsg};

pub type ClientId = Uuid;
pub type SubKey = (Feed, Uuid); // (feed, market_id)
pub type Tx = mpsc::Sender<ServerMsg>;
pub type Rx = mpsc::Receiver<ServerMsg>;

#[derive(Default)]
pub struct Inner {
    // client_id -> sender
    pub clients: HashMap<ClientId, Tx>,
    // client_id -> set of subscriptions
    pub client_subs: HashMap<ClientId, HashSet<SubKey>>,
    // reverse index: subscription -> set of client_ids
    pub subs_index: HashMap<SubKey, HashSet<ClientId>>,
}

#[derive(Clone, Default)]
pub struct UserManager {
    pub inner: Arc<RwLock<Inner>>,
}

impl UserManager {
    pub async fn register_client(&self) -> (ClientId, Rx) {
        let (tx, rx) = mpsc::channel(1024);
        let id = Uuid::new_v4();
        let mut g = self.inner.write().await;
        g.clients.insert(id, tx);
        g.client_subs.insert(id, HashSet::new());
        (id, rx)
    }

    pub async fn unregister_client(&self, id: ClientId) {
        let mut g = self.inner.write().await;
        if let Some(set) = g.client_subs.remove(&id) {
            for sub in set {
                if let Some(s) = g.subs_index.get_mut(&sub) {
                    s.remove(&id);
                    if s.is_empty() { g.subs_index.remove(&sub); }
                }
            }
        }
        g.clients.remove(&id);
    }

    pub async fn subscribe(&self, id: ClientId, sub: SubKey) {
        let mut g = self.inner.write().await;
        if let Some(set) = g.client_subs.get_mut(&id) {
            set.insert(sub);
        }
        g.subs_index.entry(sub).or_default().insert(id);
    }

    pub async fn unsubscribe(&self, id: ClientId, sub: SubKey) {
        let mut g = self.inner.write().await;
        if let Some(set) = g.client_subs.get_mut(&id) {
            set.remove(&sub);
        }
        if let Some(s) = g.subs_index.get_mut(&sub) {
            s.remove(&id);
            if s.is_empty() { g.subs_index.remove(&sub); }
        }
    }

    pub async fn broadcast(&self, sub: SubKey, msg: ServerMsg) {
        let g = self.inner.read().await;
        if let Some(ids) = g.subs_index.get(&sub) {
            for id in ids {
                if let Some(tx) = g.clients.get(id) {
                    let _ = tx.try_send(msg.clone());
                }
            }
        }
    }
}