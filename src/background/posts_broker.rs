#![allow(unused)]

use std::{hash::Hash, sync::Arc};

use serde::{Deserialize, Serialize};

use dashmap;

impl PostsBroker {
    fn new(cfg: PostsBrokerConfig) -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostsBrokerConfig {
    n_workers: u32,
}

impl Default for PostsBrokerConfig {
    fn default() -> Self {
        Self { n_workers: 1 }
    }
}

// #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Subscription {
    rx: tokio::sync::mpsc::Receiver<serde_json::Value>,
}

impl Hash for Subscriber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Subscriber {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Subscriber {}

impl PartialOrd for Subscriber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

struct Subscriber {
    id: uuid::Uuid,
    tx: tokio::sync::mpsc::Sender<serde_json::Value>,
}

struct PostsSubscriptionManager {
    subscriptions: dashmap::DashSet<Subscriber>,
}

impl PostsSubscriptionManager {
    fn new() -> Self {
        Self {
            subscriptions: dashmap::DashSet::new(),
        }
    }
}

pub struct PostsBroker {
    posts_subscription_mgr: Arc<PostsSubscriptionManager>,
    msg_chan: lapin::Connection,
}

impl PostsBroker {
    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = tokio::sync::mpsc::channel(24);
        let sub = Subscriber {
            id: uuid::Uuid::now_v7(),
            tx,
        };
        self.posts_subscription_mgr.subscriptions.insert(sub);

        rx
    }
}
