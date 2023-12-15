use tracing_subscriber::{filter, prelude::*};

pub trait HttpTracingExt: tracing::Subscriber {
    fn with_http_tracing(self) -> tracing_subscriber::layer::Layered<filter::Targets, Self>
    where
        Self: Sized,
    {
        self.with(
            filter::Targets::new()
                // .with_target("tower_http::trace::on_response", tracing::Level::TRACE)
                // .with_target("tower_http::trace::on_request", tracing::Level::TRACE)
                .with_default(tracing::Level::INFO),
        )
    }
}

impl<S: tracing::Subscriber> HttpTracingExt for S {}
