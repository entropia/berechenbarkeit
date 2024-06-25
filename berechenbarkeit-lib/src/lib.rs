use std::{
    path::PathBuf,
    num::{ParseFloatError, ParseIntError},
};

use clap::Parser;
use serde::Serialize;
use thiserror::Error;
use once_cell::sync::Lazy;
use time::{PrimitiveDateTime, error::ComponentRange};
use vendors::regex::{
    METRO, BAUHAUS, IKEA, MEDICALCORNER, MOLTONDISCOUNT,
};

pub mod vendors;

#[derive(Debug, Error, PartialEq)]
pub enum InvoiceParseError {
    #[error("Failed to parse floating point number on invoice: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("Failed to parse integer number on invoice: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Unparseable date: {0}")]
    DateError(#[from] ComponentRange),
    #[error("Required field {0} not found on invoice")]
    FieldMissingError(String),
    #[error("Unrecognized VAT class '{0}'")]
    UnrecognizedVatClass(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum InvoiceVendor {
    Metro,
    Bauhaus,
    Ikea,
    MedicalCorner,
    MoltonDiscount,
}

#[derive(Debug, Clone, Serialize)]
pub enum InvoiceItemType {
    Expense,
    Credit
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoiceItem {
    pub typ: InvoiceItemType,
    pub pos: u32,
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
    pub sum_gross: f64,
    pub payment_type: Option<String>,
    pub date: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct Invoice {
    pub vendor: InvoiceVendor,
    pub meta: InvoiceMeta,
    pub items: Vec<InvoiceItem>,
}


impl ToString for InvoiceVendor {
    fn to_string(&self) -> String {
        match self {
            Self::Metro => "metro".to_string(),
            Self::Bauhaus => "bauhaus".to_string(),
            Self::Ikea => "ikea".to_string(),
            Self::MedicalCorner => "medicalcorner".to_string(),
            Self::MoltonDiscount => "moltondiscount".to_string(),
        }
    }
}

pub enum InvoiceParser {
    Regex(&'static Lazy<vendors::regex::RegexVendor>),
}

pub trait Vendor {
    fn extract_invoice_data(&self, pdf: &[u8], vendor: InvoiceVendor) -> anyhow::Result<Invoice>;
}
#[derive(Parser)]
struct Cli {
    /// Path to PDF file
    path: PathBuf,
}

pub fn get_parser_for_vendor(vendor: Option<InvoiceVendor>) -> Option<InvoiceParser> {
    return match vendor {
        Some(InvoiceVendor::Metro) => Some(InvoiceParser::Regex(&METRO)),
        Some(InvoiceVendor::Bauhaus) => Some(InvoiceParser::Regex(&BAUHAUS)),
        Some(InvoiceVendor::Ikea) => Some(InvoiceParser::Regex(&IKEA)),
        Some(InvoiceVendor::MedicalCorner) => Some(InvoiceParser::Regex(&MEDICALCORNER)),
        Some(InvoiceVendor::MoltonDiscount) => Some(InvoiceParser::Regex(&MOLTONDISCOUNT)),
        None => None
    };
}
