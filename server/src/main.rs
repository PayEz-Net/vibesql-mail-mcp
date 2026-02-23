mod auth;
mod db;
mod error;
mod models;
mod routes;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::middleware::Logger;
use actix_web::{web, App, Error, HttpServer};
use log::info;
use std::env;
use std::time::Duration;

use db::DbClient;
use routes::stream::SseBroadcaster;

#[derive(Clone)]
struct AppConfig {
    port: u16,
    micro_url: String,
    secret: Option<String>,
    dev_mode: bool,
}

impl AppConfig {
    fn from_env() -> Self {
        // Try config file first, then env vars
        let config_path = dirs_path().join("server.toml");
        let file_config = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| s.parse::<toml::Table>().ok());

        let port = env::var("VIBESQL_MAIL_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| {
                file_config
                    .as_ref()
                    .and_then(|c| c.get("port"))
                    .and_then(|v| v.as_integer())
                    .map(|v| v as u16)
            })
            .unwrap_or(5188);

        let micro_url = env::var("VIBESQL_MAIL_MICRO_URL")
            .ok()
            .or_else(|| {
                file_config
                    .as_ref()
                    .and_then(|c| c.get("micro_url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "http://localhost:5173".to_string());

        let secret = env::var("VIBESQL_MAIL_SECRET").ok().or_else(|| {
            file_config
                .as_ref()
                .and_then(|c| c.get("secret_key"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        let dev_mode = env::var("VIBESQL_MAIL_DEV")
            .map(|v| v == "true" || v == "1")
            .unwrap_or_else(|_| {
                file_config
                    .as_ref()
                    .and_then(|c| c.get("dev_mode"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            });

        Self {
            port,
            micro_url,
            secret,
            dev_mode,
        }
    }
}

fn dirs_path() -> std::path::PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibesql-mail")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = AppConfig::from_env();
    let db = DbClient::new(&config.micro_url);

    // Run migrations on startup
    if let Err(e) = db.run_migrations().await {
        log::warn!("Migration warning (may be OK on first run without DB): {}", e);
    }

    let broadcaster = web::Data::new(SseBroadcaster::new());
    let broadcaster_clone = broadcaster.clone();

    // Heartbeat task — every 30 seconds
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            broadcaster_clone.heartbeat();
        }
    });

    let auth_secret = config.secret.clone();
    let auth_dev = config.dev_mode;
    let bind_addr = format!("127.0.0.1:{}", config.port);

    info!("vibesql-mail-server v{}", env!("CARGO_PKG_VERSION"));
    info!("Storage: vibesql-micro at {}", config.micro_url);
    info!("Listening: http://{}", bind_addr);
    info!(
        "Auth: {}",
        if config.dev_mode {
            "disabled (dev mode)"
        } else {
            "HMAC-SHA256"
        }
    );

    HttpServer::new(move || {
        let secret = auth_secret.clone();
        let dev = auth_dev;

        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(db.clone()))
            .app_data(broadcaster.clone())
            .app_data(web::Data::new((secret.clone(), dev)))
            // Health — no auth
            .route("/v1/mail/health", web::get().to(routes::admin::health))
            // All other routes go through auth check via guard
            .service(
                web::scope("/v1/mail")
                    .wrap(AuthMiddleware::new(secret, dev))
                    // Messages
                    .route("/send", web::post().to(routes::messages::send_message))
                    .route(
                        "/inbox/{agent}",
                        web::get().to(routes::messages::get_inbox),
                    )
                    .route(
                        "/messages/{id}",
                        web::get().to(routes::messages::read_message),
                    )
                    .route(
                        "/messages/{id}/read",
                        web::post().to(routes::messages::mark_read),
                    )
                    .route(
                        "/sent/{agent}",
                        web::get().to(routes::messages::get_sent),
                    )
                    // Agents
                    .route("/agents", web::get().to(routes::agents::list_agents))
                    .route("/agents", web::post().to(routes::agents::register_agent))
                    // Threads
                    .route(
                        "/threads/{thread_id}",
                        web::get().to(routes::threads::get_thread),
                    )
                    // SSE Stream
                    .route("/stream", web::get().to(routes::stream::event_stream))
                    // Admin
                    .route("/admin/init", web::post().to(routes::admin::init_db))
                    .route("/admin/status", web::get().to(routes::admin::status)),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}

// --- Auth Middleware ---

use futures::future::{ok, Ready};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

pub struct AuthMiddleware {
    secret: Option<String>,
    dev_mode: bool,
}

impl AuthMiddleware {
    pub fn new(secret: Option<String>, dev_mode: bool) -> Self {
        Self { secret, dev_mode }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService {
            service: Rc::new(service),
            secret: self.secret.clone(),
            dev_mode: self.dev_mode,
        })
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    secret: Option<String>,
    dev_mode: bool,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let secret = self.secret.clone();
        let dev_mode = self.dev_mode;

        Box::pin(async move {
            auth::check_auth(&req, &secret, dev_mode)?;
            svc.call(req).await
        })
    }
}
