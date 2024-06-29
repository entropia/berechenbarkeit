use std::fmt;
use axum::{
    async_trait,
    extract::{
        FromRef,
        FromRequestParts
    },
    http::{
        StatusCode,
        request::Parts,
    },
};
use serde::{
    Deserialize,
    Serialize,
};
use sqlx::{
    Postgres,
    pool::PoolConnection,
    postgres::PgPool
};
use time::{
    macros::format_description,
    PrimitiveDateTime,
};

pub(crate) type DBResult<T, E = sqlx::Error> = std::result::Result<T, E>;

pub(crate) struct DatabaseConnection(pub(crate) PoolConnection<Postgres>);

#[derive(Deserialize, Debug, Clone, Serialize)]
pub(crate) struct DbDate {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub datetime: Option<PrimitiveDateTime>,
}

impl fmt::Display for DbDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
        write!(f, "{}", self.datetime.map(|d| d.format(&format).ok()).flatten().unwrap_or("".to_string()))
    }
}

impl From<Option<PrimitiveDateTime>> for DbDate {
    fn from(e: Option<PrimitiveDateTime>) -> Self{
        DbDate {datetime: e}
    }
}

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
