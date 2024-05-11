-- Add migration script here
ALTER TABLE invoice_item
    DROP CONSTRAINT fk_invoice_id,
    ADD CONSTRAINT fk_invoice_id
        FOREIGN KEY (invoice_id)
            REFERENCES invoice (id)
            ON DELETE CASCADE;
