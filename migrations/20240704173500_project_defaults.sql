ALTER TABLE project ADD COLUMN "default" BOOL NOT NULL DEFAULT false;
CREATE UNIQUE INDEX project_default_idx ON project ("default") WHERE "default" = true;
