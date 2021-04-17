#![allow(proc_macro_derive_resolution_fallback)]
#![recursion_limit = "128"]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate anyhow;

mod adm;
mod dbopt;
mod fetch_places;
mod models;
mod myexif;
mod photosdir;
mod pidfiles;
mod schema;
mod server;

use crate::adm::stats::show_stats;
use crate::adm::{findphotos, makepublic, storestatics, users};
use crate::dbopt::DbOpt;
use dotenv::dotenv;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use anyhow::Result;

/// Command line interface for rphotos.
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum RPhotos {
    /// Make specific image(s) public.
    ///
    /// The image path(s) are relative to the image root.
    Makepublic(makepublic::Makepublic),
    /// Get place tags for photos by looking up coordinates in OSM
    Fetchplaces(fetch_places::Fetchplaces),
    /// Find new photos in the photo directory
    Findphotos(findphotos::Findphotos),
    /// Show some statistics from the database
    Stats(DbOpt),
    /// Store statics as files for a web server
    Storestatics {
        /// Directory to store the files in
        dir: String,
    },
    /// List existing users
    Userlist {
        #[structopt(flatten)]
        db: DbOpt,
    },
    /// Set password for a (new or existing) user
    Userpass {
        #[structopt(flatten)]
        db: DbOpt,
        /// Username to set password for
        // TODO: Use a special type that only accepts nice user names.
        user: String,
    },
    /// Run the rphotos web server.
    Runserver(server::Args),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct DirOpt {
    /// Path to the root directory storing all actual photos.
    #[structopt(long, env = "RPHOTOS_DIR")]
    photos_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    match run(&RPhotos::from_args()).await {
        Ok(()) => (),
        Err(err) => {
            println!("{}", err);
            exit(1);
        }
    }
}

async fn run(args: &RPhotos) -> Result<()> {
    match args {
        RPhotos::Findphotos(cmd) => cmd.run(),
        RPhotos::Makepublic(cmd) => cmd.run(),
        RPhotos::Stats(db) => show_stats(&db.connect()?),
        RPhotos::Userlist { db } => users::list(&db.connect()?),
        RPhotos::Userpass { db, user } => users::passwd(&db.connect()?, user),
        RPhotos::Fetchplaces(cmd) => cmd.run().await,
        RPhotos::Storestatics { dir } => storestatics::to_dir(dir),
        RPhotos::Runserver(ra) => server::run(ra).await,
    }
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
