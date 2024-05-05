use askama::Template;
use axum_core::response::IntoResponse;
use crate::{AppError, HtmlTemplate};
use crate::db::{CostCentreWithSum, DatabaseConnection, DBCostCentre};

#[derive(Template)]
#[template(path = "summary/overview.html")]
struct SummaryOverview {
    sums: Vec<CostCentreWithSum>,
}

pub(crate) async fn summary_overview(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let sums = DBCostCentre::get_summary(&mut conn).await?;
    Ok(HtmlTemplate(SummaryOverview { sums }))

}
