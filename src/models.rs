use crate::schema::photos;
use crate::schema::photos::dsl as p;
use crate::schema::thumbnail::dsl as th;
use chrono::naive::NaiveDateTime;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::{Sqlite, SqliteConnection};
use image::{DynamicImage, GenericImageView};
use std::{collections::HashMap,  io::Write, path::Path};
use sha2::Digest;
use anyhow::Result;

#[derive(AsChangeset, Clone, Debug, Identifiable, Insertable, Queryable, Default)]
pub struct Photo {
    pub id: String,
    pub path: String,
    pub date: Option<NaiveDateTime>,
    pub rotation: i16,
    pub is_public: bool,
    pub width: i32,
    pub height: i32,
    pub story: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub caption: Option<String>,
}

impl Photo {
    /// Read Exif data from a basic image, as a reader
    ///
    /// This could be a file or an IO cursor depending on your use case
    pub fn read_from(image_slice: &[u8], story: &str) -> anyhow::Result<Self> {
        use crate::myexif::*;
        use exif::*;

        // Start with an empty photo
        let mut result = Self::default();
        // Fill the basics
        result.id = format!("{:x}", sha2::Sha256::digest(&image_slice));

        // Width and height are different; we always read the image.
        {
            let image = image::load_from_memory(&image_slice)?;
            result.width = image.width() as i32;
            result.height = image.height() as i32;
        }
        let mut cursor = std::io::Cursor::new(image_slice);
        let reader = Reader::new().read_from_container(&mut cursor)?;
        let exif_map: HashMap<Tag, &Field> = reader
            .fields()
            .filter(|f| f.ifd_num == In::PRIMARY)
            .filter_map(|f| Some((f.tag, f)))
            .collect();
        result.date = exif_map.get(&Tag::DateTimeOriginal).and_then(|f| is_datetime(f))
            .or_else(|| is_datetime(exif_map.get(&Tag::DateTime)?))
            .or_else(|| is_datetime(exif_map.get(&Tag::DateTimeDigitized)?));
        result.make = exif_map
            .get(&Tag::Make)
            .and_then(|f| is_string(f));
        result.model = exif_map
            .get(&Tag::Model)
            .and_then(|f| is_string(f));
        result.rotation = exif_map
            .get(&Tag::Orientation)
            .and_then(|f| is_u32(f))
            .unwrap_or(0) as i16;
        result.lat = exif_map
            .get(&Tag::GPSLatitude)
            .and_then(|f| is_lat_long(f));
        result.lon = exif_map
            .get(&Tag::GPSLongitude)
            .and_then(|f| is_lat_long(f));
        result.caption = exif_map
            .get(&Tag::ImageDescription)
            .and_then(|f| is_string(f));
        result.story = story.into();
        

        let ext = *image::guess_format(image_slice)?
            .extensions_str()
            .first()
            .unwrap_or(&"image");
        result.path = match result.date {
            Some(d) => format!("{} {}.{}", d, result.id, ext),
            None => result.id.clone()
        };
        
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn is_public(&self) -> bool {
        self.is_public
    }

    #[allow(dead_code)]
    pub fn query<'a>(auth: bool) -> photos::BoxedQuery<'a, Sqlite> {
        let result = p::photos
            .filter(p::path.not_like("%.CR2"))
            .filter(p::path.not_like("%.dng"))
            .into_boxed();
        if !auth {
            result.filter(p::is_public)
        } else {
            result
        }
    }

    pub fn exists(
        &self,
        db: &SqliteConnection
    ) -> bool {
        p::photos.find(&self.id).first::<Photo>(db).is_ok()
    }

    pub fn save(
        &self,
        db: &SqliteConnection,
        image_slice: &[u8],
        basedir: &Path
    ) -> Result<()> {
        // Actually save the file
        if self.exists(db) {
            bail!("Photo is already indexed.");
        }
        let path = basedir.join(&self.path);
        if path.exists() {
            bail!("Conflict with unindexed photo on disk.");
        }
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(image_slice)?;

        // Create a thumbnail
        let thumbnail = self.create_thumbnail(image_slice)?;
        
        // Index the photo
        diesel::insert_into(th::thumbnail)
            .values((
                th::id.eq(&self.id),
                th::content.eq(thumbnail)
            ))
            .execute(db)?;
        diesel::insert_into(p::photos)
            .values(self)
            .execute(db)?;
        Ok(())
    }

    fn create_thumbnail(&self, image_bytes: &[u8]) -> Result<Vec<u8>> {
        let mut thumbnail_buf = std::io::Cursor::new(vec![]);
        let thumbnail = image::load_from_memory(&image_bytes)?
            .thumbnail(256, 256);
        // Rotate if necessary before saving
        let thumbnail = match self.rotation {
            90 | -270 => thumbnail.rotate90(),
            180 | -180  => thumbnail.rotate180(),
            270 | -90 => thumbnail.rotate270(),
            _ => thumbnail
        };
        thumbnail.write_to(&mut thumbnail_buf, image::ImageOutputFormat::Jpeg(80))?;
        Ok(thumbnail_buf.into_inner())
    }

    #[cfg(test)]
    pub fn mock(y: i32, mo: u32, da: u32, h: u32, m: u32, s: u32) -> Self {
        use chrono::naive::NaiveDate;
        Photo {
            id: format!("{}-{}-{}T{}:{}:{}", y, mo, da, h, m, s),
            path: format!(
                "{}/{:02}/{:02}/IMG{:02}{:02}{:02}.jpg",
                y, mo, da, h, m, s,
            ),
            date: Some(NaiveDate::from_ymd(y, mo, da).and_hms(h, m, s)),
            rotation: 0,
            is_public: false,
            width: 4000,
            height: 3000,
            story: "default".into(),
            lat: None,
            lon: None,
            make: None,
            model: None,
            caption: None
        }
    }
}

pub trait Facet {
    fn by_slug(slug: &str, db: &SqliteConnection) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(Debug, Clone, Queryable)]
pub struct Story {
    pub name: String,
    pub description: String,
    pub created_on: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Queryable)]
pub struct Thumbnail {
    pub id: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Queryable)]
pub struct Annotation {
    pub photo_id: String,
    pub name: String,
    pub top: i64,
    pub bottom: i64,
    pub left: i64,
    pub right: i64,
    pub details: Option<String>
}

pub trait Annotator {
    fn annotate(&self, photo: &Photo, image: &DynamicImage) -> Annotation;
}

struct ExifCaption;
impl Annotator for ExifCaption {
    fn annotate(&self, photo: &Photo, image: &DynamicImage) -> Annotation {
        todo!();
    }
}