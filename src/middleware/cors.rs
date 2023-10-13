use axum::Router;
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
                origin.as_bytes().ends_with(b".dns.podman")
                    || origin.as_bytes().starts_with(b"https://localhost:3333")
            }))
            .allow_methods(cors::Any)
            .allow_headers(cors::Any);

        self.route_layer(cors_layer)
    }
}
