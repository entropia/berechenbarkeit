use askama::Template;
use axum::response::IntoResponse;
use crate::HtmlTemplate;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {}

pub async fn home() -> impl IntoResponse {
    HtmlTemplate(HomeTemplate {})
}
