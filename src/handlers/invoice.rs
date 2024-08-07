use crate::db::{
    cost_centres::DBCostCentre,
    invoices::{DBInvoice, DBInvoiceItem},
    projects::DBProject,
    util::DatabaseConnection,
};
use crate::{AppError, HtmlTemplate};
use askama::Template;
use axum::body::Bytes;
use axum::extract::{Path, RawForm};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use axum_typed_multipart::{TryFromMultipart, TypedMultipart};
use berechenbarkeit_lib::{get_parser_for_vendor, InvoiceItemType, InvoiceParser, InvoiceVendor, Vendor};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(TryFromMultipart, Debug)]
pub(crate) struct InvoiceUploadRequest {
    vendor: String,
    file: Bytes,
}

pub(crate) async fn invoice_add_upload(DatabaseConnection(mut conn): DatabaseConnection, TypedMultipart(data): TypedMultipart<InvoiceUploadRequest>) -> Result<Redirect, AppError> {
    let vendor: Option<InvoiceVendor> = TryInto::<InvoiceVendor>::try_into(data.vendor).ok();
    let parser: Option<InvoiceParser> = get_parser_for_vendor(vendor.clone());

    let file = data.file;

    let parsed_invoice = match parser.unwrap() {
        InvoiceParser::Regex(p) => p.extract_invoice_data(&file, vendor.unwrap())?,
    };
    let invoice_id = DBInvoice::insert(parsed_invoice.clone().into(), &mut conn).await?;

    DBInvoiceItem::bulk_insert(
        &mut conn,
        (parsed_invoice.items)
            .into_iter()
            .map(|i| DBInvoiceItem {
                id: None,
                invoice_id,
                position: i.pos as i64,
                typ: match i.typ {
                    InvoiceItemType::Credit => "Credit".to_string(),
                    InvoiceItemType::Expense => "Expense".to_string(),
                },
                description: i.description.clone(),
                amount: i.amount,
                net_price_single: i.net_price_single,
                vat: i.vat,
                vat_exempt: false,
                cost_centre_id: None,
                cost_centre: None,
                project_id: None,
            })
            .collect(),
    )
    .await?;

    let file_storage_base_path = std::env::var("BERECHENBARKEIT_STORAGE_BASE_PATH");
    if file_storage_base_path.is_ok() {
        let path = format!("{}/invoice-{}.pdf", file_storage_base_path.unwrap(), invoice_id);
        let mut fileio = File::create(path)?;
        fileio.write_all(&file)?;
    }

    Ok(Redirect::to(&format!("/invoice/{}/edit", invoice_id)))
}

#[derive(Template)]
#[template(path = "invoice/edit.html")]
struct InvoiceEditTemplate {
    invoice: DBInvoice,
    invoice_items: Vec<DBInvoiceItem>,
    cost_centres: Vec<DBCostCentre>,
    projects: Vec<DBProject>,
    diff_invoice_item_sum: f64,
}

pub(crate) async fn download(Path(invoice_id): Path<i64>) -> Result<(StatusCode, HeaderMap, Vec<u8>), impl IntoResponse> {
    let file_storage_base_path = std::env::var("BERECHENBARKEIT_STORAGE_BASE_PATH");
    let mut pdf_content: Vec<u8> = vec![];
    if file_storage_base_path.is_ok() {
        let filepath = format!("{}/invoice-{}.pdf", file_storage_base_path.unwrap(), invoice_id);
        if File::open(filepath).and_then(|mut f| f.read_to_end(&mut pdf_content)).is_err() {
            return Err((StatusCode::NOT_FOUND, "No invoice could be found"));
        }
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert("Content-Disposition", format!("attachment; filename=\"{}.pdf\"", invoice_id).parse().unwrap());
    response_headers.insert("Content-Length", pdf_content.len().to_string().parse().unwrap());
    response_headers.insert("Content-Type", "application/pdf".parse().unwrap());
    Ok((StatusCode::OK, response_headers, pdf_content))
}

pub(crate) async fn invoice_edit(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let invoice = DBInvoice::get_by_id(invoice_id, &mut conn).await?;
    let invoice_items = DBInvoiceItem::get_by_invoice_id(invoice_id, &mut conn).await?;
    let cost_centres = DBCostCentre::get_all(&mut conn).await?;
    let projects = DBProject::get(&mut conn).await?;
    let diff_invoice_item_sum = f64::round((invoice.sum_gross - DBInvoiceItem::calculate_sum_gross_by_invoice_id(invoice_id, &mut conn).await?) * 1000f64) / 1000f64;
    let used_project_ids: Vec<_> = invoice_items.clone().into_iter().map(|invoice_item| invoice_item.project_id).collect();

    Ok(HtmlTemplate(InvoiceEditTemplate {
        invoice,
        invoice_items,
        cost_centres,
        projects: projects.into_iter().filter(|p| p.active || used_project_ids.contains(&p.id)).collect(),
        diff_invoice_item_sum,
    }))
}

#[derive(Template)]
#[template(path = "invoice/delete_confirm.html")]
struct InvoiceDeleteConfirmTemplate {
    invoice: DBInvoice,
}
pub(crate) async fn invoice_delete_confirm(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let invoice = DBInvoice::get_by_id(invoice_id, &mut conn).await?;

    Ok(HtmlTemplate(InvoiceDeleteConfirmTemplate { invoice }))
}

pub(crate) async fn invoice_delete(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    DBInvoice::delete(invoice_id, &mut conn).await?;

    Ok(Redirect::to("/invoices"))
}

#[derive(Serialize)]
struct InvoiceItemSplitResponse {
    new_id: i64,
}

pub(crate) async fn invoice_item_split(
    DatabaseConnection(mut conn): DatabaseConnection,
    Path((_invoice_id, invoiceitem_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let invoice_item = DBInvoiceItem::get_by_id(invoiceitem_id, &mut conn).await?;
    let new_id = DBInvoiceItem::insert(
        DBInvoiceItem {
            id: None,
            position: invoice_item.position,
            invoice_id: invoice_item.invoice_id,
            typ: invoice_item.typ,
            description: invoice_item.description,
            amount: 0f64,
            net_price_single: invoice_item.net_price_single,
            vat: invoice_item.vat,
            vat_exempt: false,
            cost_centre_id: None,
            cost_centre: None,
            project_id: None,
        },
        &mut conn,
    )
    .await?;
    Ok(Json(InvoiceItemSplitResponse { new_id }))
}

pub(crate) async fn invoice_edit_submit(DatabaseConnection(mut conn): DatabaseConnection, Path(invoice_id): Path<i64>, RawForm(form): RawForm) -> Result<Redirect, AppError> {
    let form_data = serde_html_form::from_bytes::<Vec<(String, String)>>(&form)?;

    let mut vat_exempt_items_changed = HashSet::new();

    for form_field in form_data.into_iter() {
        // TODO: Use bulk UPDATE
        let (invoiceitem_id, data_type) = form_field.0.split_once('-').unwrap();
        let invoiceitem_id = invoiceitem_id.parse()?;
        if data_type == "amount" {
            DBInvoiceItem::update_amount(invoiceitem_id, f64::from_str(&form_field.1)?, &mut conn).await?;
        } else if data_type == "costcentre" {
            let mut cost_centre_id = None;
            if !form_field.1.is_empty() {
                cost_centre_id = Some(form_field.1.parse()?);
            }
            DBInvoiceItem::update_cost_centre(invoiceitem_id, cost_centre_id, &mut conn).await?;
        } else if data_type == "vatexempt" {
            if form_field.1 != "on" {
                continue;
            }
            vat_exempt_items_changed.insert(invoiceitem_id);
            DBInvoiceItem::update_vat_exemption(invoiceitem_id, true, &mut conn).await?;
        } else if data_type == "project" {
            if !form_field.1.is_empty() {
                let project_id: i64 = form_field.1.parse()?;
                DBInvoiceItem::update_project(invoiceitem_id, project_id, &mut conn).await?;
            }
        }
    }

    // As html <input type="checkbox"> only send the value if they're checked, we need to set all other values to null.
    for ii in DBInvoiceItem::get_by_invoice_id(invoice_id, &mut conn).await? {
        if !vat_exempt_items_changed.contains(&ii.id.unwrap()) {
            DBInvoiceItem::update_vat_exemption(ii.id.unwrap(), false, &mut conn).await?
        }
    }

    Ok(Redirect::to(&format!("/invoice/{}/edit", invoice_id)))
}

#[derive(Template)]
#[template(path = "invoice/list.html")]
struct InvoiceListTemplate {
    invoices: Vec<DBInvoice>,
}

pub(crate) async fn invoice_list(DatabaseConnection(mut conn): DatabaseConnection) -> Result<impl IntoResponse, AppError> {
    let invoices = DBInvoice::get_all(&mut conn).await?;
    Ok(HtmlTemplate(InvoiceListTemplate { invoices }))
}
