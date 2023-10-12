use sqlx::*;

use ockam_core::async_trait;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use crate::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType};
use crate::identity::IdentityConstants;
use crate::models::{Identifier, PurposeKeyAttestation};
use crate::purpose_keys::storage::{PurposeKeysReader, PurposeKeysRepository, PurposeKeysWriter};
use crate::Purpose;

/// Storage for own [`super::super::super::purpose_key::PurposeKey`]s
#[derive(Clone)]
pub struct PurposeKeysSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

#[async_trait]
impl PurposeKeysRepository for PurposeKeysSqlxDatabase {
    fn as_reader(&self) -> Arc<dyn PurposeKeysReader> {
        Arc::new(self.clone())
    }

    fn as_writer(&self) -> Arc<dyn PurposeKeysWriter> {
        Arc::new(self.clone())
    }
}

impl PurposeKeysSqlxDatabase {
    /// Create a new database for purpose keys
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        Self { database }
    }

    /// Create a new in-memory database for purpose keys
    pub fn create() -> Arc<Self> {
        todo!("implement the in-memory version of the purpose keys database")
    }
}

#[async_trait]
impl PurposeKeysWriter for PurposeKeysSqlxDatabase {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO purpose_key VALUES (?, ?, ?, ?, ?)")
            .bind(subject.to_sql())
            .bind(purpose.to_sql())
            .bind(minicbor::to_vec(purpose_key_attestation)?.to_sql());
        query
            .execute(&self.database.pool)
            .await
            .map(|_| ())
            .into_core()
    }

    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()> {
        let query = query("DELETE FROM purpose_key WHERE identifier = ? and purpose = ?")
            .bind(subject.to_sql())
            .bind(purpose.to_sql());
        query
            .execute(&self.database.pool)
            .await
            .map(|_| ())
            .into_core()
    }
}

#[async_trait]
impl PurposeKeysReader for PurposeKeysSqlxDatabase {
    async fn retrieve_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>> {
        let query = query_as("SELECT * FROM purpose_key WHERE identifier=$1 and purpose=$2")
            .bind(identifier.to_sql())
            .bind(purpose.to_sql());
        let row: Option<PurposeKeyRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.purpose_key_attestation()).transpose()?)
    }
}

#[derive(FromRow)]
pub(crate) struct PurposeKeyRow {
    // The identifier who is using this key
    identifier: String,
    // Purpose of the key (signing, encrypting, etc...)
    purpose: String,
    // Attestation that this key is valid
    purpose_key_attestation: Vec<u8>,
}

impl PurposeKeyRow {
    fn purpose_key_attestation(&self) -> Result<PurposeKeyAttestation> {
        Ok(minicbor::decode(self.purpose_key_attestation.as_slice())?)
    }
}

impl ToSqlxType for Purpose {
    fn to_sql(&self) -> SqlxType {
        match self {
            Purpose::SecureChannel => {
                SqlxType::Text(IdentityConstants::SECURE_CHANNEL_PURPOSE_KEY.to_string())
            }
            Purpose::Credentials => {
                SqlxType::Text(IdentityConstants::CREDENTIALS_PURPOSE_KEY.to_string())
            }
        }
    }
}
