-- Add migration script here
CREATE TABLE project
(
    id			BIGSERIAL	PRIMARY KEY,
    name		VARCHAR		NOT NULL,
    description		TEXT		NOT NULL,
    active 		BOOLEAN		NOT NULL,
    "start"		TIMESTAMP	NULL,
    "end"		TIMESTAMP	NULL
);

CREATE INDEX project_name_idx ON project (name);
CREATE INDEX project_start_date_idx ON project ("start");
CREATE INDEX project_end_date_idx ON project ("end");
ALTER TABLE invoice_item ADD COLUMN project_id BIGINT REFERENCES project (id) NULL DEFAULT NULL;
