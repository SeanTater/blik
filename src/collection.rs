use crate::models::Photo;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::io;
use tokio::task::JoinError;

pub struct Collection {
    pub basedir: PathBuf,
}
impl Collection {
    pub fn manage<'t>(&'t self, conn: &'t diesel::SqliteConnection) -> CollectionManager<'t> {
        CollectionManager {
            basedir: &self.basedir,
            conn
        }
    }

    pub fn get_raw_path(&self, photo: &Photo) -> PathBuf {
        self.basedir.join(&photo.path)
    }
}

pub struct CollectionManager<'t> {
    basedir: &'t Path,
    conn: &'t diesel::SqliteConnection
}

impl<'t> CollectionManager<'t> {
    pub fn index_photo(&self, image_slice: &[u8], story_name: &str) -> Result<Photo> {
        let pho = Photo::read_from(&image_slice, story_name).context("Failed reading exif data")?;
        pho.save(&self.conn, image_slice, self.basedir)?;
        Ok(pho)
    }
}