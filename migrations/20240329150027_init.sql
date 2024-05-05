-- Add migration script here
CREATE TABLE cost_centre
(
    id   BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL
);

CREATE TABLE invoice
(
    id             BIGSERIAL PRIMARY KEY,
    vendor         VARCHAR          NOT NULL,
    invoice_number VARCHAR          NOT NULL,
    sum            DOUBLE PRECISION NOT NULL,
    date           TIMESTAMP        NOT NULL,
    payment_type   VARCHAR          NOT NULL
);

CREATE TABLE invoice_item
(
    id               BIGSERIAL PRIMARY KEY,
    invoice_id       BIGINT           NOT NULL,
    CONSTRAINT fk_invoice_id
        FOREIGN KEY (invoice_id)
            REFERENCES "invoice" (id),
    typ              VARCHAR          NOT NULL,
    description      VARCHAR          NOT NULL,
    amount           DOUBLE PRECISION NOT NULL,
    net_price_single DOUBLE PRECISION NOT NULL,
    net_price_total  DOUBLE PRECISION NOT NULL,
    vat              DOUBLE PRECISION NOT NULL,
    cost_centre_id   BIGINT REFERENCES cost_centre (id) NULL
);

CREATE INDEX invoice_item_invoice_id ON invoice_item (invoice_id);
CREATE INDEX invoice_item_cost_centre_id ON invoice_item (cost_centre_id);
