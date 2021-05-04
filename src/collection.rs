use crate::myexif::ExifData;
use crate::{models::Modification, models::Photo};
use anyhow::{Context, Result};
use io::Write;
use log::{debug, info};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::{fs, io};
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

    pub fn find_files(
        &self,
        dir: &Path,
        cb: &dyn Fn(&Path),
    ) -> anyhow::Result<()> {
        let absdir = self.basedir.join(dir);
        if fs::metadata(&absdir)?.is_dir() {
            debug!("Should look in {:?}", absdir);
            for entry in fs::read_dir(absdir)? {
                let path = entry?.path();
                if fs::metadata(&path)?.is_dir() {
                    self.find_files(&path, cb)?;
                } else {
                    let subpath =
                        path.strip_prefix(&self.basedir).map_err(|_| {
                            anyhow!(
                                "Directory not in collection: {}",
                                self.basedir.display()
                            )
                        })?;
                    cb(&subpath);
                }
            }
        }
        Ok(())
    }
}

pub struct CollectionManager<'t> {
    basedir: &'t Path,
    conn: &'t diesel::SqliteConnection
}

impl<'t> CollectionManager<'t> {
    pub fn save_photo(&self, contents: &[u8], story_name: &str) -> Result<(String, PathBuf)> {
        let ext = *image::guess_format(contents)?
            .extensions_str()
            .first()
            .unwrap_or(&"image");
        let hash = format!("{:x}", sha2::Sha256::digest(&contents));
        let filename = format!("{}.{}", hash, ext);
        let filename = PathBuf::from(filename);
        let path = self.basedir.join(&filename);
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(contents)?;
        self.index_photo(&filename, Some(contents), story_name)?;
        Ok((hash, filename))
    }

    pub fn index_photo(&self, file_path: &Path, image_bytes: Option<&[u8]>, story_name: &str) -> Result<()> {
        let image_vec = match image_bytes {
            Some(_) => vec![],
            None => std::fs::read(self.basedir.join(file_path))?
        };
        let image_bytes = image_bytes.unwrap_or(&image_vec);
        let exif = ExifData::read_from(&image_bytes).context("Failed reading exif data")?;
        let id = format!("{:x}", sha2::Sha256::digest(&image_bytes));
        let mut thumbnail_buf = std::io::Cursor::new(vec![]);
        let thumbnail = image::load_from_memory(&image_bytes)?
            .thumbnail(256, 256);
        // Rotate if necessary before saving
        let thumbnail = match exif.rotation() {
            Ok(90) | Ok(-270) => thumbnail.rotate90(),
            Ok(180) | Ok(-180)  => thumbnail.rotate180(),
            Ok(270) | Ok(-90) => thumbnail.rotate270(),
            _ => thumbnail
        };
        thumbnail.write_to(&mut thumbnail_buf, image::ImageOutputFormat::Jpeg(80))?;
        match Photo::create_or_set_basics(
            self.conn,
            &id,
            file_path
                .to_str()
                .ok_or(anyhow!("Invalid characters in filename"))?,
            exif.width as i32,
            exif.height as i32,
            exif.dateval,
            exif.rotation()?,
            &thumbnail_buf.into_inner(),
            &story_name
        )? {
            Modification::Created(photo) => {
                info!("Created #{}, {}", photo.id, photo.path);
            }
            Modification::Updated(photo) => {
                info!("Modified {:?}", photo);
            }
            Modification::Unchanged(photo) => {
                debug!("No change for {:?}", photo);
            }
        };
        Ok(())
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