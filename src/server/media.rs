use std::sync::Arc;

use super::{context::GlobalContext, LoggedIn, BlikDB};
use crate::models::{Media, Story};
use anyhow::Result;
use diesel::prelude::*;
use rocket::response::{
    content::Content,
    Flash, Redirect,
};
use rocket::{http::ContentType, Data, State};
use std::io::Read;

#[get("/media/<id>/thumbnail")]
pub fn thumbnail(
    _user: LoggedIn,
    db: BlikDB,
    id: String,
) -> Result<Content<Vec<u8>>> {
    use crate::schema::thumbnail::dsl::thumbnail;
    let db = db.0;
    let thumb = thumbnail.find(&id).first::<crate::models::Thumbnail>(&db)?;

    Ok(Content(
        rocket::http::ContentType::new("image", "avif"),
        thumb.content.clone(),
    ))
}

#[get("/media/<id>")]
pub fn full(
    _user: LoggedIn,
    db: BlikDB,
    globe: State<Arc<GlobalContext>>,
    id: String,
) -> Option<Content<Vec<u8>>> {
    use crate::schema::media::dsl::media;
    let db = db.0;
    let pho = media.find(&id).first::<Media>(&db).ok()?;
    let pho_path = globe.collection.get_raw_path(&pho);
    let mime = mime_guess::from_path(&pho_path).first_or_octet_stream();
    Some(Content(
        ContentType::new(mime.type_().to_string(), mime.subtype().to_string()),
        std::fs::read(pho_path).ok()?,
    ))
}

#[post("/story/<story_name>", data = "<image>")]
pub fn upload(
    _user: LoggedIn,
    db: BlikDB,
    content_type: &ContentType,
    globe: State<Arc<GlobalContext>>,
    story_name: String,
    image: Data,
) -> Option<Flash<Redirect>> {
    let boundary = content_type.params().find(|(x, _)| *x == "boundary")?.1;
    let reader = image.open();
    let mut parts = multipart::server::Multipart::with_body(reader, boundary);
    let mut messages = vec![];
    let mut success = true;
    if !Story::by_name(&db.0, &story_name).is_ok() {
        return Some(Flash::warning(
            Redirect::to("/"), 
            format!("There is no story by the name {}", story_name)))
    }
    while let Some(part) = parts.read_entry().ok()? {
        // This limits each image to 100MB but no limit for total images
        // This is intended for one user so as long as they are logged in,
        // So this should mostly reduce careless mistakes
        // Even a 100 MB limit may not make sense if we support video later.

        let mut image_slice = vec![];
        part.data.take(1 << 30).read_to_end(&mut image_slice).ok()?;
        //
        // Read an image
        //
        if image_slice.len() == 1 << 30 {
            success = false;
            messages.push(format!(
                "Image {} is too large: it needs to be under 1 GB per image",
                part.headers.filename.unwrap_or("untitled".to_string())
            ));
        } else {
            println!("Read image {} bytes long", image_slice.len());
            messages.push(match globe.collection
                    .manage(&db.0)
                    .index_media(&part.headers.content_type?, &image_slice, &story_name) {
                Ok(_) => "Saved image".into(),
                Err(err) => {
                    success = false;
                    format!(
                        "Failed to upload {}: {}",
                        part.headers
                            .filename
                            .unwrap_or("untitled".to_string()),
                        err
                    )
                }
            });
        }
    }
    Some(if success {
        Flash::success(Redirect::to("/"), messages.join("\n"))
    } else {
        Flash::warning(Redirect::to("/"), messages.join("\n"))
    })
}
