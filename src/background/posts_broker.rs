#![allow(unused)]

use std::{hash::Hash, str::FromStr, sync::Arc, time::Duration};

use anyhow::Error;
use deadpool_lapin::Timeouts;
use futures::{StreamExt, TryStreamExt};
use futures_util::{future, Future};
use lapin::{
    options::{BasicConsumeOptions, QueueBindOptions},
    types::{FieldTable, ShortString},
};
use serde::{Deserialize, Serialize};

use dashmap;
use tracing::{error, info, warn, Instrument};

use crate::error::AppError;

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
    tx: tokio::sync::mpsc::Sender<Arc<serde_json::Value>>,
}

pub struct PostsSubscriptionManager {
    subscriptions: dashmap::DashSet<Subscriber>,
}

impl PostsSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: dashmap::DashSet::new(),
        }
    }

    pub fn subscribe(&self) -> tokio::sync::mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = tokio::sync::mpsc::channel(24);
        let sub = Subscriber {
            id: uuid::Uuid::now_v7(),
            tx,
        };
        self.subscriptions.insert(sub);

        rx
    }
}

pub struct PostsBroker {
    pub posts_subscription_mgr: Arc<PostsSubscriptionManager>,
    q_pool: deadpool_lapin::Pool,
}

impl PostsBroker {
    pub fn new(
        posts_subscription_mgr: Arc<PostsSubscriptionManager>,
        q_pool: deadpool_lapin::Pool,
    ) -> Self {
        Self {
            posts_subscription_mgr,
            q_pool,
        }
    }

    pub async fn tick(self) {}

    pub async fn run(self) -> Result<(), Error> {
        // TODO: shutdown signal

        let pb = &self;
        let mut q_conn = pb.q_pool.get().in_current_span().await?;
        let mut chan = q_conn.create_channel().in_current_span().await?;
        let consumer = &mut chan
            .basic_consume(
                "posts",
                "",
                BasicConsumeOptions {
                    no_ack: true,
                    ..Default::default()
                },
                Default::default(),
            )
            .in_current_span()
            .await?;

        consumer
            .inspect(|_| {
                info!("new post")
            })
            // ensure delivery success
            .filter_map(|maybe_delivery| {
                if let Err(ref e) = maybe_delivery {
                    error!(error = %e, "posts consume fail: {}", e);
                }
                future::ready(maybe_delivery.ok())
            })
            // ensure json
            .filter(|delivery| {
                let content_type = delivery
                    .properties
                    .content_type()
                    .as_ref()
                    .map(ShortString::as_str)
                    .unwrap_or("");
                if content_type != "application/json" {
                    let delivery_id = delivery
                        .properties
                        .message_id()
                        .as_ref()
                        .map(ShortString::as_str)
                        .unwrap_or("n/a");
                    info!(%delivery_id);
                    return future::ready(false);
                };
                future::ready(true)
            })
            .filter_map(|delivery| {
                let v = std::str::from_utf8(&delivery.data)
                    .map_err(Error::from)
                    .and_then(|d| serde_json::Value::from_str(d).map_err(Error::from));

                if let Err(e) = &v {
                    error!(error = %e);
                }
                future::ready(v.ok())
            })
            // TODO: maybe manage manually instead
            .for_each_concurrent(5, |v| async move {
                let v = Arc::new(v); // ensures no copy
                for sub in pb.posts_subscription_mgr.subscriptions.iter() {
                    if let Err(e) = sub.tx.send_timeout(v.clone(), Duration::from_secs(5)).await {
                        error!(error = %e);
                    }
                }
            })
            .instrument(tracing::info_span!(target: "posts_consume", "posts_consume"));

        Ok(())
    }
}
