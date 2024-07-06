use axum::http::HeaderMap;

use axum_core::response::IntoResponse;

use crate::AppError;

pub fn make_htmx_redirect(request_headers: HeaderMap, target: &str) -> Result<impl IntoResponse, AppError> {
    let mut headers = HeaderMap::new();
    let header_name: &str = request_headers.get("HX-Request").filter(|v| *v == "true").map_or_else(|| "Location", |_| "HX-Location");
    headers.insert(header_name, target.parse().unwrap());
    Ok(headers)
}
