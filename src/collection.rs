use crate::models::{Media, Thumbnail};
use anyhow::{Context, Result};
use diesel::SqliteConnection;
use std::path::{Path, PathBuf};

pub struct Collection {
    pub basedir: PathBuf,
}
impl Collection {
    pub fn manage<'t>(&'t self, db: &'t diesel::SqliteConnection) -> CollectionManager<'t> {
        CollectionManager {
            basedir: &self.basedir,
            db
        }
    }

    pub fn get_raw_path(&self, media: &Media) -> PathBuf {
        self.basedir.join(&media.path)
    }
}

pub struct CollectionManager<'t> {
    basedir: &'t Path,
    db: &'t diesel::SqliteConnection
}

impl<'t> CollectionManager<'t> {
    pub fn index_media(&self, image_slice: &[u8], story_name: &str) -> Result<Media> {
        if image_slice.len() == 0 {
            bail!("Uploaded image is empty.");
        }
        let media = Media::read_from_image(&image_slice, story_name)
            .context("Failed to read the image's exif data.")?;
        media.save(self.db, image_slice, self.basedir)?;

        // Create a thumbnail
        crate::image
            ::create_thumbnail(&media, image_slice)?
            .save(self.db)?;
        Ok(media)
    }
}