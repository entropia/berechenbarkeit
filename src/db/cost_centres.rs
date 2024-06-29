use serde::{
    Serialize,
};
use sqlx::{
    PgConnection,
};
use crate::db::util::DBResult;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DBCostCentre {
    pub id: i64,
    pub name: String,
}


#[derive(Debug, Clone, Serialize)]
pub(crate) struct CostCentreWithSum {
    pub cost_centre_name: String,
    pub vat: f64,
    pub sum_net: f64,
    pub sum_vat_exempted: f64,
}

impl DBCostCentre {
    pub(crate) async fn get_all(connection: &mut PgConnection) -> DBResult<Vec<DBCostCentre>> {
        sqlx::query_as!(DBCostCentre, r#"SELECT id, name FROM "cost_centre""#).fetch_all(connection).await
    }

    pub(crate) async fn insert(name: &str, connection: &mut PgConnection) -> DBResult<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO "cost_centre" (name) VALUES ($1) RETURNING id"#,
            name,
        ).fetch_one(connection).await?.id)
    }

    pub(crate) async fn delete(id: i64, connection: &mut PgConnection) -> DBResult<()> {
        sqlx::query!(r#"DELETE FROM "cost_centre" WHERE id=$1"#, id).execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn get_summary(connection: &mut PgConnection) -> DBResult<Vec<CostCentreWithSum>> {
        Ok(sqlx::query!(r#"SELECT cost_centre.name AS cost_centre_name,
                invoice_item.vat AS vat,
                ROUND(SUM(invoice_item.amount::numeric * invoice_item.net_price_single::numeric), 3)::double precision AS sum_net,
                ROUND(SUM(CASE WHEN invoice_item.vat_exempt THEN (invoice_item.amount::numeric * invoice_item.net_price_single::numeric) else 0 END), 3)::double precision as sum_vat_exempted
                FROM cost_centre
                JOIN invoice_item ON cost_centre.id=invoice_item.cost_centre_id
                GROUP BY cost_centre_name, vat
                ORDER BY cost_centre_name, vat;"#)
            .fetch_all(connection).await?.into_iter().map(|x| CostCentreWithSum {
                cost_centre_name: x.cost_centre_name,
                vat: x.vat,
                sum_net: x.sum_net.unwrap_or(0f64),
                sum_vat_exempted: x.sum_vat_exempted.unwrap_or(0f64)
            }).collect())
    }
}
