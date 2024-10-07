#![allow(unused)]

use std::{any::type_name, fmt::Debug, hash::Hash, str::FromStr, sync::Arc, time::Duration};

use anyhow::Error;
use futures::{StreamExt, TryStreamExt};
use futures_util::{future, Future};
use lapin::{
    options::{BasicConsumeOptions, QueueBindOptions},
    types::{FieldTable, ShortString},
};
use serde::{Deserialize, Serialize};

use dashmap;
use tracing::{error, info, info_span, instrument, warn, Instrument, Span};
use uuid::Uuid;

use macros::ert;
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
    pub id: uuid::Uuid,
    pub rx: tokio::sync::mpsc::Receiver<Arc<serde_json::Value>>,
}

impl Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.id)
            .finish()
    }
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

impl Debug for Subscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("id", &self.id)
            .finish()
    }
}

pub struct PostsSubscriptionManager {
    subscriptions: dashmap::DashMap<Uuid, Subscriber>,
}

impl Debug for PostsSubscriptionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostsSubscriptionManager")
            .field("subscriptions_count", &self.subscriptions.len())
            .finish()
    }
}

impl PostsSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: dashmap::DashMap::new(),
        }
    }

    #[instrument]
    pub fn subscribe(&self) -> Subscription {
        let (tx, rx) = tokio::sync::mpsc::channel(24);
        let id = uuid::Uuid::now_v7();
        let sub = Subscriber { id, tx };
        info!(action = "subscribe", id = %sub.id);
        self.subscriptions.insert(id, sub);

        Subscription { id, rx }
    }

    #[instrument]
    pub fn unsubscribe(&self, s: &Uuid) -> Option<Uuid> {
        self.subscriptions.remove(s).map(|(id, _)| id)
    }
}

pub struct PostsBroker {
    span: Option<Span>,
    pub posts_subscription_mgr: Arc<PostsSubscriptionManager>,
    q_pool: deadpool_lapin::Pool,
}

impl PostsBroker {
    pub fn new(
        posts_subscription_mgr: Arc<PostsSubscriptionManager>,
        q_pool: deadpool_lapin::Pool,
    ) -> Self {
        Self {
            span: None,
            posts_subscription_mgr,
            q_pool,
        }
    }

    pub fn instrument(mut self, span: Span) -> Self {
        self.span.replace(span);
        self
    }

    pub async fn tick(self) {}

    pub async fn run(mut self) -> Result<(), Error> {
        // TODO: shutdown signal

        let posts_subscriber_mgr = &self.posts_subscription_mgr;

        info!("ahhhhhh");
        info!("get conn");
        let mut q_conn = self.q_pool.get().await.inspect_err(ert!())?;
        info!("get chan");
        let mut chan = q_conn.create_channel().await?;
        info!("chan id: {}", chan.id());
        let q = chan
            .queue_declare("posts", Default::default(), Default::default())
            .await?;
        let consumer = &mut chan
            .basic_consume(
                q.name().as_str(),
                "posts-consumer",
                Default::default(),
                Default::default(),
            )
            .await
            .inspect_err(ert!())?;

        consumer
            .into_stream()
            .inspect_err(ert!())
            .inspect_ok(|_| info!("new new!"))
            // ensure delivery success
            .filter_map(|maybe_delivery| {
                if let Err(ref e) = maybe_delivery {
                    error!(error = %e, "posts consume fail: {}", e);
                }
                future::ready(maybe_delivery.ok())
            })
            // ensure json
            .filter_map(|delivery| async move {
                info!("acking");
                delivery
                    .ack(Default::default())
                    .await
                    .inspect_err(ert!())
                    .ok();
                info!("acked");
                let content_type = delivery
                    .properties
                    .content_type()
                    .as_ref()
                    .map(ShortString::as_str)
                    .unwrap_or("");
                if content_type != "application/json" {
                    return None;
                }
                Some(delivery)
            })
            .filter_map(|delivery| async move {
                let v = std::str::from_utf8(&delivery.data)
                    .inspect_err(ert!())
                    .map_err(Error::from)
                    .and_then(|d| {
                        serde_json::Value::from_str(d)
                            .map_err(Error::from)
                            .inspect_err(ert!())
                    });
                info!("deserialized");

                if let Err(e) = &v {
                    error!(error = %e);
                }
                v.ok()
            })
            // TODO: maybe manage manually instead
            .for_each_concurrent(5, |v| async move {
                let v = Arc::new(v); // ensures no copy
                for sub in posts_subscriber_mgr.subscriptions.iter() {
                    info!("sending_post to: {sub_id}", sub_id = sub.id);
                    if let Err(e) = sub.tx.send_timeout(v.clone(), Duration::from_secs(5)).await {
                        error!(%e, "failed to send post to subscriber");
                    }
                }
            })
            .await;

        Ok(())
    }
}
