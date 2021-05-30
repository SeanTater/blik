use crate::schema::{media, story, annotation, thumbnail};
use crate::schema::media::dsl as p;
use crate::schema::thumbnail::dsl as th;
use chrono::naive::NaiveDateTime;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use image::{DynamicImage, GenericImageView};
use std::{collections::HashMap, io::Write, path::Path};
use sha2::Digest;
use anyhow::Result;
#[derive(AsChangeset, Clone, Debug, Identifiable, Insertable, Queryable, QueryableByName, Default)]
#[table_name = "media"]
pub struct Media {
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

impl Media {
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
        let exif_map = match Reader::new().read_from_container(&mut cursor) {
            Ok(ex) => ex
                .fields()
                .filter(|f| f.ifd_num == In::PRIMARY)
                .filter_map(|f| Some((f.tag, f.clone())))
                .collect(),
            Err(x) => {
                log::warn!("Couldn't read EXIF: {}", x);
                HashMap::new()
            }
        };
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
            .map(|value| match value {
                // EXIF has a funny way of encoding rotations
                3 => 180,
                6 => 90,
                8 => 270,
                1 | 0 | _ => 0,
            })
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

    pub fn exists(
        &self,
        db: &SqliteConnection
    ) -> bool {
        p::media.find(&self.id).first::<Media>(db).is_ok()
    }

    pub fn save(
        &self,
        db: &SqliteConnection,
        image_slice: &[u8],
        basedir: &Path
    ) -> Result<()> {
        // Actually save the file
        if self.exists(db) {
            bail!("Photo/Video is already indexed.");
        }
        let path = basedir.join(&self.path);
        if path.exists() {
            bail!("Conflict with unindexed photo/video on disk.");
        }
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(image_slice)?;

        // Create a thumbnail
        let thumbnail = self.create_thumbnail(image_slice)?;
        
        // Index the media
        diesel::insert_into(th::thumbnail)
            .values((
                th::id.eq(&self.id),
                th::content.eq(thumbnail)
            ))
            .execute(db)?;
        diesel::insert_into(p::media)
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
        Media {
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

#[derive(Clone, Debug, Insertable, Queryable, QueryableByName)]
#[table_name = "story"]
pub struct Story {
    pub name: String,
    pub title: String,
    pub description: String,
    pub created_on: NaiveDateTime,
    pub last_updated: NaiveDateTime,
    pub latest_media: Option<String>,
    pub media_count: i32
}
impl Story {
    /// Create a new story by name and description with otherwise default attributes
    /// This doesn't save the story to a database
    pub fn new(name: String, title: String, description: String) -> Story {
        let now = chrono::Local::now().naive_local();
        Story {
            name, title, description,
            created_on: now,
            last_updated: now,
            latest_media: None,
            media_count: 0
        }
    }

    /// Save this story to the database.
    pub fn save(&self, db: &SqliteConnection) -> anyhow::Result<()> {
        use crate::schema::story::dsl as st;
        diesel::insert_or_ignore_into(st::story)
            .values(self)
            .execute(db)?;
        Ok(())
    }

    /// Get all the stories from the database
    pub fn all(db: &SqliteConnection) -> anyhow::Result<Vec<Story>> {
        use crate::schema::story::dsl as st;
        let stories = st::story
            .order_by(st::created_on.desc())
            .load(db)?;
        Ok(stories)
    }

    /// Get the stories associated with just one year
    pub fn by_year(db: &SqliteConnection, year: i32) -> anyhow::Result<Vec<Story>> {
        use crate::schema::story::dsl as st;
        let stories = st::story
            .filter(st::created_on.ge(year.to_string()))
            .filter(st::created_on.lt((year+1).to_string()))
            .order_by(st::created_on.desc())
            .load(db)?;
        Ok(stories)
    }

    /// Get the most recent 20 stories from the database
    pub fn recent(db: &SqliteConnection, year: Option<i32>) -> anyhow::Result<Vec<Story>> {
        use crate::schema::story::dsl as st;

        let stories = match year {
            None => st::story
                .order_by(st::created_on.desc())
                .limit(20)
                .load(db)?,
            Some(year) => {
                let start_date = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
                    .ok_or(anyhow!("Out of range year"))?
                    .and_hms(0, 0, 0);
                let end_date = chrono::NaiveDate::from_ymd_opt(year+1, 1, 1)
                    .ok_or(anyhow!("Out of range year"))?
                    .and_hms(0, 0, 0);
                st::story
                    .filter(st::created_on.ge(start_date))
                    .filter(st::created_on.lt(end_date))
                    .order_by(st::created_on.desc())
                    .load(db)?
            }
        };
        Ok(stories)
    }

    /// Get media related to this story
    pub fn related_media(&self, db: &SqliteConnection) -> anyhow::Result<Vec<Media>> {
        let media = p::media
            .filter(p::story.eq(&self.name))
            .load(db)?;
        Ok(media)
    }

    /// Check if there is a story by that name (for uploading media attached to it)
    pub fn by_name(db: &SqliteConnection, name: &str) -> anyhow::Result<Story> {
        use crate::schema::story::dsl as st;
        Ok(st::story.find(name).first::<Story>(db)?)
    }
}

#[derive(Debug, Clone, Queryable, QueryableByName)]
#[table_name = "thumbnail"]
pub struct Thumbnail {
    pub id: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Queryable, QueryableByName)]
#[table_name = "annotation"]
pub struct Annotation {
    pub media_id: String,
    pub name: String,
    pub top: i64,
    pub bottom: i64,
    pub left: i64,
    pub right: i64,
    pub details: Option<String>
}

pub trait Annotator {
    fn annotate(&self, media: &Media, image: &DynamicImage) -> Annotation;
}

struct ExifCaption;
impl Annotator for ExifCaption {
    fn annotate(&self, media: &Media, image: &DynamicImage) -> Annotation {
        todo!();
    }
}