use crate::myexif::ExifData;
use crate::{dbopt, models::Modification, models::Photo};
use anyhow::{Context, Result};
use diesel::{insert_into, ExpressionMethods, QueryDsl, RunQueryDsl};
use image::{self, GenericImageView, ImageError};
use io::Write;
use log::{debug, info, warn};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::task::JoinError;

pub struct Collection {
    basedir: PathBuf,
    pool: dbopt::SqlitePool,
}

impl Collection {
    pub fn new(basedir: &Path, pool: dbopt::SqlitePool) -> Self {
        Collection {
            basedir: basedir.into(),
            pool,
        }
    }

    pub fn get_raw_path(&self, photo: &Photo) -> PathBuf {
        self.basedir.join(&photo.path)
    }

    pub fn save_photo(&self, contents: &[u8]) -> Result<(String, PathBuf)> {
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
        self.index_photo(&filename)?;
        Ok((hash, filename))
    }

    pub fn index_photo(&self, file_path: &Path) -> Result<()> {
        let ref db = self.pool.get()?;
        let image_bytes = std::fs::read(self.basedir.join(file_path))?;
        let exif =
            load_exif(&image_bytes).context("Failed reading exif data")?;
        let width = exif.width.ok_or(anyhow!("Missing width"))?;
        let height = exif.height.ok_or(anyhow!("Missing height"))?;
        let id = format!("{:x}", sha2::Sha256::digest(&image_bytes));
        let mut thumbnail = std::io::Cursor::new(vec![]);
        image::load_from_memory(&image_bytes)?
            .thumbnail(256, 256)
            .write_to(&mut thumbnail, image::ImageOutputFormat::Jpeg(80))?;
        let photo = match Photo::create_or_set_basics(
            db,
            &id,
            file_path
                .to_str()
                .ok_or(anyhow!("Invalid characters in filename"))?,
            width as i32,
            height as i32,
            exif.date(),
            exif.rotation()?,
            &thumbnail.into_inner(),
        )? {
            Modification::Created(photo) => {
                info!("Created #{}, {}", photo.id, photo.path);
                photo
            }
            Modification::Updated(photo) => {
                info!("Modified {:?}", photo);
                photo
            }
            Modification::Unchanged(photo) => {
                debug!("No change for {:?}", photo);
                photo
            }
        };
        if let Some((lat, long)) = exif.position() {
            debug!("Position for {} is {} {}", file_path.display(), lat, long);
            use crate::schema::positions::dsl::*;
            if let Ok((clat, clong)) = positions
                .filter(photo_id.eq(&photo.id))
                .select((latitude, longitude))
                .first::<(i32, i32)>(db)
            {
                let lat = (lat * 1e6) as i32;
                let long = (long * 1e6) as i32;
                if clat != lat || clong != long {
                    warn!(
                        "Photo #{}: {}: \
                         Exif position {}, {} differs from saved {}, {}",
                        photo.id, photo.path, clat, clong, lat, long,
                    );
                }
            } else {
                info!(
                    "Position for {} is {} {}",
                    file_path.display(),
                    lat,
                    long
                );
                insert_into(positions)
                    .values((
                        photo_id.eq(&photo.id),
                        latitude.eq((lat * 1e6) as i32),
                        longitude.eq((long * 1e6) as i32),
                    ))
                    .execute(db)
                    .context("Insert image position")?;
            }
        }
        Ok(())
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

/// Read EXIF data from a slice
///
/// This could be done with a reader but there's no reason to because we need
/// to read the whole file for the hash anyway, and I think it's safe to assume
/// it's smaller than memory
fn load_exif(slice: &[u8]) -> Result<ExifData> {
    let reader = std::io::Cursor::new(slice);
    if let Ok(mut exif) = ExifData::read_from(reader) {
        if exif.width.is_none() || exif.height.is_none() {
            if let Ok((width, height)) = actual_image_size(slice) {
                exif.width = Some(width);
                exif.height = Some(height);
            }
        }
        Ok(exif)
    } else if let Ok((width, height)) = actual_image_size(slice) {
        let mut meta = ExifData::default();
        meta.width = Some(width);
        meta.height = Some(height);
        Ok(meta)
    } else {
        Err(anyhow!("Couldn't read Exif data"))
    }
}

fn actual_image_size(image_slice: &[u8]) -> Result<(u32, u32), ImageError> {
    let image = image::load_from_memory(&image_slice)?;
    Ok((image.width(), image.height()))
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