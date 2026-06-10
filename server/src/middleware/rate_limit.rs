//! Rate limiting middleware per IP (sliding window).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }
}

pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    let now = Instant::now();
    let allowed = {
        let mut map = limiter.state.lock().await;
        let timestamps = map.entry(ip).or_default();
        // Drop timestamps outside the window
        while timestamps.front().map_or(false, |t| now.duration_since(*t) > limiter.window) {
            timestamps.pop_front();
        }
        if timestamps.len() < limiter.max_requests {
            timestamps.push_back(now);
            true
        } else {
            false
        }
    };

    if allowed {
        next.run(req).await
    } else {
        (StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response()
    }
}
