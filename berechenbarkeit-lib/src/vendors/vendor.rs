use std::{
    num::{ParseFloatError, ParseIntError},
};

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;
use time::{error::ComponentRange, Date, PrimitiveDateTime, Time};
use crate::{Invoice, InvoiceItem, InvoiceItemType, InvoiceMeta, InvoiceVendor};

#[derive(Debug, Error, PartialEq)]
pub(crate) enum InvoiceParseError {
    #[error("Could not parse floating point number on invoice: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("Could not parse integer number on invoice: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Unparseable Date on invoice: {0}")]
    DateFormatError(#[from] ComponentRange),
    #[error("Unrecognized VAT class: {0}")]
    UnrecognizedVatClass(String),
    #[error("Missing required field {0} on invoice")]
    FieldMissingError(String),
}

#[derive(Debug)]
pub(crate) struct Vendor {
    vendor_id: InvoiceVendor,
    customer_number: Option<String>,
    invoice_number: String,
    invoice_date: String,
    invoice_total: String,
    invoice_item: String,
    invoice_credit_item: Option<String>,
    vat_map: VatMap,
}

#[derive(Debug)]
pub(crate) struct VatMap {
    entries: Vec<(String, f64)>
}

impl VatMap {
    pub fn new(entries: Vec<(String, f64)>) -> VatMap {
        VatMap {
            entries,
        }
    }

    pub fn get(&self, key: String) -> Result<f64, InvoiceParseError> {
        match self.entries.into_iter().find(|(k, _v)| k == &key) {
            Some((_k, v)) => Ok(v),
            None => Err(InvoiceParseError::UnrecognizedVatClass(key))
        }
    }
}

pub fn extract_invoice_data(invoice_text: &str, vendor: Vendor) -> Result<Invoice, InvoiceParseError> {
    return Invoice {
        vendor: vendor.vendor_id,
        meta: extract_invoice_meta(invoice_text, vendor)?,
        items: extract_items(invoice_text, vendor)?,

    }


fn extract_invoice_meta(invoice_text: str, vendor: Vendor) -> Result<InvoiceMeta, InvoiceParseError> {
    let invoice_number = find(&vendor.invoice_number, "INVOICE_NUMBER", invoice_text)?;
    let invoice_date = extract_invoice_date(invoice_text, vendor);
    let invoice_total = parse_number::<f64>(find(&vendor.invoice_total, "SUM", invoice_text)?)?;
    let payment_type: Option<String> = vendor.payment_type.map(|pt| find(&pt, "PAYMENT_TYPE", invoice_text)?);
    return InvoiceMeta {
        invoice_number,
        sum: invoice_total,
        payment_type,
        date: invoice_date,
    }
}

fn extract_invoice_date(invoice_text: str, vendor: Vendor) -> Result<PrimitiveDateTime, InvoiceParseError> {
    let datetime_regex: Lazy<Regex> = Lazy::new(|| Regex::new(vendor.invoice_date).unwrap());
    let matches = datetime_regex.captures(invoice_text).ok_or(InvoiceParseError::FieldMissingError("DATE"))?;
    let (year, month, day): (i32, u8, u8) = (
        matches.name("year").ok_or(InvoiceParseError::FieldMissingError("DATE.year"))?.as_str().parse()?,
        matches.name("month").ok_or(InvoiceParseError::FieldMissingError("DATE.month"))?.as_str().parse()?,
        matches.name("day").ok_or(InvoiceParseError::FieldMissingError("DATE.day"))?.as_str().parse()?,
    );
    let (hour, minute, second): (u8, u8, u8) = (
        matches.name("hour").unwrap_or("0").as_str().parse()?,
        matches.name("minute").unwrap_or("0").as_str().parse()?,
        matches.name("second").unwrap_or("0").as_str().parse()?,
    );
    return PrimitiveDateTime::new(
        Date::from_calendar_date(year, month.try_into()?, day)?,
        Time::from_hms(hour, minute, second)?,
    );
}

fn extract_items(invoice_text: str, vendor: Vendor) -> Result<Vec<InvoiceItem>, InvoiceParseError> {
    let item_regex: Lazy<Regex> = Lazy::new(|| Regex::new(vendor.invoice_item).unwrap());
    //TODO add handling for credit items
    invoice_text
        .lines()
        .filter(|line| item_regex.is_match(line))
        .map(|line| {
            item_regex.captures(line).and_then(|item| InvoiceItem {
                typ: InvoiceItemType::Expense,
                pos: item.name("POS")?.parse::<u32>().unwrap(),
                article_number: item.name("ARTNR").to_string(),
                description: item.name("DESC").to_string(),
                net_price_single: item.name("NET_PRICE_SINGLE").unwrap_or({
                    let vat = vat_from_field(item.name("VAT").unwrap());
                    parse_number::<f64>(item.name("GROSS_PRICE_SINGLE").unwrap()) * (1 - (vat / (1 + vat)))?
                }),
                vat: vat_from_field(item.name("VAT").unwrap()),
                amount: parse_number::<f64>(item.name("AMOUNT")),
                net_total_price: parse_number::<f64>(item.name("ITEM_NET_TOTAL").unwrap_or({
                    let vat = vat_from_field(item.name("VAT").unwrap());
                    parse_number::<f64>(item.name("ITEM_TOTAL_GROSS")).unwrap() * (1 - (vat / (1 + vat)))?
                }))
            })
        })
    .collect::<Result<Vec<InvoiceItem>>, InvoiceParseError>()?

}

fn vat_from_field(value: &str, vendor: Vendor) -> f64 {
    vendor.vat_map.get(value.to_string())?;
}

fn find(re: &Regex, name: &'static str, text: &str) -> Result<String, InvoiceParseError> {
    Ok(re
        .captures(text)
        .and_then(|caps| caps.name(name))
        .ok_or(InvoiceParseError::MissingField(name))?
        .as_str()
        .trim()
        .to_string())
}


pub(crate) fn parse_number<T: Into<f64>>(num: &str) -> Result<T, InvoiceParseError> {
    let raw = num.replace(".", "").replacen(" ", "", 1000);
    let is_negative = raw.contains("-");
    let value = raw.replace("-", "").replace(",", ".");
    if is_negative {
        Ok(value.parse::<T>()? * -1)
    } else {
        Ok(value.parse::<T>()?)
    }
} 
