use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::repository::migrations::all_migrations;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};

#[derive(Clone)]
pub struct Migration {
    up_statements: String,
    down_statements: Option<String>,
}

impl Migration {
    pub(crate) fn up(up: String) -> Self {
        Self {
            up_statements: up,
            down_statements: None,
        }
    }

    fn to_sqlite_migration(&self) -> M {
        let up = &self.up_statements;
        let mut m = M::up(up.as_str());
        if let Some(down) = &self.down_statements {
            m = m.down(&down)
        }
        m
    }
}

pub fn migrate(connection: &mut Connection) -> Result<()> {
    let migrations = all_migrations();
    let migrations = Migrations::new_iter(migrations.iter().map(|m| m.to_sqlite_migration()));
    migrations
        .to_latest(connection)
        .map_err(map_sqlite_migration_error)?;
    Ok(())
}

pub(crate) fn map_sqlite_migration_error(err: rusqlite_migration::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}
