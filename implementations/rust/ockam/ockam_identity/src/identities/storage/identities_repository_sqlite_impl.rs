use core::str::FromStr;
use std::collections::BTreeMap;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Type, Value, ValueRef};
use rusqlite::Error::{FromSqlConversionFailure, ToSqlConversionFailure};
use rusqlite::{named_params, params, Row, ToSql};

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use crate::models::{ChangeHistory, Identifier};
use crate::repository::{FromRow, SqliteDb};
use crate::utils::now;
use crate::{
    AttributesEntry, IdentitiesReader, IdentitiesRepository, IdentitiesWriter,
    IdentityAttributesReader, IdentityAttributesWriter, TimestampInSeconds,
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
    async fn get_attributes(&self, identity: &Identifier) -> Result<Option<AttributesEntry>> {
        self.get_attributes_sync(identity)
    }

    async fn list(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        self.db.query_all(
            "SELECT * FROM identity_attributes",
            [],
            IdentifierAttributesFromRow,
        )
    }
}

impl IdentitiesSqliteRepository {
    fn get_attributes_sync(&self, identity: &Identifier) -> Result<Option<AttributesEntry>> {
        self.db.query_maybe_one(
            "SELECT FROM identity_attributes WHERE identifier=?1",
            params![identity],
            AttributesEntryFromRow,
        )
    }

    fn put_attribute_value_sync(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()> {
        self.db.with_transaction(|transaction| {
            let mut attributes = match self.get_attributes_sync(subject)? {
                Some(entry) => (*entry.attrs()).clone(),
                None => BTreeMap::new(),
            };
            attributes.insert(attribute_name, attribute_value);
            let entry = AttributesEntry::new(attributes, now()?, None, Some(subject.clone()));
            self.put_attributes_sync(subject, entry)
        })
    }

    fn put_attributes_sync(&self, sender: &Identifier, entry: AttributesEntry) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO identity_attributes VALUES (:identifier, :attributes, :added, :expires, :attested)",
                named_params! {
                    ":identifier": sender,
                    ":attributes": minicbor::to_vec(entry.attrs())?,
                    ":added": entry.added(),
                    ":expires": entry.expires(),
                },
            )
    }
}

#[async_trait]
impl IdentityAttributesWriter for IdentitiesSqliteRepository {
    async fn put_attributes(&self, sender: &Identifier, entry: AttributesEntry) -> Result<()> {
        self.put_attributes_sync(sender, entry)
    }

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()> {
        self.put_attribute_value_sync(subject, attribute_name, attribute_value)
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        self.db.execute(
            "DELETE FROM identity_attributes WHERE identifier=?1",
            params![identity],
        )
    }
}

#[async_trait]
impl IdentitiesWriter for IdentitiesSqliteRepository {
    async fn update_identity(
        &self,
        identifier: &Identifier,
        change_history: &ChangeHistory,
    ) -> Result<()> {
        self.db.execute(
            "INSERT INTO identity VALUES (?1, ?2)",
            params![identifier, change_history],
        )
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

impl FromSql for Identifier {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Identifier::from_str(value.as_str()?).map_err(|e| FromSqlError::Other(e.into()))?)
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

impl ToSql for TimestampInSeconds {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0 as i64)))
    }
}

impl FromSql for TimestampInSeconds {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(TimestampInSeconds(value.as_i64()? as u64))
    }
}

impl FromSql for ChangeHistory {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(ChangeHistory::import(value.as_blob()?).map_err(|e| FromSqlError::Other(e.into()))?)
    }
}

struct Bytes {
    bytes: Vec<u8>,
}

impl FromSql for Bytes {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Bytes {
            bytes: value.as_blob()?.to_vec(),
        })
    }
}

struct AttributesEntryFromRow;

impl AttributesEntryFromRow {
    fn make_attributes_entry_from_row(row: &Row) -> rusqlite::Result<AttributesEntry> {
        let attributes = minicbor::decode(row.get::<_, Bytes>("attributes")?.bytes.as_slice())
            .map_err(|e| FromSqlConversionFailure(1, Type::Blob, e.into()))?;
        let added: TimestampInSeconds = row.get("added")?;
        let expires: Option<TimestampInSeconds> = row.get("expires")?;
        let attested_by: Option<Identifier> = row.get("attested_by")?;

        Ok(AttributesEntry::new(
            attributes,
            added,
            expires,
            attested_by,
        ))
    }
}

impl FromRow<AttributesEntry> for AttributesEntryFromRow {
    fn make(&self, row: &Row) -> rusqlite::Result<AttributesEntry> {
        AttributesEntryFromRow::make_attributes_entry_from_row(row)
    }
}

struct IdentifierAttributesFromRow;

impl FromRow<(Identifier, AttributesEntry)> for IdentifierAttributesFromRow {
    fn make(&self, row: &Row) -> rusqlite::Result<(Identifier, AttributesEntry)> {
        let identifier = row.get("identifier")?;
        let attributes = AttributesEntryFromRow::make_attributes_entry_from_row(row)?;
        Ok((identifier, attributes))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::NamedTempFile;

    use crate::{Identity, Vault};

    use super::*;

    #[tokio::test]
    async fn test_identities_repository() -> Result<()> {
        let identity1 = create_identity1().await?;
        let identity2 = create_identity2().await?;
        let db_file = NamedTempFile::new().unwrap();
        let repository = create_repository(db_file.path()).await?;

        // store and retrieve or get an identity
        repository
            .update_identity(identity1.identifier(), identity1.change_history())
            .await?;

        let result = repository
            .retrieve_identity(&identity1.identifier())
            .await?;
        assert_eq!(result, Some(identity1.change_history().clone()));

        let result = repository.get_identity(&identity1.identifier()).await?;
        assert_eq!(result, identity1.change_history().clone());

        // trying to retrieve a missing identity returns None
        let result = repository
            .retrieve_identity(&identity2.identifier())
            .await?;
        assert_eq!(result, None);

        // trying to get a missing identity returns an error
        let result = repository.get_identity(&identity2.identifier()).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_identities_attributes_repository() -> Result<()> {
        let identity1 = create_identity1().await?;
        let identity2 = create_identity2().await?;
        let attributes = create_attributes_entry().await?;
        let db_file = NamedTempFile::new().unwrap();
        let repository = create_repository(db_file.path()).await?;

        // store attributes
        repository
            .put_attribute_value(
                identity1.identifier(),
                "name".as_bytes().to_vec(),
                "value".as_bytes().to_vec(),
            )
            .await?;
        Ok(())
    }

    /// HELPERS
    async fn create_identity1() -> Result<Identity> {
        let change_history = ChangeHistory::import(&hex::decode("81a201583ba20101025835a4028201815820530d1c2e9822433b679a66a60b9c2ed47c370cd0ce51cbe1a7ad847b5835a96303f4041a64dd4060051a77a94360028201815840042fff8f6c80603fb1cec4a3cf1ff169ee36889d3ed76184fe1dfbd4b692b02892df9525c61c2f1286b829586d13d5abf7d18973141f734d71c1840520d40a0e").unwrap())?;
        Identity::import_from_change_history(None, change_history, Vault::create_verifying_vault())
            .await
    }

    async fn create_identity2() -> Result<Identity> {
        let change_history = ChangeHistory::import(&hex::decode("81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805").unwrap())?;
        Identity::import_from_change_history(None, change_history, Vault::create_verifying_vault())
            .await
    }

    async fn create_attributes_entry() -> Result<AttributesEntry> {
        let identity1 = create_identity1().await?;
        Ok(AttributesEntry::new(
            BTreeMap::from([
                ("name".as_bytes().to_vec(), "alice".as_bytes().to_vec()),
                ("age".as_bytes().to_vec(), "20".as_bytes().to_vec()),
            ]),
            TimestampInSeconds(1000),
            Some(TimestampInSeconds(2000)),
            Some(identity1.identifier().clone()),
        ))
    }

    async fn create_repository(path: &Path) -> Result<Arc<dyn IdentitiesRepository>> {
        let db = SqliteDb::create(path).await?;
        Ok(Arc::new(IdentitiesSqliteRepository::new(Arc::new(db))))
    }
}
