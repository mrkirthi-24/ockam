use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Params, Row};
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;

use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::tokio::task::JoinError;

use crate::repository::migrations;

pub struct SqliteDb {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteDb {
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
            Self::create_and_migrate(path.as_ref())
        })
        .await
    }

    /// Constructor for an in-memory database
    pub fn in_memory() -> Result<Self> {
        debug!("create an in memory database");
        let mut connection = Self::create_in_memory_connection()?;
        migrations::migrate(&mut connection)?;
        Ok(SqliteDb {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn create_and_migrate(path: &Path) -> Result<Self> {
        debug!("create a database at {}", path.display());
        // Creates database file if it doesn't exist
        let mut connection = Self::create_connection(path)?;
        migrations::migrate(&mut connection)?;
        Ok(SqliteDb {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn create_connection(path: &Path) -> Result<Connection> {
        let connection = Connection::open(path).map_err(Self::map_sqlite_err)?;
        Self::add_pragmas(&connection)?;
        Ok(connection)
    }

    fn create_in_memory_connection() -> Result<Connection> {
        let connection = Connection::open_in_memory().map_err(Self::map_sqlite_err)?;
        Self::add_pragmas(&connection)?;
        Ok(connection)
    }

    fn add_pragmas(connection: &Connection) -> Result<()> {
        let pragmas = vec![("encoding", "UTF-8")];
        for (pragma_name, pragma_value) in pragmas {
            connection
                .pragma_update(None, pragma_name, pragma_value)
                .map_err(Self::map_sqlite_err)?
        }
        Ok(())
    }

    /// Execute a statement
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        let connection = self.connection.lock().unwrap();
        let rows_number = connection
            .execute(sql, params)
            .map_err(Self::map_sqlite_err)?;
        Ok(rows_number)
    }

    /// Query a table
    pub fn query_one<P: Params, R>(
        &self,
        sql: &str,
        params: P,
        from_row: impl FromRow<R>,
    ) -> Result<Option<R>> {
        let connection = self.connection.lock().unwrap();
        connection
            .query_row(sql, params, |r| from_row.make(r))
            .optional()
            .map_err(Self::map_sqlite_err)
    }

    pub(crate) fn map_join_err(err: JoinError) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    pub(crate) fn map_sqlite_err(err: rusqlite::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }
}

pub trait FromRow<T> {
    fn make(&self, row: &Row) -> rusqlite::Result<T>;
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_create_identity_table() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqliteDb::create(db_file.path()).await?;
        let inserted = db.execute(
            "INSERT INTO identity VALUES (?1, ?2)",
            params![
                "Ifa804b7fca12a19eed206ae180b5b576860ae651",
                "123".as_bytes()
            ],
        )?;
        assert_eq!(inserted, 1);
        Ok(())
    }
}
