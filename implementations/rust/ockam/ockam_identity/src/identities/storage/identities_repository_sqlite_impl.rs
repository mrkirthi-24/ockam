use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef};
use rusqlite::Error::ToSqlConversionFailure;
use rusqlite::{params, Row, ToSql};

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use crate::models::{ChangeHistory, Identifier};
use crate::repository::{FromRow, SqliteDb};
use crate::{
    AttributesEntry, IdentitiesReader, IdentitiesRepository, IdentitiesWriter,
    IdentityAttributesReader, IdentityAttributesWriter,
};

/// Implementation of `IdentityAttributes` trait based on an underlying `Storage`
#[derive(Clone)]
pub struct IdentitiesSqliteRepository {
    db: Arc<SqliteDb>,
}

#[async_trait]
impl IdentitiesRepository for IdentitiesSqliteRepository {
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        Arc::new(self.clone())
    }

    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        Arc::new(self.clone())
    }

    fn as_identities_reader(&self) -> Arc<dyn IdentitiesReader> {
        Arc::new(self.clone())
    }

    fn as_identities_writer(&self) -> Arc<dyn IdentitiesWriter> {
        Arc::new(self.clone())
    }
}

impl IdentitiesSqliteRepository {
    /// Create a new repository
    pub fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }

    /// Create a new in-memory repository
    pub fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(Arc::new(SqliteDb::in_memory()?))))
    }
}

#[async_trait]
impl IdentityAttributesReader for IdentitiesSqliteRepository {
    async fn get_attributes(&self, identity_id: &Identifier) -> Result<Option<AttributesEntry>> {
        todo!()
    }

    async fn list(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        todo!()
    }
}

#[async_trait]
impl IdentityAttributesWriter for IdentitiesSqliteRepository {
    async fn put_attributes(&self, sender: &Identifier, entry: AttributesEntry) -> Result<()> {
        todo!()
    }

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()> {
        todo!()
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        todo!()
    }
}

#[async_trait]
impl IdentitiesWriter for IdentitiesSqliteRepository {
    async fn update_identity(
        &self,
        identifier: &Identifier,
        change_history: &ChangeHistory,
    ) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO identity VALUES (?1, ?2)",
                params![identifier, change_history],
            )
            .map(|_| ())
    }
}

#[async_trait]
impl IdentitiesReader for IdentitiesSqliteRepository {
    async fn retrieve_identity(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        self.db.query_maybe_one(
            "SELECT change_history FROM IDENTITY WHERE identifier = ?1",
            params![identifier],
            ChangeHistoryFromRow,
        )
    }
}

struct ChangeHistoryFromRow;

impl FromRow<ChangeHistory> for ChangeHistoryFromRow {
    fn make(&self, row: &Row) -> rusqlite::Result<ChangeHistory> {
        row.get("change_history")
    }
}

impl ToSql for Identifier {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl ToSql for ChangeHistory {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
        let exported = self
            .export()
            .map_err(|e| ToSqlConversionFailure(e.into()))?;
        Ok(ToSqlOutput::Owned(Value::Blob(exported)))
    }
}

impl FromSql for ChangeHistory {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(ChangeHistory::import(value.as_blob()?).map_err(|e| FromSqlError::Other(e.into()))?)
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_identities_repository() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqliteDb::create(db_file.path()).await?;
        let repository = IdentitiesSqliteRepository::new(Arc::new(db));
        let identifier = Identifier::from_str("I6342c580429b9a0733880bea4fa18f8055871130")?;
        let change_history = ChangeHistory::import(&hex::decode("81a201583ba20101025835a4028201815820530d1c2e9822433b679a66a60b9c2ed47c370cd0ce51cbe1a7ad847b5835a96303f4041a64dd4060051a77a94360028201815840042fff8f6c80603fb1cec4a3cf1ff169ee36889d3ed76184fe1dfbd4b692b02892df9525c61c2f1286b829586d13d5abf7d18973141f734d71c1840520d40a0e").unwrap())?;
        repository
            .update_identity(&identifier, &change_history)
            .await?;

        let identity = repository.retrieve_identity(&identifier).await?;
        assert!(identity.is_some());
        Ok(())
    }
}
