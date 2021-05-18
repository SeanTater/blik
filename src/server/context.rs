use super::Args;
use crate::collection::Collection;
use chrono::{DateTime, Utc};
use rand::prelude::Distribution;
use std::sync::Mutex;

// Does _not_ derive debug, copy or clone, since it contains the jwt
// secret and some connection pools.
pub struct GlobalContext {
    pub collection: Collection,
    // Note for simplicity only one new person can login at a time.
    open_token: Mutex<Option<(u32, DateTime<Utc>)>>,
}

impl GlobalContext {
    pub fn new(args: &Args) -> Self {
        let gc = GlobalContext {
            collection: Collection{
                basedir: args.photos.blik_home
                    .as_ref()
                    .cloned()
                    .or_else(|| std::env::current_dir().ok())
                    .expect("blik_home not specified on the command line, or in the environment, \
                            and the PWD is not accessible either. I have nowhere to store photos.")
            },
            open_token: Mutex::new(None),
        };
        gc
    }

    /// Generate a new login token that expires in a few minutes
    pub fn generate_login_token(&self, ttl_minutes: usize) -> u32 {
        let sampler = rand::distributions::Uniform::new(0, 1000000);
        let code = sampler.sample(&mut rand::thread_rng());
        let ttl_minutes = ttl_minutes.min(24 * 60) as i64 * 60;
        let ttl_minutes = chrono::Duration::minutes(ttl_minutes);
        *self.open_token.lock().unwrap() = Some((
            code,
            chrono::Utc::now().checked_add_signed(ttl_minutes).unwrap(),
        ));
        code
    }

    /// Check and possibly consume the login token
    pub fn use_login_token(&self, code: u32) -> bool {
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