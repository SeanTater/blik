use super::LoggedIn;
use super::{PhotoLink, RPhotosDB};
use crate::models::Photo;
use crate::templates;
use anyhow::Result;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use rocket::{request::FlashMessage, response::content::Html};

#[get("/")]
pub fn timeline(
    _user: LoggedIn,
    db: RPhotosDB,
    flash: Option<FlashMessage>,
) -> Result<Html<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let photo_list = photos
        .order(sql::<Integer>("date").desc())
        .limit(100)
        .load(&db)?
        .iter()
        .map(|photo: &Photo| PhotoLink {
            title: Some(photo.date.map(|d| d.date().format("%Y-%m-%d").to_string())).unwrap_or_default(),
            href: format!("/photo/{}/details", photo.id),
            label: Some(String::new()),
            id: photo.id.clone(),
        })
        .collect::<Vec<_>>();
    let mut out = std::io::Cursor::new(vec![]);
    templates::index(
        &mut out,
        "All photos",
        &photo_list,
        flash,
    )?;
    Ok(Html(out.into_inner()))
}
