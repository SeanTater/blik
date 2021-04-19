use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel::{Connection, ConnectionError};
use log::debug;
use std::{sync::Arc, time::{Duration, Instant}};

pub type SqlitePool = Arc<Pool<ConnectionManager<SqliteConnection>>>;
pub type PooledSqlite = PooledConnection<ConnectionManager<SqliteConnection>>;

pub fn connect() -> Result<SqliteConnection, ConnectionError> {
    let time = Instant::now();
    let db = SqliteConnection::establish("rphotos.db")?;
    debug!("Got db connection in {:?}", time.elapsed());
    Ok(db)
}
pub fn create_pool() -> Result<SqlitePool> {
    let time = Instant::now();
    let pool = Pool::builder()
        .min_idle(Some(2))
        .test_on_check_out(false)
        .connection_timeout(Duration::from_millis(500))
        .build(ConnectionManager::new("rphotos.db"))?;
    debug!("Created pool in {:?}", time.elapsed());
    Ok(Arc::new(pool))
}
