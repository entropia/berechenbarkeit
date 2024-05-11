use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use time::PrimitiveDateTime;

pub(crate) mod vendors;

#[derive(Parser)]
struct Cli {
    /// Path to PDF file
    path: PathBuf,
}

pub fn parse_pdf(pdf: &[u8], vendor: InvoiceVendor) -> anyhow::Result<Invoice> {
    let text = pdf_extract::extract_text_from_mem(pdf)?;
    let vendor_config = match vendor {
        InvoiceVendor::Metro => vendors::metro::METRO,
        InvoiceVendor::Bauhaus => vendors::bauhaus::BAUHAUS,
    };
    return Ok(vendors::vendor::extract_invoice_data(&text, vendor_config)?);
}

#[derive(Debug, Clone, Serialize)]
pub enum InvoiceVendor {
    Metro,
    Bauhaus,
    Unknown
}

impl ToString for InvoiceVendor {
    fn to_string(&self) -> String {
        match self {
            Self::Metro => "metro".to_string(),
            Self::Bauhaus => "bauhaus".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Invoice {
    pub vendor: InvoiceVendor,
    pub meta: InvoiceMeta,
    pub items: Vec<InvoiceItem>,
}


#[derive(Debug, Clone, Serialize)]
pub enum InvoiceItemType {
    Expense,
    Credit
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoiceItem {
    pub typ: InvoiceItemType,
    pub pos: Option<u32>,
    pub article_number: String,
    pub description: String,
    pub net_price_single: f64,
    pub vat: f64,
    pub amount: f64,
    pub net_total_price: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoiceMeta {
    pub invoice_number: String,
    pub sum: f64,
    pub payment_type: Option<String>,
    pub date: PrimitiveDateTime,
}
