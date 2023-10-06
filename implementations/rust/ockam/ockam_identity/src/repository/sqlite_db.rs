use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Params, Row, Transaction};
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
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<()> {
        let _ = self.execute_statement(sql, params);
        Ok(())
    }

    /// Execute a statement and return the number of inserted rows
    pub fn execute_statement<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        let connection = self.connection.lock().unwrap();
        let rows_number = connection
            .execute(sql, params)
            .map_err(Self::map_sqlite_err)?;
        Ok(rows_number)
    }

    /// Run some statements within a transaction
    pub fn with_transaction<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&Transaction) -> Result<()>,
    {
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction().map_err(Self::map_sqlite_err)?;
        f(&transaction)?;
        transaction.commit().map_err(Self::map_sqlite_err)
    }

    /// Query a table to get back one entity if it can be found
    /// If the query returns several entities, the entity corresponding to the first row is returned
    pub fn query_maybe_one<P: Params, R>(
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

    /// Query a table to get back one entity
    /// If the query returns no row then an error is raised
    /// If the query returns several entities, the entity corresponding to the first row is returned
    pub fn query_one<P: Params, R>(
        &self,
        sql: &str,
        params: P,
        from_row: impl FromRow<R>,
    ) -> Result<R> {
        let connection = self.connection.lock().unwrap();
        connection
            .query_row(sql, params, |r| from_row.make(r))
            .map_err(Self::map_sqlite_err)
    }

    /// Query a table to get back a list of entities
    pub fn query_all<P: Params, R>(
        &self,
        sql: &str,
        params: P,
        from_row: impl FromRow<R>,
    ) -> Result<Vec<R>> {
        let connection = self.connection.lock().unwrap();
        let mut query = connection.prepare(sql).map_err(Self::map_sqlite_err)?;
        let result: rusqlite::Result<Vec<R>> = query
            .query_map(params, |r| Ok(from_row.make(r)?))
            .map_err(Self::map_sqlite_err)?
            .collect();
        Ok(result.map_err(Self::map_sqlite_err)?)
    }

    pub fn map_join_err(err: JoinError) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    pub fn map_sqlite_err(err: rusqlite::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    pub fn map_decode_err(err: minicbor::decode::Error) -> Error {
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

    /// This is a sanity check to test that the database can be created with a file path
    /// and that migrations are running ok, at least for one table
    #[tokio::test]
    async fn test_create_identity_table() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqliteDb::create(db_file.path()).await?;
        let inserted = db.execute_statement(
            "INSERT INTO identity VALUES (?1, ?2)",
            params![
                "Ifa804b7fca12a19eed206ae180b5b576860ae651",
                "123".as_bytes()
            ],
        )?;
        assert_eq!(inserted, 1);
        Ok(())
    }

    /// This test checks that the query_maybe_one method
    /// returns an optional entity
    #[tokio::test]
    async fn test_query_maybe_one() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqliteDb::create(db_file.path()).await?;
        db.execute(
            "INSERT INTO identity VALUES (?1, ?2)",
            params![
                "Ifa804b7fca12a19eed206ae180b5b576860ae651",
                "123".as_bytes()
            ],
        )?;

        // successful query
        let result: Option<String> = db
            .query_maybe_one(
                "SELECT identifier FROM identity WHERE identifier=?1",
                params!["Ifa804b7fca12a19eed206ae180b5b576860ae651"],
                StringFromRow,
            )
            .unwrap();
        assert_eq!(
            result,
            Some("Ifa804b7fca12a19eed206ae180b5b576860ae651".into())
        );

        // failed query
        let result: Option<String> = db
            .query_maybe_one(
                "SELECT identifier FROM identity WHERE identifier=?1",
                params!["x"],
                StringFromRow,
            )
            .unwrap();
        assert_eq!(result, None);
        Ok(())
    }

    /// This test checks that the query_one method
    /// returns an entity
    #[tokio::test]
    async fn test_query_exactly_one() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqliteDb::create(db_file.path()).await?;
        let insert = |id: &str| {
            db.execute(
                "INSERT INTO identity VALUES (?1, ?2)",
                params![id, "123".as_bytes()],
            )
            .unwrap()
        };
        let ids = vec![
            "Ifa804b7fca12a19eed206ae180b5b576860ae651",
            "Ifa804b7fca12a19eed206ae180b5b576860ae652",
            "Ifa804b7fca12a19eed206ae180b5b576860ae653",
        ];
        for id in ids {
            insert(id)
        }

        // successful query
        let result = db
            .query_one(
                "SELECT identifier FROM identity WHERE identifier=?1",
                params!["Ifa804b7fca12a19eed206ae180b5b576860ae651"],
                StringFromRow,
            )
            .unwrap();
        assert_eq!(
            result,
            "Ifa804b7fca12a19eed206ae180b5b576860ae651".to_string()
        );

        // failed query
        let result = db.query_one(
            "SELECT identifier FROM identity WHERE identifier=?1",
            params!["xxx"],
            StringFromRow,
        );
        assert!(result.is_err());
        Ok(())
    }
}

struct StringFromRow;

impl FromRow<String> for StringFromRow {
    fn make(&self, row: &Row) -> rusqlite::Result<String> {
        row.get(0)
    }
}
