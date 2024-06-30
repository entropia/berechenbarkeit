use askama::Template;
use axum::{
    Form,
    extract::Path,
    http::HeaderMap,
    response::Redirect,
};
use axum_core::response::IntoResponse;
use serde::Deserialize;
use crate::{AppError, HtmlTemplate};
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

pub(crate) async fn cost_centre_add(DatabaseConnection(mut conn): DatabaseConnection, Form(form): Form<CostCentreFormInput>) -> Result<impl IntoResponse, AppError> {
    DBCostCentre::insert(&form.name, &mut conn).await?;
    Ok(Redirect::to("/cost_centres"))
}

pub(crate) async fn update(
    request_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Path(cost_centre_id): Path<i64>,
    Form(cost_centre_form): Form<CostCentreFormInput>,
) -> Result<impl IntoResponse, AppError> {
    DBCostCentre::update(cost_centre_id, &cost_centre_form.name, &mut conn).await?;
    let mut response_headers = HeaderMap::new();
    if request_headers.contains_key("HX-Request") {
        response_headers.insert("HX-Location", "/cost_centres".parse().unwrap());
    } else {
        response_headers.insert("Location", "/cost_centres".parse().unwrap());
    }
    Ok(response_headers)
}

pub(crate) async fn cost_centre_delete(DatabaseConnection(mut conn): DatabaseConnection, Path(cost_centre_id): Path<i64>) -> Result<Redirect, AppError> {
    DBCostCentre::delete(cost_centre_id, &mut conn).await?;

    Ok(Redirect::to("/cost_centres"))
}
