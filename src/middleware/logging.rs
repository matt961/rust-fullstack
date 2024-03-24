use axum::Router;
use tracing::Span;

pub trait HttpLoggingExt<S> {
    fn with_http_logging(self) -> Self;
}

impl<S> HttpLoggingExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Add HTTP logging to Router
    fn with_http_logging(self) -> Router<S> {
        self.route_layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(|_r: &axum::http::Request<_>| tracing::info_span!("http-request"))
                .on_request(|request: &axum::http::Request<_>, _span: &Span| {
                    tracing::info!(target: "tower_http",
                        parent: _span,
                        path = ?request.uri().path_and_query().unwrap().as_str());
                })
                .on_response(|response: &axum::http::Response<_>, _, _span: &Span| {
                    tracing::info!(target: "tower_http",
                        parent: _span,
                        status = format!("{} {}",
                            response.status().as_str(),
                            response.status().canonical_reason().unwrap_or("")))
                }),
        )
    }
}
