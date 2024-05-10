-- Add migration script here
ALTER TABLE invoice_item ADD COLUMN "position" BIGINT;

UPDATE invoice_item SET "position" = "id";

ALTER TABLE invoice_item ALTER COLUMN "position" SET NOT NULL;
