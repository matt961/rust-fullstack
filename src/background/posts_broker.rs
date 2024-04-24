#![allow(unused)]

use std::{fmt::Debug, hash::Hash, str::FromStr, sync::Arc, time::Duration};

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
use tracing::{error, info, info_span, instrument, warn, Instrument, Span};

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
            subscriptions: dashmap::DashSet::new(),
        }
    }

    #[instrument]
    pub fn subscribe(&self) -> tokio::sync::mpsc::Receiver<Arc<serde_json::Value>> {
        let (tx, rx) = tokio::sync::mpsc::channel(24);
        let sub = Subscriber {
            id: uuid::Uuid::now_v7(),
            tx,
        };
        info!(action = "subscribe", id = %sub.id);
        self.subscriptions.insert(sub);

        rx
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
        let mut q_conn = self.q_pool.get().await.inspect_err(|e| error!(%e))?;
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
            .inspect_err(|e| error!(%e))?;

        consumer
            .into_stream()
            .inspect_err(|e| error!(%e))
            .inspect_ok(|_| info!("new new!"))
            // ensure delivery success
            .filter_map(|maybe_delivery| {
                if let Err(ref e) = maybe_delivery {
                    error!(error = %e, "posts consume fail: {}", e);
                }
                future::ready(maybe_delivery.ok())
            })
            // ensure json
            .filter_map(|delivery| {
                async move {
                    delivery
                        .ack(Default::default())
                        .await
                        .inspect_err(|e| error!(%e))
                        .ok();
                    let content_type = delivery
                        .properties
                        .content_type()
                        .as_ref()
                        .map(ShortString::as_str)
                        .unwrap_or("");
                    if content_type != "application/json" {
                        // let correlation_id = delivery
                        //     .properties
                        //     .correlation_id()
                        //     .as_ref()
                        //     .map(ShortString::as_str)
                        //     .unwrap_or("n/a");
                        // info!(%correlation_id);
                        return None;
                    }
                    Some(delivery)
                }
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
            .for_each_concurrent(5, |v| {
                async move {
                    let v = Arc::new(v); // ensures no copy
                    for sub in posts_subscriber_mgr.subscriptions.iter() {
                        info!(target: "sending_post", sub_id = %sub.id);
                        if let Err(e) = sub.tx.send_timeout(v.clone(), Duration::from_secs(5)).await
                        {
                            error!(error = %e);
                        }
                    }
                }
                .in_current_span()
            })
            .in_current_span()
            .await;

        Ok(())
    }
}
