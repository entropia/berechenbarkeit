use askama::Template;
use axum::{
    Form,
    extract::Path,
    http::HeaderMap,
};
use axum_core::response::IntoResponse;
use serde::Deserialize;
use crate::{
    AppError,
    HtmlTemplate,
    utils::make_htmx_redirect,
};
use crate::db::{
    util::DatabaseConnection,
    cost_centres::DBCostCentre
};

#[derive(Template)]
#[template(path = "cost_centre/list.html")]
struct CostCentreListTemplate {
    cost_centres: Vec<DBCostCentre>,
}

pub(crate) async fn cost_centre_list(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let cost_centres = DBCostCentre::get_all(&mut conn).await?;
    Ok(HtmlTemplate(CostCentreListTemplate { cost_centres }))

}

#[derive(Deserialize, Debug)]
pub(crate) struct CostCentreFormInput {
    name: String,
}

pub(crate) async fn cost_centre_add(
    request_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Form(form): Form<CostCentreFormInput>,
) -> Result<impl IntoResponse, AppError> {
    DBCostCentre::insert(&form.name, &mut conn).await?;
    make_htmx_redirect(request_headers, "/cost_centres")
}

pub(crate) async fn update(
    request_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Path(cost_centre_id): Path<i64>,
    Form(cost_centre_form): Form<CostCentreFormInput>,
) -> Result<impl IntoResponse, AppError> {
    DBCostCentre::update(cost_centre_id, &cost_centre_form.name, &mut conn).await?;
    make_htmx_redirect(request_headers, "/cost_centres")
}

pub(crate) async fn cost_centre_delete(
    request_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Path(cost_centre_id): Path<i64>
) -> Result<impl IntoResponse, AppError> {
    DBCostCentre::delete(cost_centre_id, &mut conn).await?;
    make_htmx_redirect(request_headers, "cost_centres")
}
