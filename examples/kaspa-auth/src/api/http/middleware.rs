use axum::middleware::Next;
use axum::response::Response;
use axum::extract::Request;
use log::info;

pub async fn rate_limiting(req: Request, next: Next) -> Response {
    // Rate limiting logic
    info!("Applying rate limiting middleware.");
    next.run(req).await
}

pub async fn logging(req: Request, next: Next) -> Response {
    // Logging logic
    info!("Applying logging middleware.");
    next.run(req).await
}
