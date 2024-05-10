use std::{
    num::{ParseFloatError, ParseIntError},
};

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;
use time::{error::ComponentRange, Date, PrimitiveDateTime, Time};
use crate::{Invoice, InvoiceItem, InvoiceItemType, InvoiceMeta, InvoiceVendor};

static INVOICE_ITEM: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<MM>.) (?P<ARTNR>\d{6}\.\d) (?P<EAN>[\d ]{14}) (?P<BEZEICHNUNG>.{31}) (?P<PACK>.{2}) (?P<EINZELPREIS>.{11}) (?P<INHALTKOLLI>.{10}) (?P<KOLLIPREIS>.{10}) (?P<MENGE>.{6}) (?P<GESAMTPREIS>.{11}) (?P<M>.) (?P<STUECKPREIS>.{10})[  ](?P<INT>.) (?P<KD>.+)?$").unwrap()
});
static INVOICE_ITEM_CREDIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^ {26}(?P<BEZEICHNUNG>.{50}) *(?P<GESAMTPREIS>.{11}) (?P<M>.)?[ 0-9]{12}$").unwrap()
});
static INVOICE_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"RECHNUNGS?-? ?NR\.?\:?\s+(?P<INVOICE_NUMBER>[\.\d\/]+)").unwrap());
static CUSTOMER_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"KUNDE.\s+(?P<CUSTOMER_NUMBER>\d[\d ]*\d)").unwrap());
static SUM_AND_PAYMENT_TYPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"SUMME EUR\s+(?P<SUM>[\d\.,\-]+)[\s\-]+(?P<PAYMENT_TYPE>[a-zA-Z0-9:\-\., ]+) +[\d\.,\-]+",
    )
    .unwrap()
});
static INVOICE_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"RECHNUNGSDATUM:\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4}) (?P<hour>\d\d):(?P<minute>\d\d)").unwrap()
});

static EXTRA_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

pub(crate) fn invoice(text: &str) -> Result<Invoice, MetroInvoiceError> {
    let invoice_number = find(&INVOICE_NUMBER, "INVOICE_NUMBER", text)?;
    let customer_number = find(&CUSTOMER_NUMBER, "CUSTOMER_NUMBER", text)?;
    let sum = parse_metro_number(&find(&SUM_AND_PAYMENT_TYPE, "SUM", text)?)?;
    let payment_type = find(&SUM_AND_PAYMENT_TYPE, "PAYMENT_TYPE", text)?;
    let date_caps = INVOICE_DATE
        .captures(text)
        .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE"))?;
    let (year, month, day, hour, minute): (i32, u8, u8, u8, u8) = (
        date_caps
            .name("year")
            .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE.year"))?
            .as_str()
            .parse()?,
        date_caps
            .name("month")
            .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE.month"))?
            .as_str()
            .parse()?,
        date_caps
            .name("day")
            .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE.day"))?
            .as_str()
            .parse()?,
        date_caps
            .name("hour")
            .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE.hour"))?
            .as_str()
            .parse()?,
        date_caps
            .name("minute")
            .ok_or(MetroInvoiceError::MissingField("INVOICE_DATE.minute"))?
            .as_str()
            .parse()?,
    );
    let date = PrimitiveDateTime::new(
        Date::from_calendar_date(year, month.try_into()?, day)?,
        Time::from_hms(hour, minute, 0)?,
    );
    let payment_type = EXTRA_SPACES.replace_all(&payment_type, " ").to_string();
    let meta = InvoiceMeta {
        invoice_number,
        sum,
        payment_type: Some(payment_type),
        date,
    };
    let mut items = text
        .lines()
        .filter(|line| INVOICE_ITEM.is_match(line))
        .map(|line| {
            let Some(caps) = INVOICE_ITEM.captures(line) else {
                panic!("line was marked as a match before")
            };
            RawMetroInvoiceItem {
                typ: InvoiceItemType::Expense,
                _mm: caps["MM"].to_string(),
                artnr: caps["ARTNR"].to_string(),
                _ean: caps["EAN"].to_string(),
                bezeichnung: caps["BEZEICHNUNG"].to_string(),
                _pack: caps["PACK"].to_string(),
                einzelpreis: caps["EINZELPREIS"].to_string(),
                inhaltkolli: caps["INHALTKOLLI"].to_string(),
                _kollipreis: caps["KOLLIPREIS"].to_string(),
                menge: caps["MENGE"].to_string(),
                gesamtpreis: caps["GESAMTPREIS"].to_string(),
                m: caps["M"].to_string(),
                _stueckpreis: caps["STUECKPREIS"].to_string(),
                _int: caps["INT"].to_string(),
                _kd: caps.name("KD").map(|kd| kd.as_str().to_string()),
            }
        })
        .map(TryInto::try_into)
        .collect::<Result<Vec<InvoiceItem>, MetroInvoiceError>>()?;

    let mut items_credit = text.lines().filter(|line| INVOICE_ITEM_CREDIT.is_match(line)).map(|line| {
        let Some(caps) = INVOICE_ITEM_CREDIT.captures(line) else {
            panic!("line was marked as match before")
        };
        RawMetroInvoiceItem {
            typ: InvoiceItemType::Credit,
            _mm: "".to_string(),
            artnr: "".to_string(),
            _ean: "".to_string(),
            bezeichnung: caps["BEZEICHNUNG"].to_string(),
            _pack: "".to_string(),
            einzelpreis: caps["GESAMTPREIS"].to_string(),
            inhaltkolli: "1".to_string(),
            _kollipreis: "".to_string(),
            menge: "1".to_string(),
            gesamtpreis: caps["GESAMTPREIS"].to_string(),
            m: caps["M"].to_string(),
            _stueckpreis: "".to_string(),
            _int: "".to_string(),
            _kd: None,
        }
    }).map(TryInto::try_into)
        .collect::<Result<Vec<InvoiceItem>, MetroInvoiceError>>()?;

    items.append(&mut items_credit);
    
    for (i, el) in items.iter_mut().enumerate() {
        el.pos = i as u32;
    }
    Ok(Invoice { vendor: InvoiceVendor::Metro, meta, items })
}

fn find(re: &Regex, name: &'static str, text: &str) -> Result<String, MetroInvoiceError> {
    Ok(re
        .captures(text)
        .and_then(|caps| caps.name(name))
        .ok_or(MetroInvoiceError::MissingField(name))?
        .as_str()
        .trim()
        .to_string())
}

#[derive(Debug)]
pub(crate) struct RawMetroInvoiceItem {
    typ: InvoiceItemType,
    _mm: String,
    artnr: String,
    _ean: String,
    bezeichnung: String,
    _pack: String,
    einzelpreis: String,
    inhaltkolli: String,
    _kollipreis: String,
    menge: String,
    gesamtpreis: String,
    m: String,
    _stueckpreis: String,
    _int: String,
    _kd: Option<String>,
}


impl TryFrom<RawMetroInvoiceItem> for InvoiceItem {
    type Error = MetroInvoiceError;
    fn try_from(value: RawMetroInvoiceItem) -> Result<Self, MetroInvoiceError> {
        Ok(InvoiceItem {
            typ: value.typ,
            pos: 0,
            article_number: value.artnr.trim().to_string(),
            description: value.bezeichnung.trim().to_string(),
            net_price_single: parse_metro_number(&value.einzelpreis)?,
            net_total_price: parse_metro_number(&value.gesamtpreis)?,
            vat: match value.m.as_str() {
                "A" => 0.19f64,
                "B" => 0.07f64,
                unknown_vat_class => {
                    return Err(MetroInvoiceError::UnknownVatClass(
                        unknown_vat_class.to_string(),
                    ))
                }
            },
            amount: parse_metro_number(&value.menge)? * parse_metro_number(&value.inhaltkolli)?,
        })
    }
}

#[derive(Debug, Error, PartialEq)]
pub(crate) enum MetroInvoiceError {
    #[error("couldn't parse number on metro invoice: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("couldn't parse number on metro invoice: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("broken date: {0}")]
    DateError(#[from] ComponentRange),
    #[error("unknown vat class: {0}")]
    UnknownVatClass(String),
    #[error("couldn't find field {0} on invoice")]
    MissingField(&'static str),
}

pub(crate) fn parse_metro_number(text: &str) -> Result<f64, MetroInvoiceError> {
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
    fn parse_metro_number() {
        assert_eq!(Ok(-10f64), super::parse_metro_number("10,000-"));
        assert_eq!(Ok(1312f64), super::parse_metro_number("1.312,00"));
    }
}
