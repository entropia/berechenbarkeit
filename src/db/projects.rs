use serde::{
    Deserialize, Serialize,
};
use sqlx::{
    PgConnection,
};

use crate::db::util::{
    DbDate,
    DBResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DBProject {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub default: bool,
    pub start: DbDate,
    pub end: DbDate,
}

impl DBProject {
    pub(crate) async fn get_ordered_by_id(conn: &mut PgConnection) -> DBResult<Vec<DBProject>> {
        Ok(sqlx::query_as!(
                DBProject,
                r#"SELECT * FROM "project" ORDER BY id ASC;"#
        ).fetch_all(conn).await?)
    }
    pub(crate) async fn get(conn: &mut PgConnection) -> DBResult<Vec<DBProject>> {
        Ok(sqlx::query_as!(
                DBProject,
                r#"SELECT * FROM "project" ORDER BY "default" DESC, active DESC, id DESC;"#
        ).fetch_all(conn).await?)
    }

    pub(crate) async fn get_by_id(project_id: i64, conn: &mut PgConnection) -> DBResult<DBProject> {
        Ok(sqlx::query_as!(
                DBProject,
                r#"SELECT * FROM "project" WHERE id = $1 ORDER BY id ASC;"#,
                project_id,
        ).fetch_one(conn).await?)
    }

    pub(crate) async fn add(project: DBProject, conn: &mut PgConnection) -> DBResult<DBProject> {
        Ok(sqlx::query_as!(
                DBProject,
                r#"INSERT INTO "project" (name, description, active, "default", "start", "end") VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"#,
                project.name,
                project.description,
                project.active,
                project.default,
                project.start.datetime,
                project.end.datetime,
        ).fetch_one(conn).await?)
    }

    pub(crate) async fn update(project: DBProject, conn: &mut PgConnection) -> DBResult<DBProject> {
       Ok(sqlx::query_as!(
               DBProject,
               r#"INSERT INTO "project" (id, name, description, active, "default", "start", "end")
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   ON CONFLICT(id)
                   DO UPDATE SET name = $2, description = $3, active = $4, "default" = $5, "start" = $6, "end" = $7
                   RETURNING *"#,
               project.id.unwrap(),
               project.name,
               project.description,
               project.active,
               project.default,
               project.start.datetime,
               project.end.datetime,
        ).fetch_one(conn).await?)
    }

    pub(crate) async fn delete(project_id: i64, conn: &mut PgConnection) -> DBResult<()> {
        sqlx::query!(
            r#"DELETE FROM "project" WHERE id=$1"#,
            project_id,
        ).execute(conn).await?;
        Ok(())
    }

    pub(crate) async fn set_default(project_id: i64, conn: &mut PgConnection) -> DBResult<()> {
        Self::clear_default(conn).await?;
        sqlx::query!(
            r#"UPDATE "project" SET "default" = true WHERE id = $1;"#,
            project_id,
        ).execute(conn).await?;
        Ok(())
    }

    pub(crate) async fn clear_default(conn: &mut PgConnection) -> DBResult<()> {
        sqlx::query!(r#"UPDATE "project" SET "default" = false WHERE "default";"#)
            .execute(conn).await?;
        Ok(())
    }
}
