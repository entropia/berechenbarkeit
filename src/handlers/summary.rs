use askama::Template;
use axum_core::response::IntoResponse;
use crate::{AppError, HtmlTemplate};
use crate::db::{
    cost_centres::{CostCentreWithSum, DBCostCentre},
    util::DatabaseConnection,
    invoices::DBInvoiceItem
};
use http::header;

#[derive(Template)]
#[template(path = "summary/overview.html")]
struct SummaryOverview {
    sums: Vec<CostCentreWithSum>,
}

pub(crate) async fn summary_overview(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let sums = DBCostCentre::get_summary(&mut conn).await?;
    Ok(HtmlTemplate(SummaryOverview { sums }))
}

pub(crate) async fn summary_csv_aggregated(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let sums = DBCostCentre::get_summary(&mut conn).await?;

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["kostenstelle", "mwst_satz", "summe_netto", "summe_mwst_befreit"])?;

    for record in sums {
        wtr.write_record([record.cost_centre_name, record.vat.to_string(), record.sum_net.to_string(), record.sum_vat_exempted.to_string()])?;
    }

    let csv_string = String::from_utf8(wtr.into_inner()?)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"berechenbarkeit-aggregated.csv\"",
            ),
        ],
        csv_string,
    ))
}


pub(crate) async fn summary_csv_raw(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let items = DBInvoiceItem::get_all(&mut conn).await?;

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["haendler", "rechnungsdatum", "rechnungsnummer", "typ", "beschreibung", "menge", "einzelpreis_netto", "gesamtpreis_netto", "mwst_satz", "mwst_befreit", "kostenstelle"])?;

    for record in items {
        wtr.write_record([record.invoice_vendor, record.invoice_date.to_string(), record.invoice_number, record.typ, record.description, record.amount.to_string(), record.net_price_single.to_string(), (record.net_price_single * record.amount).to_string(), record.vat.to_string(), match record.vat_exempt { true => "true".to_string(), false => "false".to_string()}, record.cost_centre.unwrap_or_else(|| "".to_string())])?;
    }

    let csv_string = String::from_utf8(wtr.into_inner()?)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"berechenbarkeit-raw.csv\"",
            ),
        ],
        csv_string,
    ))
}
