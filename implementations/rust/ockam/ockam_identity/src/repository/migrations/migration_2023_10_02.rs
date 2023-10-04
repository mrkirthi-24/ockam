use crate::repository::migrations::migration::Migration;

pub(crate) fn new() -> Migration {
    Migration::up(create_identity_table())
}

fn create_identity_table() -> String {
    r#"
CREATE TABLE identity (
  identifier TEXT,
  change_history BLOB
);
    "#
    .into()
}
