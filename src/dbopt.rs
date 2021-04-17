use anyhow::Result;
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{Connection, ConnectionError};
use log::debug;
use std::time::{Duration, Instant};
use structopt::StructOpt;

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;
pub type PooledSqlite = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct DbOpt {
    /// How to connect to the postgres database.
    #[structopt(long, env = "DATABASE_URL", hide_env_values = true)]
    db_url: String,
}

impl DbOpt {
    pub fn connect(&self) -> Result<SqliteConnection, ConnectionError> {
        let time = Instant::now();
        let db = SqliteConnection::establish(&self.db_url)?;
        debug!("Got db connection in {:?}", time.elapsed());
        Ok(db)
    }
    pub fn create_pool(&self) -> Result<SqlitePool> {
        let time = Instant::now();
        let pool = Pool::builder()
            .min_idle(Some(2))
            .test_on_check_out(false)
            .connection_timeout(Duration::from_millis(500))
            .build(ConnectionManager::new(&self.db_url))?;
        debug!("Created pool in {:?}", time.elapsed());
        Ok(pool)
    }
}
