use super::Args;
use crate::collection::Collection;
use chrono::{DateTime, Utc};
use std::sync::Mutex;

// Does _not_ derive debug, copy or clone, since it contains the jwt
// secret and some connection pools.
pub struct GlobalContext {
    pub collection: Collection,
    // Note for simplicity only one new person can login at a time.
    open_token: Mutex<Option<(u64, DateTime<Utc>)>>,
}

impl GlobalContext {
    pub fn new(args: &Args) -> Self {
        let pool = crate::dbopt::create_pool().expect("Sqlite pool");
        let gc = GlobalContext {
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