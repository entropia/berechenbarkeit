use time::{
    Date, Time, PrimitiveDateTime
};
use regex::{Captures, Regex, RegexBuilder};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use crate::{
    Invoice, InvoiceMeta, InvoiceItem, InvoiceItemType, InvoiceVendor,
    InvoiceParseError,
    Vendor,
};

pub struct RegexVendor {
    invoice_number_regex: Regex,
    invoice_date_regex: Regex,
    invoice_total_regex: Regex,
    invoice_item_regex: Regex,
    invoice_discount_regex: Option<Regex>,
    vat_classes: HashMap<&'static str, f64>,
    default_vat_class: Option<f64>,
}
#[derive(Default,Debug)]
pub struct ItemRegex {
    re: &'static str,
    multi_line: bool,
    dot_matches_newline: Option<bool>,
}
impl RegexVendor {
    fn new(invoice_number: ItemRegex, invoice_date: ItemRegex, invoice_total: ItemRegex, invoice_item: ItemRegex, invoice_discount_item: Option<ItemRegex>, vat_entries: Vec<(&'static str, f64)>, default_vat_class: Option<f64>) -> RegexVendor {
        let mut vat_map: HashMap<&str, f64> = HashMap::with_capacity(100);
        for (k, v) in vat_entries {
            vat_map.insert(k, v);
        }
        let build_re = |re: &str, multi_line: bool, dot_matches_new_line: bool| -> Regex {
            RegexBuilder::new(re).multi_line(multi_line).dot_matches_new_line(dot_matches_new_line).build().unwrap()
        };
        RegexVendor {
            invoice_number_regex: build_re(invoice_number.re, !invoice_number.multi_line, invoice_number.dot_matches_newline.unwrap_or(invoice_number.multi_line)),
            invoice_total_regex: build_re(invoice_total.re, !invoice_total.multi_line, invoice_total.dot_matches_newline.unwrap_or(invoice_total.multi_line)),
            invoice_date_regex: build_re(invoice_date.re, !invoice_date.multi_line, invoice_date.dot_matches_newline.unwrap_or(invoice_date.multi_line)),
            invoice_item_regex: build_re(invoice_item.re, !invoice_item.multi_line, invoice_item.dot_matches_newline.unwrap_or(invoice_date.multi_line)),
            invoice_discount_regex: invoice_discount_item.map(|item| Regex::new(item.re).unwrap()),
            vat_classes: vat_map,
            default_vat_class,
        }
    }

    pub fn get_meta(&self, invoice_text: &str) -> Result<InvoiceMeta, InvoiceParseError> {
        let invoice_number: String = self.invoice_number_regex.captures(invoice_text)
            .and_then(|captures| captures.name("INVOICE_NUMBER"))
            .ok_or(InvoiceParseError::FieldMissingError("INVOICE_NUMBER".to_string()))?
            .as_str()
            .trim()
            .to_string();
        let payment_type = None;
        Ok(InvoiceMeta {
            invoice_number,
            sum_gross: self.get_gross_sum(&invoice_text)?,
            payment_type,
            date: self.get_date(&invoice_text)?,
        })
    }

    fn get_gross_sum(&self, invoice_text: &str) -> Result<f64, InvoiceParseError> {
        return Ok(parse_as_float(self.invoice_total_regex.captures(invoice_text)
                .and_then(|c| c.name("SUM"))
                .ok_or(InvoiceParseError::FieldMissingError("SUM".to_string()))?
                .as_str()));
    }

    fn get_date(&self, invoice_text: &str) -> Result<PrimitiveDateTime, InvoiceParseError> {
        let date_matches = self.invoice_date_regex.captures(invoice_text)
            .ok_or(InvoiceParseError::FieldMissingError("INVOICE_DATE".to_string()))?;
        let (y, m, d): (i32, u8, u8) = (
            date_matches.name("year").ok_or(InvoiceParseError::FieldMissingError("date.year".to_string()))?.as_str().parse()?,
            date_matches.name("month").ok_or(InvoiceParseError::FieldMissingError("date.month".to_string()))?.as_str().parse()?,
            date_matches.name("day").ok_or(InvoiceParseError::FieldMissingError("date.day".to_string()))?.as_str().parse()?,
        );
        let (h, i, s): (u8, u8, u8) = (
            match date_matches.name("hour") {
                Some(re_match) => re_match.as_str().parse()?,
                None => 0
            },
            match date_matches.name("min") {
                Some(re_match) => re_match.as_str().parse()?,
                None => 0
            },
            match date_matches.name("sec") {
                Some(re_match) => re_match.as_str().parse()?,
                None => 0
            },
        );
        Ok(PrimitiveDateTime::new(
            Date::from_calendar_date(y, m.try_into()?, d)?,
            Time::from_hms(h, i, s)?,
        ))
    }

    pub fn get_items(&self, invoice_text: &str) -> Result<Vec<InvoiceItem>, InvoiceParseError> {
        let mut position_counter: u32 = 1;
        let discount_items: Vec<InvoiceItem> = match &self.invoice_discount_regex {
            Some(re) => invoice_text.lines()
                .filter(|line| re.is_match(line))
                .map(|line| self.extract_discount_item_from_capture_groups(re.captures(line).unwrap()))
                .collect::<Result<Vec<InvoiceItem>,InvoiceParseError>>()?,
            None => vec![],
        };
        let mut items: Vec<InvoiceItem> = self.invoice_item_regex
            .captures_iter(&invoice_text)
            .map(|captures| self.extract_item_from_capture_groups(captures, &mut position_counter))
            .collect::<Result<Vec<InvoiceItem>, InvoiceParseError>>()?;
        items.extend(discount_items);
        Ok(items)
    }

    fn extract_discount_item_from_capture_groups(&self, groups: Captures) -> Result<InvoiceItem, InvoiceParseError> {
        let vat: f64 = self.get_vat_rate_from_class(groups.name("VAT").unwrap().as_str().trim())?;
        let discount: f64 = match groups.name("NET_PRICE_SINGLE") {
            Some(net_price_total) => parse_as_float(net_price_total.as_str().trim()),
            None => parse_as_float(groups.name("GROSS_PRICE_TOTAL").unwrap().as_str().trim()) * (1f64 - (vat / (1f64 + vat))),
        };
        Ok(InvoiceItem {
            typ: InvoiceItemType::Credit,
            pos: u32::MAX,
            article_number: "".to_string(),
            description: self.extract_description(groups)?,
            net_price_single: discount,
            vat,
            amount: 1f64,
            net_total_price: discount,

        })
    }

    fn extract_item_from_capture_groups(&self, groups: Captures, pos_counter: &mut u32) -> Result<InvoiceItem, InvoiceParseError> {
        let pos: u32 = match groups.name("POS") {
            Some(p) => p.as_str().parse::<u32>()?,
            None => pos_counter.clone()
        };
        *pos_counter = *pos_counter + 1u32;
        let vat: f64 = match groups.name("VAT") {
            Some(class) => self.get_vat_rate_from_class(class.as_str().trim())?,
            None => self.default_vat_class.ok_or(InvoiceParseError::FieldMissingError("VAT".to_string()))?,
        };
        let packaging_unit_amount: f64 = match groups.name("PU_AMOUNT") {
            Some(pu_amount) => parse_as_float(pu_amount.as_str().trim()),
            None => 1f64,
        };
        let amount: f64 = parse_as_float(groups.name("AMOUNT").unwrap().as_str().trim());
        let net_price_single: f64 =  match groups.name("NET_PRICE_SINGLE") {
            Some(net_price_single) => parse_as_float(net_price_single.as_str().trim()),
            None => (parse_as_float(groups.name("GROSS_PRICE_SINGLE").unwrap().as_str().trim()) * (1f64 - (vat / (1f64 + vat))) * 1000f64).round() / 1000f64,
        };
        let net_price_total: f64 = match groups.name("NET_PRICE_TOTAL") {
            Some(net_price_total) => parse_as_float(net_price_total.as_str().trim()),
            None => (parse_as_float(groups.name("GROSS_PRICE_TOTAL").unwrap().as_str().trim()) * (1f64 - (vat / (1f64 + vat))) * 1000f64).round() / 1000f64,
        };

        Ok(InvoiceItem {
            typ: if (net_price_single * amount) >= 0.0f64 { InvoiceItemType::Expense } else { InvoiceItemType::Credit },
            pos,
            article_number: match groups.name("ARTNR") {
                Some(m) => m.as_str().to_string(),
                None => "".to_string(),
            },
            description: self.extract_description(groups)?,
            net_price_single: ((net_price_single / packaging_unit_amount) * 1000f64).round() / 1000f64,
            vat,
            amount: amount * packaging_unit_amount,
            net_total_price: net_price_total,
        })
    }

    fn extract_description(&self, groups: Captures) -> Result<String, InvoiceParseError> {
        Ok(groups.name("DESC")
            .ok_or(InvoiceParseError::FieldMissingError("DESC".to_string()))?
            .as_str().to_string())
    }

    fn get_vat_rate_from_class(&self, class: &str) -> Result<f64, InvoiceParseError> {
        self.vat_classes.get(class).ok_or_else(|| InvoiceParseError::UnrecognizedVatClass(class.to_string())).copied()
    }
}

fn parse_as_float(float: &str) -> f64 {
    // remove 1000-dot and all whitespace
    let raw = float.replace(".", "").replacen(" ", "", 1000);
    let is_negative = raw.contains("-");
    let absolute = raw.replace("-", "").replace(",", ".");
    if is_negative {
        return -1f64 * (absolute.parse::<f64>().unwrap())
    }
    return absolute.parse::<f64>().unwrap();
}

impl Vendor for RegexVendor {
    fn extract_invoice_data(&self, pdf: &[u8], vendor: InvoiceVendor) -> anyhow::Result<Invoice> {
        let text = pdf_extract::extract_text_from_mem(pdf)?;
        Ok(Invoice {
            vendor: vendor,
            meta: self.get_meta(&text)?,
            items: self.get_items(&text)?,
        })
    }
}

pub static METRO: Lazy<RegexVendor> = Lazy::new(|| RegexVendor::new(
    ItemRegex {re: r"RECHNUNGS?-? ?NR\.?\:?\s+(?P<INVOICE_NUMBER>[\.\d\/]+)", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"RECHNUNGSDATUM:\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4}) (?P<hour>\d\d):(?P<min>\d\d)", multi_line: false, ..ItemRegex::default() },
    ItemRegex {re: r"SUMME EUR\s+(?P<SUM>[\d\.,\-]+)([\s\-]+(?P<PAYMENT_TYPE>[a-zA-Z0-9:\-\., ]+) +[\d\.,\-]+)?", multi_line: false, ..ItemRegex::default() },
    ItemRegex {re: r"^(?P<MM>.) (?P<ARTNR>\d{6}\.\d) (?P<EAN>[\d ]{14}) (?P<DESC>.{31}) (?P<PACK>.{2}) (?P<EINZELPREIS>.{11}) (?P<PU_AMOUNT>.{10}) (?P<NET_PRICE_SINGLE>.{10}) (?P<AMOUNT>.{6}) (?P<NET_PRICE_TOTAL>.{11}) (?P<VAT>.) (?P<STUECKPREIS>.{10})[  ](?P<INT>.) (?P<KD>.+)?$", multi_line: false, ..ItemRegex::default() },
    Some(ItemRegex {re: r"^ {26}(?P<DESC>.{50}) *(?P<NET_PRICE_SINGLE>.{11}) (?P<VAT>.)?[ 0-9]{12}$", multi_line: false, ..ItemRegex::default() }),
    vec![("A", 0.19f64), ("B", 0.07f64)],
    None,
));

pub static BAUHAUS: Lazy<RegexVendor> = Lazy::new(|| RegexVendor::new(
    ItemRegex {re: r"Einzelrechnung\s+Nr\.\s+(?P<INVOICE_NUMBER>[\.\d\/]+)", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"Rechnungsdatum\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4})", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"Zu zahlender Betrag\s+(?P<SUM>[\d\.,\-]+)\ EUR", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"^(?P<POS>\d)\s+(?P<ARTNR>\d{8})\s+(?P<DESC>.{1,100})\s+(?P<AMOUNT>\d{1,6}) (ST|KAR)\s+(?P<GROSS_PRICE_SINGLE>.{1,7})\s+(?P<GROSS_PRICE_TOTAL>.{1,7})\s+(?P<VAT>\w)$", multi_line: false, ..ItemRegex::default()},
    None,
    vec![("C", 0.19f64)],
    None,
));

pub static IKEA: Lazy<RegexVendor> = Lazy::new(|| RegexVendor::new(
    ItemRegex {re: r"Rechnungsnummer: (?P<INVOICE_NUMBER>\w+)", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"Rechnungsdatum:\s+(?P<day>\d{1,2})\.(?P<month>\d{1,2})\.(?P<year>\d{2,4})", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"Rechnungssumme:\s+€\s+(?P<SUM>[0-9,\.]+)", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"^(?P<ARTNR>\d{2,3}\.\d{2,3}\.\d{2,3})\s+(?P<DESC>.+)\s+(?P<AMOUNT>\d+)\s+(?P<GROSS_PRICE_SINGLE>\d{1,5},\d{0,2})\s+(?P<VAT>\d{1,2}) %\s+€ (?P<GROSS_PRICE_TOTAL>[0-9,\.]+)$", multi_line: false, ..ItemRegex::default()},
    None,
    vec![("19", 0.19f64), ("7", 0.07f64)],
    None,
));

pub static MEDICALCORNER: Lazy<RegexVendor> = Lazy::new(|| RegexVendor::new(
    ItemRegex {re: r"Rechnung\s+(?P<INVOICE_NUMBER>\w+)", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"Rechnungsdatum:\s+\nKundennummer:\s+\nLieferschein:\s+\nLieferdatum:\s+\nBearbeiter:\s+\n.+\n(?<day>\d{1,2})\.(?<month>\d{2})\.(?<year>\d{4})", multi_line: true, ..ItemRegex::default()},
    ItemRegex {re: r"Gesamt (?<SUM>\d+,\d{2}) EUR", multi_line: false, ..ItemRegex::default()},
    ItemRegex {re: r"\n\n(?P<POS>\d+) (?P<ARTNR>[A-Z0-9-_]+([\w&&[^A-Z]]{4})?) (?P<DESC>.+?) (?P<AMOUNT>\d+) (?<PVAT>\d+)% (?P<GROSS_PRICE_SINGLE>\d+,\d{2}) (?P<GROSS_PRICE_TOTAL>\d+,\d{2})", multi_line: true, dot_matches_newline: Some(true), ..ItemRegex::default()},
    None,
    vec![("0", 0.0f64), ("19", 0.19f64), ("7", 0.07f64)],
    None,
));
