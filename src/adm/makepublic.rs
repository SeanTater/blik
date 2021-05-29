use anyhow::Result;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use diesel::update;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use structopt::clap::ArgGroup;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
#[structopt(group = ArgGroup::with_name("spec").required(true))]
pub struct Makepublic {
    /// Image path to make public
    #[structopt(group = "spec")]
    image: Option<String>,
    /// File listing image paths to make public
    #[structopt(long, short, group = "spec")]
    list: Option<String>,
}

impl Makepublic {
    pub fn run(&self) -> Result<()> {
        let db = crate::dbopt::connect()?;
        match (
            self.list.as_ref().map(AsRef::as_ref),
            &self.image,
        ) {
            (Some("-"), None) => {
                let list = io::stdin();
                by_file_list(&db, list.lock())?;
                Ok(())
            }
            (Some(list), None) => {
                let list = BufReader::new(File::open(list)?);
                by_file_list(&db, list)
            }
            (None, Some(image)) => one(&db, image),
            _ => Err(anyhow!("bad command")),
        }
    }
}

pub fn one(db: &SqliteConnection, tpath: &str) -> Result<()> {
    use crate::schema::media::dsl::*;
    match update(media.filter(path.eq(&tpath)))
        .set(is_public.eq(true))
        .execute(db)
    {
        Ok(count) => {
            println!("Made {} photos public, at {}", count, tpath);
            Ok(())
        }
        Err(DieselError::NotFound) => {
            Err(anyhow!("File {} is not known", tpath))
        }
        Err(error) => Err(error.into()),
    }
}

pub fn by_file_list<In: BufRead + Sized>(
    db: &SqliteConnection,
    list: In,
) -> Result<()> {
    for line in list.lines() {
        one(db, &line?)?;
    }
    Ok(())
}
