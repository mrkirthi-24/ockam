use std::path::Path;

use rusqlite::Connection;
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;

use crate::repository::migrations;
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::tokio::task::JoinError;

pub struct Repository {
    connection: Arc<Mutex<Connection>>,
}

impl Repository {
    /// Constructor
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self> {
        // Not sure we need this
        // creating a new database might be failing a few times
        // if the files are currently being held by another pod which is shutting down.
        // In that case we retry a few times, between 1 and 10 seconds.
        let retry_strategy = FixedInterval::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(10); // limit to 10 retries

        let path: &Path = p.as_ref();
        Retry::spawn(retry_strategy, || async { Self::make(path) }).await
    }

    fn make(path: &Path) -> Result<Self> {
        debug!("create the repository at {}", path.display());
        // Creates database file if it doesn't exist
        let mut connection = Self::create_connection(path)?;
        migrations::migrate(&mut connection)?;
        Ok(Repository {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn create_connection(path: &Path) -> Result<Connection> {
        let connection = Connection::open(path).map_err(Self::map_sqlite_err)?;
        let pragmas = vec![("encoding", "UTF-8")];
        for (pragma_name, pragma_value) in pragmas {
            connection
                .pragma_update(None, pragma_name, pragma_value)
                .map_err(Self::map_sqlite_err)?
        }
        Ok(connection)
    }

    /// Get the current connection
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.connection)
    }

    pub(crate) fn map_join_err(err: JoinError) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    pub(crate) fn map_sqlite_err(err: rusqlite::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }
}
