#![feature(proc_macro_hygiene, decl_macro)]
#![allow(proc_macro_derive_resolution_fallback)]
#![recursion_limit = "128"]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate anyhow;

mod adm;
mod collection;
mod dbopt;
mod models;
mod myexif;
mod schema;
mod server;

use crate::adm::stats::show_stats;
use crate::adm::makepublic;
use anyhow::Result;
use dotenv::dotenv;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

/// Command line interface for rphotos.
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum RPhotos {
    /// Make specific image(s) public.
    ///
    /// The image path(s) are relative to the image root.
    Makepublic(makepublic::Makepublic),
    /// Show some statistics from the database
    Stats,
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
        RPhotos::Makepublic(cmd) => cmd.run(),
        RPhotos::Stats => show_stats(&dbopt::connect()?),
        RPhotos::Runserver(ra) => server::run(ra).await,
    }
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
