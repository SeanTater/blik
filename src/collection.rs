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

#[derive(Debug)]
pub enum ImageLoadFailed {
    File(io::Error),
    Image(image::ImageError),
    Join(JoinError),
}

impl std::error::Error for ImageLoadFailed {}

impl std::fmt::Display for ImageLoadFailed {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ImageLoadFailed::File(e) => e.fmt(out),
            ImageLoadFailed::Image(e) => e.fmt(out),
            ImageLoadFailed::Join(e) => e.fmt(out),
        }
    }
}

impl From<io::Error> for ImageLoadFailed {
    fn from(e: io::Error) -> ImageLoadFailed {
        ImageLoadFailed::File(e)
    }
}
impl From<image::ImageError> for ImageLoadFailed {
    fn from(e: image::ImageError) -> ImageLoadFailed {
        ImageLoadFailed::Image(e)
    }
}
impl From<JoinError> for ImageLoadFailed {
    fn from(e: JoinError) -> ImageLoadFailed {
        ImageLoadFailed::Join(e)
    }
}