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
    /// Read Exif data from a basic image stored compressed in memory as a slice
    ///
    /// Images and videos are handled a bit differently, and this is a convenience method that
    /// in turn calls the image-specific annotation pipeline.
    pub fn read_from_image(image_slice: &[u8], story: &str) -> anyhow::Result<Self> {
        crate::image::read_media_from(image_slice, story)
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

        diesel::insert_into(p::media)
            .values(self)
            .execute(db)?;
        Ok(())
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

impl Thumbnail {
    pub fn save(&self, db: &SqliteConnection) -> Result<()> {
        // Index the media
        diesel::insert_into(th::thumbnail)
            .values((
                th::id.eq(&self.id),
                th::content.eq(&self.content)
            ))
            .execute(db)?;
        Ok(())
    }
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