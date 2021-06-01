use crate::models::{Media, Thumbnail};
use anyhow::{Context, Result};
use diesel::SqliteConnection;
use mime::Mime;
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
    pub fn index_media(&self, mime_hint: &Mime, file_bytes: &[u8], story_name: &str) -> Result<Media> {
        if file_bytes.len() == 0 {
            bail!("Uploaded image is empty.");
        }
        match mime_hint.type_() {
            mime::IMAGE => {
                let media = crate::image::read_media_from(&file_bytes, story_name)
                    .context("Failed to read the image's exif data.")?;
                media.save(self.db, file_bytes, self.basedir)?;

                // Create a thumbnail
                crate::image
                    ::create_thumbnail(&media, file_bytes)?
                    .save(self.db)?;
                Ok(media)
            }
            mime::VIDEO => {
                let mut vhandle = crate::video::VideoHandle::open(&file_bytes)?;
                let media = vhandle.read_media(story_name)?;

                // Create a thumbnail
                let thumbnail = vhandle.create_thumbnail(&media)?;
                thumbnail.save(self.db)?;

                // Save *after* everything else worked
                media.save(self.db, file_bytes, self.basedir)?;
                Ok(media)
            }
            _ => Err(anyhow!("Videos are not supported yet"))
        }
    }
}