use crate::models::Media;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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

    pub fn get_raw_path(&self, media: &Media) -> PathBuf {
        self.basedir.join(&media.path)
    }
}

pub struct CollectionManager<'t> {
    basedir: &'t Path,
    conn: &'t diesel::SqliteConnection
}

impl<'t> CollectionManager<'t> {
    pub fn index_media(&self, image_slice: &[u8], story_name: &str) -> Result<Media> {
        let pho = Media::read_from(&image_slice, story_name).context("Failed reading exif data")?;
        pho.save(&self.conn, image_slice, self.basedir)?;
        Ok(pho)
    }
}