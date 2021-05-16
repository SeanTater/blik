use anyhow::Result;
use diesel::sqlite::SqliteConnection;
use diesel::{Connection, ConnectionError};

pub fn connect() -> Result<SqliteConnection, ConnectionError> {
    let time = std::time::Instant::now();
    let db = SqliteConnection::establish("blik.db")?;
    log::debug!("Got db connection in {:?}", time.elapsed());
    Ok(db)
}
