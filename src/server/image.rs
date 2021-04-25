use std::sync::Arc;

use super::{LoggedIn, RPhotosDB, context::GlobalContext};
use crate::models::Photo;
use anyhow::Result;
use diesel::prelude::*;
use rocket::{State, response::content::Content, http::ContentType};

#[get("/photo/<id>/thumbnail")]
pub fn thumbnail(
    _user: LoggedIn,
    db: RPhotosDB,
    id: String
) -> Result<Content<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let pho = photos.find(&id).first::<Photo>(&db)?;
    //let pho_data = std::fs::read(&pho.path)?;

    Ok(Content(rocket::http::ContentType::JPEG, pho.thumbnail.clone()))
}

#[get("/photo/<id>")]
pub fn full(
    _user: LoggedIn,
    db: RPhotosDB,
    globe: State<Arc<GlobalContext>>,
    id: String
) -> Option<Content<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let pho = photos.find(&id).first::<Photo>(&db).ok()?;
    let pho_path = globe.collection.get_raw_path(&pho);
    let mime = mime_guess::from_path(&pho_path)
        .first_or_octet_stream();
    Some(Content(
        ContentType::new(
            mime.type_().to_string(),
            mime.subtype().to_string()
        ),
        std::fs::read(pho_path).ok()?
    ))
}