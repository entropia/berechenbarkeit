use super::vendor::Vendor;
pub(crate) const METRO: Vendor = Vendor {
    vendor_id: crate::InvoiceVendor::Metro,
    customer_number: Some(r"KUNDE.\s+(?P<CUSTOMER_NUMBER>\d[\d ]*\d)".to_string()),
    invoice_number: r"RECHNUNGS?-? ?NR\.?\:?\s+(?P<INVOICE_NUMBER>[\.\d\/]+)".to_string(),
    invoice_date: r"RECHNUNGSDATUM:\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4}) (?P<hour>\d\d):(?P<minute>\d\d)".to_string(),
    invoice_total: r"SUMME EUR\s+(?P<SUM>[\d\.,\-]+)[\s\-]+(?P<PAYMENT_TYPE>[a-zA-Z0-9:\-\., ]+) +[\d\.,\-]+".to_string(),
    invoice_item: r"^(?P<MM>.) (?P<ARTNR>\d{6}\.\d) (?P<EAN>[\d ]{14}) (?P<BEZEICHNUNG>.{31}) (?P<PACK>.{2}) (?P<EINZELPREIS>.{11}) (?P<INHALTKOLLI>.{10}) (?P<KOLLIPREIS>.{10}) (?P<MENGE>.{6}) (?P<GESAMTPREIS>.{11}) (?P<M>.) (?P<STUECKPREIS>.{10})[  ](?P<INT>.) (?P<KD>.+)?$".to_string(),
    invoice_credit_item: Some(r"^ {26}(?P<BEZEICHNUNG>.{50}) *(?P<GESAMTPREIS>.{11}) (?P<M>.)?[ 0-9]{12}$".to_string()),
    vat_map: super::vendor::VatMap::new(vec![("A".to_string(), 0.19f64), ("B".to_string(), 0.07f64)]),
};
