use anyhow::Result;
use crate::DbOpt;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
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
    #[structopt(flatten)]
    db: DbOpt,
    /// Image path to make public
    #[structopt(group = "spec")]
    image: Option<String>,
    /// File listing image paths to make public
    #[structopt(long, short, group = "spec")]
    list: Option<String>,
    /// Make all images with matching tag public.
    ///
    /// The tag is specified by its slug.
    #[structopt(long, short, group = "spec")]
    tag: Option<String>,
}

impl Makepublic {
    pub fn run(&self) -> Result<()> {
        let db = self.db.connect()?;
        match (
            self.list.as_ref().map(AsRef::as_ref),
            &self.tag,
            &self.image,
        ) {
            (Some("-"), None, None) => {
                let list = io::stdin();
                by_file_list(&db, list.lock())?;
                Ok(())
            }
            (Some(list), None, None) => {
                let list = BufReader::new(File::open(list)?);
                by_file_list(&db, list)
            }
            (None, Some(tag), None) => {
                use crate::schema::photo_tags::dsl as pt;
                use crate::schema::photos::dsl as p;
                use crate::schema::tags::dsl as t;
                let n = update(
                    p::photos.filter(
                        p::id.eq_any(
                            pt::photo_tags
                                .select(pt::photo_id)
                                .left_join(t::tags)
                                .filter(t::slug.eq(tag)),
                        ),
                    ),
                )
                .set(p::is_public.eq(true))
                .execute(&db)?;
                println!("Made {} images public.", n);
                Ok(())
            }
            (None, None, Some(image)) => one(&db, image),
            _ => Err(anyhow!("bad command")),
        }
    }
}

pub fn one(db: &SqliteConnection, tpath: &str) -> Result<()> {
    use crate::schema::photos::dsl::*;
    match update(photos.filter(path.eq(&tpath)))
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
