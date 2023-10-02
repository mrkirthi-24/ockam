use crate::repository::migrations::migration::Migration;

pub(crate) fn new() -> Migration {
    Migration::up("create xxx".into())
}
