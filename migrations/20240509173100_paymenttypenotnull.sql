-- Add migration script here
ALTER TABLE invoice ALTER COLUMN "payment_type" DROP NOT NULL;
