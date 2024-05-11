use super::vendor::Vendor;

pub(crate) const BAUHAUS: Vendor = Vendor {
    vendor_id: crate::InvoiceVendor::Bauhaus,
    customer_number: Some(r"Kundennummer\s+(?P<CUSTOMER_NUMBER>\d[\d ]*\d)".to_string()),
    invoice_number: r"Einzelrechnung\s+Nr\.\s+(?P<INVOICE_NUMBER>[\.\d\/]+)".to_string(),
    invoice_date: r"Rechnungsdatum\s+(?P<day>\d\d)\.(?P<month>\d\d)\.(?P<year>\d{4})".to_string(),
    invoice_total: r"Zu zahlender Betrag\s+(?P<SUM>[\d\.,\-]+)\ EUR".to_string(),
    invoice_item: r"^(?P<POS>\d)\s+(?P<ARTNR>\d{8})\s+(?P<BEZEICHNUNG>.{1,100})\s+(?P<MENGE>\d{1,6}) (ST|KAR)\s+(?P<EINZELPREIS>.{1,7})\s+(?P<GESAMTPREIS>.{1,7})\s+(?P<MWST>\w)$".to_string(),
    invoice_credit_item: None,
    vat_map: super::vendor::VatMap::new(vec![("19".to_string(), 0.19f64)])
};
