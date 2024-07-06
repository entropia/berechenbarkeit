use crate::HtmlTemplate;
use askama::Template;
use axum::response::IntoResponse;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {}

pub async fn home() -> impl IntoResponse {
    HtmlTemplate(HomeTemplate {})
}
