//! SQLite connection, migration, and transaction foundations.

mod database;
mod error;
mod migrations;

pub use database::Database;
pub use error::{AppError, MigrationError, PersistenceError};
pub use migrations::{audit_transactional_data, run_migrations, CURRENT_SCHEMA_VERSION};

#[cfg(test)]
mod tests;
