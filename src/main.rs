use askama::Template;
use axum::{
    http::StatusCode,
    response::IntoResponse, Router,
};
use axum::extract::{DefaultBodyLimit, MatchedPath};
use axum::http::Request;
use axum::response::{Html, Response};
use axum::routing::{get, post};
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::config::Config;

pub mod handlers;
mod db;
mod config;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "berechenbarkeit=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::parse();

    let db_pool = PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .expect("sqlx: could not connect to database_url");

    // This embeds database migrations in the application binary so we can ensure the database
    // is migrated correctly on startup
    sqlx::migrate!().run(&db_pool).await.expect("sqlx: migration failed");

    let assets_base_path = match option_env!("BERECHENBARKEIT_STATIC_BASE_PATH") {
        Some(env) => env.to_string(),
        None => "src/assets".to_owned(),
    };

    let app = Router::new()
        .route("/invoices", get(handlers::invoice::invoice_list))
        .route("/invoice/upload", post(handlers::invoice::invoice_add_upload)).layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .route("/invoice/:invoice_id/invoiceitem/:invoiceitem_id/split", post(handlers::invoice::invoice_item_split))
        .route("/invoice/:invoice_id/edit", get(handlers::invoice::invoice_edit).post(handlers::invoice::invoice_edit_submit))
        .route("/invoice/:invoice_id/delete", get(handlers::invoice::invoice_delete_confirm).post(handlers::invoice::invoice_delete))
        .route("/cost_centres", get(handlers::cost_centre::cost_centre_list).post(handlers::cost_centre::cost_centre_add))
        .route("/cost_centre/:cost_centre_id/delete", get(handlers::cost_centre::cost_centre_delete))
        .route("/summary", get(handlers::summary::summary_overview))
        .route("/", get(handlers::home::home))
        .with_state(db_pool)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                    )
                })
        )
        .nest_service("/assets", ServeDir::new(assets_base_path));

    // run our app with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}


struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::debug!("stacktrace: {}", self.0.backtrace());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
    where
        T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}
