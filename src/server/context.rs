use super::Args;
use crate::collection::Collection;
use crate::dbopt::{PooledSqlite, SqlitePool};
use chrono::{DateTime, Utc};
use log::{debug, warn};
use medallion::{Header, Payload, Token};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use warp::filters::{cookie, header, BoxedFilter};
use warp::path::{self, FullPath};
use warp::{self, Filter};

pub type ContextFilter = BoxedFilter<(Context,)>;

pub fn create_session_filter(args: &Args) -> ContextFilter {
    let global = Arc::new(GlobalContext::new(args));
    let g1 = global.clone();
    warp::any()
        .and(path::full())
        .and(
            cookie::cookie("EXAUTH")
                .or(header::header("Authorization"))
                .unify()
                .map(move |key: String| {
                    g1.verify_key(&key)
                        .map_err(|e| warn!("Auth failed: {}", e))
                        .ok()
                })
                .or(warp::any().map(|| None))
                .unify(),
        )
        .map(move |path, user| {
            let global = global.clone();
            Context { global, path, user }
        })
        .boxed()
}

// Does _not_ derive debug, copy or clone, since it contains the jwt
// secret and some connection pools.
pub struct GlobalContext {
    db_pool: SqlitePool,
    photosdir: Collection,
    jwt_secret: String,
    // Note for simplicity only one new person can login at a time.
    open_token: Mutex<Option<(u64, DateTime<Utc>)>>,
}

impl GlobalContext {
    fn new(args: &Args) -> Self {
        let pool = crate::dbopt::create_pool().expect("Sqlite pool");
        let gc = GlobalContext {
            db_pool: pool.clone(),
            photosdir: Collection::new(&args.photos.photos_dir, pool),
            jwt_secret: args.jwt_key.clone(),
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

    fn verify_key(&self, jwtstr: &str) -> Result<String, String> {
        let token = Token::<Header, ()>::parse(&jwtstr)
            .map_err(|e| format!("Bad jwt token: {:?}", e))?;

        if !verify_token(&token, self.jwt_secret.as_ref())? {
            return Err(format!("Invalid token {:?}", token));
        }
        let claims = token.payload;
        debug!("Verified token for: {:?}", claims);
        let now = current_numeric_date();
        if let Some(nbf) = claims.nbf {
            if now < nbf {
                return Err(
                    format!("Not-yet valid token, {} < {}", now, nbf,),
                );
            }
        }
        if let Some(exp) = claims.exp {
            if now > exp {
                return Err(format!(
                    "Got an expired token: {} > {}",
                    now, exp,
                ));
            }
        }
        // the claimed sub is the username
        claims
            .sub
            .ok_or_else(|| "User missing in jwt claims".to_string())
    }
}

fn verify_token(
    token: &Token<Header>,
    jwt_secret: &[u8],
) -> Result<bool, String> {
    token
        .verify(jwt_secret)
        .map_err(|e| format!("Failed to verify token {:?}: {}", token, e))
}

/// The request context, providing database, and authorized user.
pub struct Context {
    pub global: Arc<GlobalContext>,
    path: FullPath,
    user: Option<String>,
}

impl Context {
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
        &self.global.photosdir
    }

    pub fn make_token(&self, user: &str) -> Option<String> {
        let header: Header = Default::default();
        let now = current_numeric_date();
        let expiration_time = Duration::from_secs(14 * 24 * 60 * 60);
        let claims = Payload::<()> {
            iss: None, // TODO?
            sub: Some(user.into()),
            exp: Some(now + expiration_time.as_secs()),
            nbf: Some(now),
            ..Default::default()
        };
        let token = Token::new(header, claims);
        token.sign(self.global.jwt_secret.as_ref()).ok()
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
