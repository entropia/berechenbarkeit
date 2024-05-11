-- Add migration script here
ALTER TABLE invoice RENAME COLUMN "sum" to "sum_gross";
