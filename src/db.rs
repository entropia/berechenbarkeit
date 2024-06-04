use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, QueryBuilder};
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPool;
use time::PrimitiveDateTime;

use berechenbarkeit_lib::Invoice;

type Result<T, E = sqlx::Error> = std::result::Result<T, E>;

pub(crate) struct DatabaseConnection(pub(crate) PoolConnection<Postgres>);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
    where PgPool: FromRef<S>,
          S: Send + Sync, {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let conn = pool.acquire().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Self(conn))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DBInvoice {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub vendor: String,
    pub invoice_number: String,
    pub sum_gross: f64,
    pub date: PrimitiveDateTime,
    pub payment_type: Option<String>,
}

impl DBInvoice {
    pub(crate) async fn get_all(connection: &mut PgConnection) -> Result<Vec<DBInvoice>> {
        sqlx::query_as!(DBInvoice, r#"SELECT * FROM "invoice" ORDER BY date DESC"#).fetch_all(connection).await
    }

    pub(crate) async fn get_by_id(id: i64, connection: &mut PgConnection) -> Result<DBInvoice> {
        sqlx::query_as!(DBInvoice, r#"SELECT * FROM "invoice" WHERE id=$1"#, id).fetch_one(connection).await
    }

    pub(crate) async fn insert(object: DBInvoice, connection: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO "invoice" (vendor, invoice_number, sum_gross, date, payment_type) VALUES ($1, $2, $3, $4, $5) RETURNING id"#,
            object.vendor,
            object.invoice_number,
            object.sum_gross,
            object.date,
            object.payment_type,
        ).fetch_one(connection).await?.id)
    }

    pub(crate) async fn delete(id: i64, connection: &mut PgConnection) -> Result<()> {
        let result = sqlx::query!(
            r#"DELETE FROM invoice WHERE id=$1"#,
            id
        ).execute(connection).await?;
        Ok(())
    }
}

impl From<Invoice> for DBInvoice {
    fn from(invoice: Invoice) -> Self {
        DBInvoice {
            id: None,
            vendor: invoice.vendor.to_string(),
            invoice_number: invoice.meta.invoice_number.clone(),
            sum_gross: invoice.meta.sum_gross,
            date: invoice.meta.date,
            payment_type: invoice.meta.payment_type.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DBInvoiceItem {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub position: i64,
    pub invoice_id: i64,
    pub typ: String,
    pub description: String,
    pub amount: f64,
    pub net_price_single: f64,
    pub vat: f64,
    pub vat_exempt: bool,
    pub cost_centre_id: Option<i64>,
    pub cost_centre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InvoiceItemExtended {
    pub invoice_vendor: String,
    pub invoice_number: String,
    pub invoice_date: PrimitiveDateTime,
    pub id: i64,
    pub position: i64,
    pub invoice_id: i64,
    pub typ: String,
    pub description: String,
    pub amount: f64,
    pub net_price_single: f64,
    pub vat: f64,
    pub vat_exempt: bool,
    pub cost_centre_id: Option<i64>,
    pub cost_centre: Option<String>,
}

impl DBInvoiceItem {
    pub(crate) async fn bulk_insert(connection: &mut PgConnection, objects: Vec<DBInvoiceItem>) -> Result<()> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO invoice_item (position, invoice_id, typ, description, amount, net_price_single, vat, vat_exempt, cost_centre_id)",
        );
        qb.push_values(objects.iter(), |mut b, rec| {
            b
                .push_bind(rec.position)
                .push_bind(rec.invoice_id)
                .push_bind(&rec.typ)
                .push_bind(&rec.description)
                .push_bind(rec.amount)
                .push_bind(rec.net_price_single)
                .push_bind(rec.vat)
                .push_bind(rec.vat_exempt)
                .push_bind(rec.cost_centre_id);
        });

        qb.build().execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn get_all(connection: &mut PgConnection) -> Result<Vec<InvoiceItemExtended>> {
        sqlx::query_as!(InvoiceItemExtended, r#"SELECT invoice.vendor as invoice_vendor, invoice.invoice_number, invoice.date AS invoice_date, invoice_item.*, cost_centre.name as "cost_centre?" FROM invoice_item LEFT OUTER JOIN cost_centre ON invoice_item.cost_centre_id = cost_centre.id JOIN invoice ON invoice_item.invoice_id = invoice.id ORDER BY invoice.date,invoice.id,invoice_item.position,invoice_item.id"#).fetch_all(connection).await
    }

    pub(crate) async fn get_by_id(invoiceitem_id: i64, connection: &mut PgConnection) -> Result<DBInvoiceItem> {
        sqlx::query_as!(DBInvoiceItem, r#"SELECT invoice_item.*, cost_centre.name as "cost_centre?" FROM invoice_item LEFT OUTER JOIN cost_centre ON invoice_item.cost_centre_id = cost_centre.id WHERE invoice_item.id = $1 ORDER BY invoice_item.position,invoice_item.id"#, invoiceitem_id).fetch_one(connection).await
    }

    pub(crate) async fn get_by_invoice_id(invoice_id: i64, connection: &mut PgConnection) -> Result<Vec<DBInvoiceItem>> {
        sqlx::query_as!(DBInvoiceItem, r#"SELECT invoice_item.*, cost_centre.name as "cost_centre?" FROM invoice_item LEFT OUTER JOIN cost_centre ON invoice_item.cost_centre_id = cost_centre.id WHERE invoice_item.invoice_id = $1 ORDER BY invoice_item.position,invoice_item.id"#, invoice_id).fetch_all(connection).await
    }

    pub(crate) async fn calculate_sum_gross_by_invoice_id(invoice_id: i64, connection: &mut PgConnection) -> Result<f64> {
        Ok(sqlx::query!(r#"SELECT SUM(invoice_item.amount * invoice_item.net_price_single * (1 + invoice_item.vat)) FROM invoice_item WHERE invoice_id=$1"#, invoice_id).fetch_one(connection).await?.sum.unwrap_or(0f64))

    }

    pub(crate) async fn update_amount(id: i64, amount: f64, connection: &mut PgConnection) -> Result<()> {
        sqlx::query!(r#"UPDATE "invoice_item" SET amount=$1 WHERE id=$2"#, amount, id).execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn update_cost_centre(id: i64, cost_centre_id: Option<i64>, connection: &mut PgConnection) -> Result<()> {
        sqlx::query!(r#"UPDATE "invoice_item" SET cost_centre_id=$1 WHERE id=$2"#, cost_centre_id, id).execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn update_vat_exemption(id: i64, vat_exempt: bool, connection: &mut PgConnection) -> Result<()> {
        sqlx::query!(r#"UPDATE "invoice_item" SET vat_exempt=$1 WHERE id=$2"#, vat_exempt, id).execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn insert(object: DBInvoiceItem, connection: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO "invoice_item" (position, invoice_id, typ, description, amount, net_price_single, vat, vat_exempt, cost_centre_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"#,
            object.position,
            object.invoice_id,
            object.typ,
            object.description,
            object.amount,
            object.net_price_single,
            object.vat,
            object.vat_exempt,
            object.cost_centre_id,
        ).fetch_one(connection).await?.id)
    }
}

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
    pub(crate) async fn get_all(connection: &mut PgConnection) -> Result<Vec<DBCostCentre>> {
        sqlx::query_as!(DBCostCentre, r#"SELECT id, name FROM "cost_centre""#).fetch_all(connection).await
    }

    pub(crate) async fn insert(name: &str, connection: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO "cost_centre" (name) VALUES ($1) RETURNING id"#,
            name,
        ).fetch_one(connection).await?.id)
    }

    pub(crate) async fn delete(id: i64, connection: &mut PgConnection) -> Result<()> {
        sqlx::query!(r#"DELETE FROM "cost_centre" WHERE id=$1"#, id).execute(connection).await?;
        Ok(())
    }

    pub(crate) async fn get_summary(connection: &mut PgConnection) -> Result<Vec<CostCentreWithSum>> {
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
