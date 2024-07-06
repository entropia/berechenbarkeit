use askama::Template;
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::response::Redirect;
use axum::Form;
use axum_core::response::IntoResponse;
use serde::Deserialize;
use time::{macros::format_description, PrimitiveDateTime};

use crate::db::{
    projects::DBProject,
    util::{DatabaseConnection, DbDate},
};
use crate::utils::make_htmx_redirect;
use crate::{AppError, HtmlTemplate};

#[derive(Deserialize, Debug)]
pub(crate) struct ProjectForm {
    name: String,
    description: String,
    active: Option<String>,
    default: Option<String>,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ProjectIdForm {
    id: i64,
}

#[derive(Template)]
#[template(path = "projects/list.html")]
struct ProjectListTemplate {
    projects: Vec<DBProject>,
}

#[derive(Template)]
#[template(path = "projects/edit.html")]
struct ProjectEditTemplate {
    project: DBProject,
}

#[derive(Template)]
#[template(path = "projects/new.html")]
struct ProjectNewTemplate {}

impl From<ProjectForm> for DBProject {
    fn from(e: ProjectForm) -> Self {
        let format = format_description!("[year]-[month]-[day]T[hour]:[minute]");
        let parse_date_into_option = |i: Option<String>| -> Option<PrimitiveDateTime> { i.as_ref().map(|d| PrimitiveDateTime::parse(d, &format).ok()).flatten() };
        let html_checkbox_to_boolean = |c: Option<String>| -> bool { c.is_some() && c.unwrap().as_str() == "true" };
        DBProject {
            id: None,
            name: e.name,
            description: e.description,
            active: html_checkbox_to_boolean(e.active),
            default: html_checkbox_to_boolean(e.default),
            start: DbDate {
                datetime: parse_date_into_option(e.start),
            },
            end: DbDate {
                datetime: parse_date_into_option(e.end),
            },
        }
    }
}

pub(crate) async fn list(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let projects = DBProject::get_ordered_by_id(&mut conn).await?;
    Ok(HtmlTemplate(ProjectListTemplate { projects }))
}

pub(crate) async fn new_project_page() -> Result<impl IntoResponse, AppError> {
    Ok(HtmlTemplate(ProjectNewTemplate {}))
}

pub(crate) async fn edit_project_page(DatabaseConnection(mut conn): DatabaseConnection, Path(project_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let project = DBProject::get_by_id(project_id, &mut conn).await?;
    Ok(HtmlTemplate(ProjectEditTemplate { project }))
}

pub(crate) async fn add(DatabaseConnection(mut conn): DatabaseConnection, Form(project_form): Form<ProjectForm>) -> Result<impl IntoResponse, AppError> {
    DBProject::add(DBProject::from(project_form), &mut conn).await?;
    Ok(Redirect::to("/projects"))
}

pub(crate) async fn update(
    req_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Path(project_id): Path<i64>,
    Form(project_form): Form<ProjectForm>,
) -> Result<impl IntoResponse, AppError> {
    DBProject::update(
        DBProject {
            id: Some(project_id),
            ..DBProject::from(project_form)
        },
        &mut conn,
    )
    .await?;
    make_htmx_redirect(req_headers, "/projects")
}

pub(crate) async fn delete(req_headers: HeaderMap, DatabaseConnection(mut conn): DatabaseConnection, Path(project_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    DBProject::delete(project_id, &mut conn).await?;
    make_htmx_redirect(req_headers, "/projects")
}

pub(crate) async fn set_default(
    req_headers: HeaderMap,
    DatabaseConnection(mut conn): DatabaseConnection,
    Form(project): Form<ProjectIdForm>,
) -> Result<impl IntoResponse, AppError> {
    DBProject::set_default(project.id, &mut conn).await?;
    make_htmx_redirect(req_headers, "/projects")
}

pub(crate) async fn clear_default(req_headers: HeaderMap, DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    DBProject::clear_default(&mut conn).await?;
    make_htmx_redirect(req_headers, "/projects")
}
