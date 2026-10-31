//! Static host for the WASM app. Game logic runs in the browser today; when a
//! backend is needed, add routes to `api()` and reuse `dart-core` there.

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use std::{env, net::SocketAddr, path::PathBuf};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};

fn api() -> Router {
    Router::new()
}

/// Trunk emits content-hashed asset names, so those can be cached forever;
/// everything else (index.html, SPA routes) must revalidate.
async fn cache_headers(req: Request, next: Next) -> Response {
    let hashed = is_hashed_asset(req.uri().path());
    let mut res = next.run(req).await;
    let value = if hashed { "public, max-age=31536000, immutable" } else { "no-cache" };
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    res
}

fn is_hashed_asset(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or_default();
    file.split(['-', '_', '.'])
        .any(|part| part.len() == 16 && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// `dart-server healthcheck`: for Docker HEALTHCHECK in an image without curl.
fn healthcheck(port: u16) -> ! {
    use std::io::{Read, Write};
    let ok = std::net::TcpStream::connect(("127.0.0.1", port))
        .and_then(|mut s| {
            s.write_all(b"GET /healthz HTTP/1.0\r\n\r\n")?;
            let mut buf = String::new();
            s.read_to_string(&mut buf)?;
            Ok(buf.starts_with("HTTP/1.0 200") || buf.starts_with("HTTP/1.1 200"))
        })
        .unwrap_or(false);
    std::process::exit(if ok { 0 } else { 1 })
}

#[tokio::main]
async fn main() {
    let dist = PathBuf::from(env::var("DIST_DIR").unwrap_or_else(|_| "crates/web/dist".into()));
    let port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8020);
    if env::args().nth(1).as_deref() == Some("healthcheck") {
        healthcheck(port);
    }

    let spa = ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html")));
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api", api())
        .fallback_service(spa)
        .layer(middleware::from_fn(cache_headers))
        .layer(CompressionLayer::new());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    println!("serving {} on http://{addr}", dist.display());
    axum::serve(listener, app).with_graceful_shutdown(shutdown()).await.expect("server");
}

async fn shutdown() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };
    #[cfg(unix)]
    let term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<Option<()>>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hashed_assets() {
        assert!(super::is_hashed_asset("/dart-web-1a2b3c4d5e6f7a8b_bg.wasm"));
        assert!(super::is_hashed_asset("/app-0123456789abcdef.css"));
        assert!(!super::is_hashed_asset("/index.html"));
        assert!(!super::is_hashed_asset("/play"));
    }
}
