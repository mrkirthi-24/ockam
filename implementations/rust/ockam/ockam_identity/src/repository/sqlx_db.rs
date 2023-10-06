use core::str::FromStr;
use std::ops::Deref;
use std::path::Path;

use sqlx::sqlite::SqliteRow;
use sqlx::{Database, Encode, FromRow, Row, SqlitePool, Type};
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::tokio::task::JoinError;

use crate::Identifier;

/// We use sqlx as our primary interface for interacting with the database
/// The database driver is currently Sqlite
pub struct SqlxDb {
    pub pool: SqlitePool,
}

impl Deref for SqlxDb {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl SqlxDb {
    /// Constructor for a database persisted on disk
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Not sure we need this
        // creating a new database might be failing a few times
        // if the files are currently being held by another pod which is shutting down.
        // In that case we retry a few times, between 1 and 10 seconds.
        let retry_strategy = FixedInterval::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(10); // limit to 10 retries

        Retry::spawn(retry_strategy, || async {
            Self::create_and_migrate(path.as_ref()).await
        })
        .await
    }

    /// Constructor for an in-memory database
    pub async fn in_memory() -> Result<Self> {
        debug!("create an in memory database");
        let pool = Self::create_in_memory_connection_pool().await?;
        let db = SqlxDb { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn create_and_migrate(path: &Path) -> Result<Self> {
        debug!("create a database at {}", path.display());
        // Creates database file if it doesn't exist
        let pool = Self::create_connection_pool(path).await?;
        let db = SqlxDb { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn create_connection_pool(path: &Path) -> Result<SqlitePool> {
        let connection = SqlitePool::connect(path.to_str().unwrap())
            .await
            .map_err(Self::map_sql_err)?;
        Ok(connection)
    }

    async fn create_in_memory_connection_pool() -> Result<SqlitePool> {
        let pool = SqlitePool::connect("file::memory:")
            .await
            .map_err(Self::map_sql_err)?;
        Ok(pool)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./src/repository/db_migrations")
            .run(&self.pool)
            .await
            .map_err(Self::map_migrate_err)
    }

    pub fn map_sql_err(err: sqlx::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    pub fn map_migrate_err(err: sqlx::migrate::MigrateError) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identifier;
    use core::str::FromStr;
    use tempfile::NamedTempFile;

    /// This is a sanity check to test that the database can be created with a file path
    /// and that migrations are running ok, at least for one table
    #[tokio::test]
    async fn test_create_identity_table() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDb::create(db_file.path()).await?;
        let inserted = sqlx::query("INSERT INTO identity VALUES (?1, ?2)")
            .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
            .bind("123".as_bytes())
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(inserted.rows_affected(), 1);
        Ok(())
    }

    /// This test checks that we can run a query and return an entity
    #[tokio::test]
    async fn test_query() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDb::create(db_file.path()).await?;
        sqlx::query("INSERT INTO identity VALUES (?1, ?2)")
            .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
            .bind("123".as_bytes())
            .execute(&db.pool)
            .await
            .unwrap();

        // successful query
        let result: Option<Identifier> =
            sqlx::query_as("SELECT identifier FROM identity WHERE identifier=?1")
                .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
                .fetch_optional(&db.pool)
                .await
                .unwrap();
        assert_eq!(
            result,
            Some(Identifier::from_str("Ifa804b7fca12a19eed206ae180b5b576860ae651").unwrap())
        );

        // failed query
        let result: Option<Identifier> =
            sqlx::query_as("SELECT identifier FROM identity WHERE identifier=?1")
                .bind("x")
                .fetch_optional(&db.pool)
                .await
                .unwrap();
        assert_eq!(result, None);
        Ok(())
    }
}

impl FromRow<'_, SqliteRow> for Identifier {
    fn from_row(row: &SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        Identifier::from_str(row.get("identifier")).map_err(|e| sqlx::Error::ColumnDecode {
            index: "identifier".to_string(),
            source: e.into(),
        })
    }
}
