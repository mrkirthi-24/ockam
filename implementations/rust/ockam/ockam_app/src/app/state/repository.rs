use std::path::Path;
use std::sync::Arc;

use miette::{miette, IntoDiagnostic};
use ockam::{SqlxDatabase, ToSqlxType};

use crate::app::state::model::{ModelState, TcpOutletRow};
use crate::Result;
use ockam_core::async_trait;
use sqlx::*;

#[async_trait]
pub trait ModelStateRepository: Send + Sync + 'static {
    async fn store(&self, model_state: &ModelState) -> Result<()>;
    async fn load(&self) -> Result<ModelState>;
}

pub struct ModelStateSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl ModelStateSqlxDatabase {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::create(Arc::new(
            SqlxDatabase::create(path).await.map_err(|e| miette!(e))?,
        )))
    }

    pub fn create(database: Arc<SqlxDatabase>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ModelStateRepository for ModelStateSqlxDatabase {
    async fn store(&self, model_state: &ModelState) -> Result<()> {
        for tcp_outlet in &model_state.tcp_outlets {
            let query = query("INSERT INTO tcp_outlet VALUES (?, ?, ?, ?)")
                .bind(tcp_outlet.socket_addr.to_sql())
                .bind(tcp_outlet.worker_addr.to_sql())
                .bind(tcp_outlet.alias.to_sql())
                .bind(tcp_outlet.payload.as_ref().map(|p| p.to_sql()));
            query
                .execute(&self.database.pool)
                .await
                .map(|_| ())
                .map_err(|e| miette!(e))?;
        }
        Ok(())
    }

    async fn load(&self) -> Result<ModelState> {
        let query = query_as("SELECT * FROM tcp_outlet");
        let rows: Vec<TcpOutletRow> = query
            .fetch_all(&self.database.pool)
            .await
            .into_diagnostic()?;
        let values: Result<Vec<_>> = rows.iter().map(|r| r.tcp_outlet_status()).collect();
        Ok(ModelState::new(values?))
    }
}
