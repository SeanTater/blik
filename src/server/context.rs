use super::Args;
use crate::collection::Collection;
use crate::dbopt::{PooledSqlite, SqlitePool};
use chrono::{DateTime, Utc};
use log::{debug, warn};
use medallion::{Header, Payload, Token};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use warp::filters::BoxedFilter;
use warp::{self, Filter};

pub type ContextFilter = BoxedFilter<(Context,)>;

// Does _not_ derive debug, copy or clone, since it contains the jwt
// secret and some connection pools.
pub struct GlobalContext {
    db_pool: SqlitePool,
    pub collection: Collection,
    // Note for simplicity only one new person can login at a time.
    open_token: Mutex<Option<(u64, DateTime<Utc>)>>,
}

impl GlobalContext {
    pub fn new(args: &Args) -> Self {
        let pool = crate::dbopt::create_pool().expect("Sqlite pool");
        let gc = GlobalContext {
            db_pool: pool.clone(),
            collection: Collection::new(&args.photos.photos_dir, pool),
            open_token: Mutex::new(None),
        };
        let code = gc.generate_login_token(15);
        println!("You can login with code {} in the next 15 minutes", code);
        gc
    }

    /// Generate a new login token that expires in a few minutes
    pub fn generate_login_token(&self, ttl_minutes: usize) -> u64 {
        let code = rand::random();
        let ttl_minutes = ttl_minutes.min(24 * 60) as i64 * 60;
        let ttl_minutes = chrono::Duration::minutes(ttl_minutes);
        *self.open_token.lock().unwrap() = Some((
            code,
            chrono::Utc::now().checked_add_signed(ttl_minutes).unwrap(),
        ));
        code
    }

    /// Check and possibly consume the login token
    pub fn use_login_token(&self, code: u64) -> bool {
        let success = match *self.open_token.lock().unwrap() {
            None => false,
            Some((correct, expiration)) => {
                code == correct && chrono::Utc::now() < expiration
            }
        };
        if success {
            // Expire token
            *self.open_token.lock().unwrap() = None;
        }
        success
    }
}

/// The request context, providing database, and authorized user.
pub struct Context {
    pub global: Arc<GlobalContext>,
    path: String,
    user: Option<String>,
}

impl Context {
    pub fn new(global: Arc<GlobalContext>) -> Context {
        Context{ global, path: "".into(), user: None}
    }
    pub fn db(&self) -> PooledSqlite {
        self.global
            .db_pool
            .get()
            .expect("Failed to connect ot database")
    }
    pub fn db_pool(&self) -> SqlitePool {
        self.global.db_pool.clone()
    }
    pub fn authorized_user(&self) -> Option<&str> {
        self.user.as_ref().map(AsRef::as_ref)
    }
    pub fn is_authorized(&self) -> bool {
        self.user.is_some()
    }
    pub fn path_without_query(&self) -> &str {
        self.path.as_str()
    }
    pub fn photos(&self) -> &Collection {
        &self.global.collection
    }
}

/// Get the current value for jwt NumericDate.
///
/// Defined in RFC 7519 section 2 to be equivalent to POSIX.1 "Seconds
/// Since the Epoch".  The RFC allows a NumericDate to be non-integer
/// (for sub-second resolution), but the jwt crate uses u64.
fn current_numeric_date() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
