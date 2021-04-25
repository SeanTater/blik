use crate::schema::attributions::dsl as a;
use crate::schema::people::dsl as h;
use crate::schema::photo_people::dsl as ph;
use crate::schema::photo_places::dsl as pl;
use crate::schema::photo_tags::dsl as pt;
use crate::schema::photos;
use crate::schema::photos::dsl as p;
use crate::schema::places::dsl as l;
use crate::schema::positions::dsl as pos;
use crate::schema::tags::dsl as t;
use chrono::{naive::NaiveDateTime, Datelike};
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sql_types::Integer;
use diesel::sqlite::{Sqlite, SqliteConnection};
use log::error;
use slug::slugify;
use std::cmp::max;

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
    pub attribution_id: Option<i32>,
    pub width: i32,
    pub height: i32,
    pub thumbnail: Vec<u8>
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
        thumbnail: &[u8]
    ) -> Result<Modification<Photo>, Error> {
        if let Some(result) =
            Self::update_by_path(db, file_path, newwidth, newheight, exifdate)?
        {
            Ok(result)
        } else {
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
                    p::thumbnail.eq(thumbnail),
                ))
                .execute(db)?;
            let pic = p::photos
                .filter(p::path.eq(&file_path.to_string()))
                .first::<Photo>(db)?;
            Ok(Modification::Created(pic))
        }
    }

    pub fn load_people(
        &self,
        db: &SqliteConnection,
    ) -> Result<Vec<Person>, Error> {
        h::people
            .filter(
                h::id.eq_any(
                    ph::photo_people
                        .select(ph::person_id)
                        .filter(ph::photo_id.eq(&self.id)),
                ),
            )
            .load(db)
    }

    pub fn load_places(
        &self,
        db: &SqliteConnection,
    ) -> Result<Vec<Place>, Error> {
        l::places
            .filter(
                l::id.eq_any(
                    pl::photo_places
                        .select(pl::place_id)
                        .filter(pl::photo_id.eq(&self.id)),
                ),
            )
            .order(l::osm_level.desc())
            .load(db)
    }
    pub fn load_tags(&self, db: &SqliteConnection) -> Result<Vec<Tag>, Error> {
        t::tags
            .filter(
                t::id.eq_any(
                    pt::photo_tags
                        .select(pt::tag_id)
                        .filter(pt::photo_id.eq(&self.id)),
                ),
            )
            .load(db)
    }

    pub fn load_position(&self, db: &SqliteConnection) -> Option<Coord> {
        match pos::positions
            .filter(pos::photo_id.eq(&self.id))
            .select((pos::latitude, pos::longitude))
            .first::<(i32, i32)>(db)
        {
            Ok(c) => Some(c.into()),
            Err(diesel::NotFound) => None,
            Err(err) => {
                error!("Failed to read position: {}", err);
                None
            }
        }
    }

    pub fn load_attribution(&self, db: &SqliteConnection) -> Option<String> {
        self.attribution_id.and_then(|i| {
            a::attributions.find(i).select(a::name).first(db).ok()
        })
    }

    pub fn get_size(&self, size: SizeTag) -> (u32, u32) {
        let (width, height) = (self.width, self.height);
        let scale = f64::from(size.px()) / f64::from(max(width, height));
        let w = (scale * f64::from(width)) as u32;
        let h = (scale * f64::from(height)) as u32;
        match self.rotation {
            _x @ 0..=44 | _x @ 315..=360 | _x @ 135..=224 => (w, h),
            _ => (h, w),
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
            attribution_id: None,
            width: 4000,
            height: 3000,
        }
    }
}

pub trait Facet {
    fn by_slug(slug: &str, db: &SqliteConnection) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(Debug, Clone, Queryable)]
pub struct Tag {
    pub id: i32,
    pub slug: String,
    pub tag_name: String,
}

impl Facet for Tag {
    fn by_slug(slug: &str, db: &SqliteConnection) -> Result<Tag, Error> {
        t::tags.filter(t::slug.eq(slug)).first(db)
    }
}

#[derive(Debug, Clone, Queryable)]
pub struct PhotoTag {
    pub id: i32,
    pub photo_id: String,
    pub tag_id: i32,
}

#[derive(Debug, Clone, Queryable)]
pub struct Person {
    pub id: i32,
    pub slug: String,
    pub person_name: String,
}

impl Person {
    pub fn get_or_create_name(
        db: &SqliteConnection,
        name: &str,
    ) -> Result<Person, Error> {
        match h::people
            .filter(h::person_name.like(name))
            .first::<Person>(db)
        {
            Ok(person) => Ok(person),
            Err(_) => {
                let slug = slugify(name);
                diesel::insert_into(h::people)
                    .values((h::person_name.eq(name), h::slug.eq(&slug)))
                    .execute(db)?;
                h::people.filter(h::slug.eq(slug)).first(db)
            }
        }
    }
}

impl Facet for Person {
    fn by_slug(slug: &str, db: &SqliteConnection) -> Result<Person, Error> {
        h::people.filter(h::slug.eq(slug)).first(db)
    }
}

#[derive(Debug, Clone, Queryable)]
pub struct PhotoPerson {
    pub id: i32,
    pub photo_id: String,
    pub person_id: i32,
}

#[derive(Debug, Clone, Queryable)]
pub struct Place {
    pub id: i32,
    pub slug: String,
    pub place_name: String,
    pub osm_id: Option<i64>,
    pub osm_level: Option<i16>,
}

impl Facet for Place {
    fn by_slug(slug: &str, db: &SqliteConnection) -> Result<Place, Error> {
        l::places.filter(l::slug.eq(slug)).first(db)
    }
}

#[derive(Debug, Clone, Queryable)]
pub struct PhotoPlace {
    pub id: i32,
    pub photo_id: String,
    pub place_id: i32,
}

#[derive(Debug, Clone)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

impl Queryable<(Integer, Integer), Sqlite> for Coord {
    type Row = (i32, i32);

    fn build(row: Self::Row) -> Self {
        Coord::from((row.0, row.1))
    }
}

impl From<(i32, i32)> for Coord {
    fn from((lat, long): (i32, i32)) -> Coord {
        Coord {
            x: f64::from(lat) / 1e6,
            y: f64::from(long) / 1e6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeTag {
    Small,
    Medium,
    Large,
}

impl SizeTag {
    pub fn px(self) -> u32 {
        match self {
            SizeTag::Small => 288,
            SizeTag::Medium => 1080,
            SizeTag::Large => 8192, // not really used
        }
    }
    pub fn tag(self) -> char {
        match self {
            SizeTag::Small => 's',
            SizeTag::Medium => 'm',
            SizeTag::Large => 'l',
        }
    }
}
