use axum::{http::Method, Router};
use tower_http::cors::{self, CorsLayer};

pub trait CorsExt<S> {
    fn with_cors(self) -> Router<S>;
}

impl<S> CorsExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Add CORS to Router
    fn with_cors(self) -> Router<S> {
        let cors_layer = CorsLayer::new()
            .allow_origin(cors::AllowOrigin::predicate(|origin, _| {
                let origin = origin.as_bytes();
                origin.ends_with(b".dns.podman") || origin.starts_with(b"https://localhost:3333")
            }))
            .allow_methods([Method::GET, Method::POST, Method::PUT])
            .allow_headers(cors::Any);

        self.route_layer(cors_layer)
    }
}
