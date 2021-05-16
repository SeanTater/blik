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
mod template_utils;

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
enum Blik {
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
    #[structopt(long, env = "BLIK_HOME")]
    blik_home: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    match run(&Blik::from_args()).await {
        Ok(()) => (),
        Err(err) => {
            println!("{}", err);
            exit(1);
        }
    }
}

async fn run(args: &Blik) -> Result<()> {
    match args {
        Blik::Makepublic(cmd) => cmd.run(),
        Blik::Stats => show_stats(&dbopt::connect()?),
        Blik::Runserver(ra) => server::run(ra).await,
    }
}

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
