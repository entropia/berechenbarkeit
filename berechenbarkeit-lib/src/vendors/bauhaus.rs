use std::{
    num::{ParseFloatError, ParseIntError},
};

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;
use time::{error::ComponentRange, Date, PrimitiveDateTime, Time};
use crate::{calculate_net_for_gross, Invoice, InvoiceItem, InvoiceItemType, InvoiceMeta, InvoiceVendor};

static INVOICE_ITEM: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<POS>\d)\s+(?P<ARTNR>\d{8})\s+(?P<BEZEICHNUNG>.{1,100})\s+(?P<MENGE>\d{1,6}) (ST|KAR)\s+(?P<EINZELPREIS>.{1,7})\s+(?P<GESAMTPREIS>.{1,7})\s+(?P<MWST>\w)$").unwrap()
});
static INVOICE_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Einzelrechnung\s+Nr\.\s+(?P<INVOICE_NUMBER>[\.\d\/]+)").unwrap());
static CUSTOMER_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Kundennummer\s+(?P<CUSTOMER_NUMBER>\d[\d ]*\d)").unwrap());
static SUM_TYPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"Zu zahlender Betrag\s+(?P<SUM>[\d\.,\-]+)\ EUR",
    )
    .unwrap()
});
static INVOICE_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Rechnungsdatum\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4})").unwrap()
});

pub(crate) fn invoice(text: &str) -> Result<Invoice, BauhausInvoiceError> {
    let invoice_number = find(&INVOICE_NUMBER, "INVOICE_NUMBER", text)?;
    let _customer_number = find(&CUSTOMER_NUMBER, "CUSTOMER_NUMBER", text)?;
    let sum_gross = parse_bauhaus_number(&find(&SUM_TYPE, "SUM", text)?)?;
    let date_caps = INVOICE_DATE
        .captures(text)
        .ok_or(BauhausInvoiceError::MissingField("INVOICE_DATE"))?;
    let (year, month, day): (i32, u8, u8) = (
        date_caps
            .name("year")
            .ok_or(BauhausInvoiceError::MissingField("INVOICE_DATE.year"))?
            .as_str()
            .parse()?,
        date_caps
            .name("month")
            .ok_or(BauhausInvoiceError::MissingField("INVOICE_DATE.month"))?
            .as_str()
            .parse()?,
        date_caps
            .name("day")
            .ok_or(BauhausInvoiceError::MissingField("INVOICE_DATE.day"))?
            .as_str()
            .parse()?,
    );
    let date = PrimitiveDateTime::new(
        Date::from_calendar_date(year, month.try_into()?, day)?,
        Time::from_hms(0, 0, 0)?,
    );
    let meta = InvoiceMeta {
        invoice_number,
        sum_gross,
        payment_type: None,
        date
    };
    let items = text
        .lines()
        .filter(|line| INVOICE_ITEM.is_match(line))
        .map(|line| {
            let Some(caps) = INVOICE_ITEM.captures(line) else {
                panic!("line was marked as a match before")
            };
            RawBauhausInvoiceItem {
                typ: InvoiceItemType::Expense,
                pos: caps["POS"].parse::<u32>().unwrap(),
                artnr: caps["ARTNR"].to_string(),
                bezeichnung: caps["BEZEICHNUNG"].to_string(),
                menge: caps["MENGE"].to_string(),
                einzelpreis: caps["EINZELPREIS"].to_string(),
                gesamtpreis: caps["GESAMTPREIS"].to_string(),
                m: caps["MWST"].to_string(),
            }
        })
        .map(TryInto::try_into)
        .collect::<Result<Vec<InvoiceItem>, BauhausInvoiceError>>()?;

    Ok(Invoice { vendor: InvoiceVendor::Bauhaus, meta, items })
}

fn find(re: &Regex, name: &'static str, text: &str) -> Result<String, BauhausInvoiceError> {
    Ok(re
        .captures(text)
        .and_then(|caps| caps.name(name))
        .ok_or(BauhausInvoiceError::MissingField(name))?
        .as_str()
        .trim()
        .to_string())
}

#[derive(Debug)]
pub(crate) struct RawBauhausInvoiceItem {
    typ: InvoiceItemType,
    pos: u32,
    artnr: String,
    bezeichnung: String,
    menge: String,
    einzelpreis: String,
    gesamtpreis: String,
    m: String,
}


impl TryFrom<RawBauhausInvoiceItem> for InvoiceItem {
    type Error = BauhausInvoiceError;
    fn try_from(value: RawBauhausInvoiceItem) -> Result<Self, BauhausInvoiceError> {
        let vat = match value.m.as_str() {
            "C" => 0.19f64,
            unknown_vat_class => {
                return Err(BauhausInvoiceError::UnknownVatClass(
                    unknown_vat_class.to_string(),
                ))
            }
        };
        Ok(InvoiceItem {
            typ: value.typ,
            pos: value.pos,
            article_number: value.artnr.trim().to_string(),
            description: value.bezeichnung.trim().to_string(),
            net_price_single: calculate_net_for_gross(parse_bauhaus_number(&value.einzelpreis)?, vat),
            net_total_price: calculate_net_for_gross(parse_bauhaus_number(&value.gesamtpreis)?, vat),
            vat,
            amount: parse_bauhaus_number(&value.menge)?,
        })
    }
}

#[derive(Debug, Error, PartialEq)]
pub(crate) enum BauhausInvoiceError {
    #[error("couldn't parse integer number on bauhaus invoice: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("couldn't parse real number on bauhaus invoice: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("broken date: {0}")]
    DateError(#[from] ComponentRange),
    #[error("unknown vat class: {0}")]
    UnknownVatClass(String),
    #[error("couldn't find field {0} on invoice")]
    MissingField(&'static str),
}

fn parse_bauhaus_number(text: &str) -> Result<f64, BauhausInvoiceError> {
    let text = text.replace(".", "").replace(" ", "").replace(" ", "");
    let negative = text.contains("-");
    let text = text.replace("-", "").replace(",", ".");
    let parsed: f64 = text.parse()?;
    if negative {
        Ok(parsed * -1f64)
    } else {
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn parse_bauhaus_number() {
        assert_eq!(Ok(-10f64), super::parse_bauhaus_number("10,000-"));
        assert_eq!(Ok(1312f64), super::parse_bauhaus_number("1.312,00"));
    }
}
