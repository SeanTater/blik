use std::sync::Arc;

use super::{context::GlobalContext, LoggedIn};
use super::{Context, PhotoLink, RPhotosDB,};
use crate::models::{Photo, SizeTag};
use crate::templates::{self, RenderRucte};
use anyhow::Result;
use chrono::naive::NaiveDateTime;
use chrono::{Datelike, Local};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use log::warn;
use rocket::{State, request::FlashMessage, response::content::Html};
use serde::Deserialize;
use warp::http::response::Builder;
use warp::reply::Response;

#[get("/")]
pub fn timeline(
    _user: LoggedIn,
    globe: State<Arc<GlobalContext>>,
    db: RPhotosDB,
    flash: Option<FlashMessage>
) -> Result<Html<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let photo_list = photos
        .order(sql::<Integer>("date").desc())
        .limit(100)
        .load(&db)?
        .iter()
        .map(|photo: &Photo| PhotoLink {
            title: Some(format!(
                "{:04}-{:02}-{:02}",
                photo.year, photo.month, photo.day
            )),
            href: format!("/photo/{}/details", photo.id),
            label: Some(String::new()),
            id: photo.id.clone(),
            size: photo.get_size(SizeTag::Small),
        })
        .collect::<Vec<_>>();
    let context = Context::new(globe.inner().clone());
    let mut out = std::io::Cursor::new(vec![]);
    templates::index(&mut out, &context, "All photos", &photo_list, &[], flash)?;
    Ok(Html(out.into_inner()))
}