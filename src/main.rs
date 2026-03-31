use callipsos_core::db;
use callipsos_core::routes::{self, AppState};
use callipsos_core::signing::lit::LitSigningProvider;
use callipsos_core::signing::SigningProvider;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    // Lit Protocol signing via Chipotle REST API
    // Requires all 3 env vars to be set; otherwise signing is disabled.
    let signing_provider: Option<Arc<dyn SigningProvider>> = {
        let api_url = std::env::var("LIT_API_URL").ok();
        let api_key = std::env::var("LIT_API_KEY").ok();
        let pkp_address = std::env::var("LIT_PKP_ADDRESS").ok();

        match (api_url, api_key, pkp_address) {
            (Some(url), Some(key), Some(pkp)) => {
                tracing::info!("Lit signing enabled via Chipotle at {}", url);
                let provider: Arc<dyn SigningProvider> =
                    Arc::new(LitSigningProvider::new(url, key, pkp));
                Some(provider)
            }
            _ => {
                tracing::info!("Lit signing disabled — set LIT_API_URL, LIT_API_KEY, LIT_PKP_ADDRESS to enable");
                None
            }
        }
    };

    let state = AppState {
        db: pool,
        signing_provider,
    };
    let app = routes::create_router(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("{host}:{port}");

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Listening on http://{bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
