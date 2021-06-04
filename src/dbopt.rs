use anyhow::Result;
use diesel::sqlite::SqliteConnection;
use diesel::{Connection, ConnectionError};

embed_migrations!();

pub fn connect() -> Result<SqliteConnection, ConnectionError> {
    let time = std::time::Instant::now();
    let db = SqliteConnection::establish("blik.db")?;
    log::debug!("Got db connection in {:?}", time.elapsed());
    Ok(db)
}

pub fn run_migrations() -> Result<()> {
    log::warn!("Running database migrations");
    embedded_migrations::run(&connect()?)?;
    Ok(())
}