-- Add migration script here
ALTER TABLE invoice_item ADD COLUMN vat_exempt BOOLEAN NOT NULL DEFAULT false;
