use crate::myexif::ExifData;
use crate::schema::photos;
use crate::schema::photos::dsl as p;
use crate::schema::thumbnail::dsl as th;
use chrono::{naive::NaiveDateTime, Datelike};
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::{Sqlite, SqliteConnection};
use image::DynamicImage;

#[derive(AsChangeset, Clone, Debug, Identifiable, Queryable)]
pub struct Photo {
    pub id: String,
    pub path: String,
    pub date: Option<NaiveDateTime>,
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub grade: Option<i16>,
    pub rotation: i16,
    pub is_public: bool,
    pub width: i32,
    pub height: i32,
    pub story: String
}

#[derive(Debug)]
pub enum Modification<T> {
    Created(T),
    Updated(T),
    Unchanged(T),
}

impl Photo {
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

    pub fn update_by_path(
        db: &SqliteConnection,
        file_path: &str,
        newwidth: i32,
        newheight: i32,
        exifdate: Option<NaiveDateTime>,
    ) -> Result<Option<Modification<Photo>>, Error> {
        if let Some(pic) = p::photos
            .filter(p::path.eq(&file_path.to_string()))
            .first::<Photo>(db)
            .optional()?
        {
            let mut change = false;
            // TODO Merge updates to one update statement!
            if pic.width != newwidth || pic.height != newheight {
                change = true;
                diesel::update(p::photos.find(&pic.id))
                    .set((p::width.eq(newwidth), p::height.eq(newheight)))
                    .execute(db)?;
            }
            if exifdate.is_some() && exifdate != pic.date {
                change = true;
                diesel::update(p::photos.find(&pic.id))
                    .set(p::date.eq(exifdate))
                    .execute(db)?;
            }
            let pic = p::photos
                .filter(p::path.eq(&file_path.to_string()))
                .first::<Photo>(db)?;
            Ok(Some(if change {
                Modification::Updated(pic)
            } else {
                Modification::Unchanged(pic)
            }))
        } else {
            Ok(None)
        }
    }

    pub fn create_or_set_basics(
        db: &SqliteConnection,
        id: &str,
        file_path: &str,
        newwidth: i32,
        newheight: i32,
        exifdate: Option<NaiveDateTime>,
        exifrotation: i16,
        thumbnail: &[u8],
        story: &str
    ) -> Result<Modification<Photo>, Error> {
        if let Some(result) =
            Self::update_by_path(db, file_path, newwidth, newheight, exifdate)?
        {
            Ok(result)
        } else {
            diesel::insert_into(th::thumbnail)
                .values((
                    th::id.eq(id),
                    th::content.eq(thumbnail)
                ))
                .execute(db)?;
            
            diesel::insert_into(p::photos)
                .values((
                    p::id.eq(id),
                    p::path.eq(file_path),
                    p::date.eq(exifdate),
                    p::rotation.eq(exifrotation),
                    p::width.eq(newwidth),
                    p::height.eq(newheight),
                    p::year
                        .eq(exifdate.map(|d| d.year()).unwrap_or(2000) as i32),
                    p::month
                        .eq(exifdate.map(|d| d.month()).unwrap_or(1) as i32),
                    p::day.eq(exifdate.map(|d| d.day()).unwrap_or(1) as i32),
                    p::story.eq(story)
                ))
                .execute(db)?;
            let pic = p::photos
                .filter(p::path.eq(&file_path.to_string()))
                .first::<Photo>(db)?;
            Ok(Modification::Created(pic))
        }
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
            year: y,
            month: mo as i32,
            day: da as i32,
            grade: None,
            rotation: 0,
            is_public: false,
            width: 4000,
            height: 3000,
            story: "default".into()
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
    pub details: Option<String>
}

pub trait Annotator {
    fn annotate(&self, exif: &ExifData, image: &DynamicImage) -> Annotation;
}

struct ExifCaption;
impl Annotator for ExifCaption {
    fn annotate(&self, exif: &ExifData, image: &DynamicImage) -> Annotation {
        todo!();
    }
}