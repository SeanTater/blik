use super::Args;
use crate::dbopt::{SqlitePool, PooledSqlite};
use crate::fetch_places::OverpassOpt;
use crate::photosdir::PhotosDir;
use diesel::r2d2;
use log::{debug, warn};
use medallion::{Header, Payload, Token};
use std::sync::Arc;
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
struct GlobalContext {
    db_pool: SqlitePool,
    photosdir: PhotosDir,
    jwt_secret: String,
    overpass: OverpassOpt,
}

impl GlobalContext {
    fn new(args: &Args) -> Self {
        GlobalContext {
            db_pool: crate::dbopt::create_pool().expect("Sqlite pool"),
            photosdir: PhotosDir::new(&args.photos.photos_dir),
            jwt_secret: args.jwt_key.clone(),
            overpass: args.overpass.clone(),
        }
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
    global: Arc<GlobalContext>,
    path: FullPath,
    user: Option<String>,
}

impl Context {
    pub fn db(&self) -> Result<PooledSqlite, diesel::r2d2::PoolError> {
        Ok(self.global.db_pool.get()?)
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
    pub fn photos(&self) -> &PhotosDir {
        &self.global.photosdir
    }
    pub fn overpass(&self) -> &OverpassOpt {
        &self.global.overpass
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
