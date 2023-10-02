mod migration;
mod migration_2023_10_02;

pub use migration::{migrate, Migration};

fn all_migrations() -> Vec<Migration> {
    vec![migration_2023_10_02::new()]
}
