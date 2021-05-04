use std::sync::Arc;

use super::{context::GlobalContext, LoggedIn, RPhotosDB};
use crate::models::{Photo, Story};
use anyhow::Result;
use diesel::prelude::*;
use rocket::response::{
    content::Content,
    Flash, Redirect,
};
use rocket::{http::ContentType, Data, State};
use std::io::Read;

#[get("/photo/<id>/thumbnail")]
pub fn thumbnail(
    _user: LoggedIn,
    db: RPhotosDB,
    id: String,
) -> Result<Content<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let pho = photos.find(&id).first::<Photo>(&db)?;
    //let pho_data = std::fs::read(&pho.path)?;

    Ok(Content(
        rocket::http::ContentType::JPEG,
        pho.thumbnail.clone(),
    ))
}

#[get("/photo/<id>")]
pub fn full(
    _user: LoggedIn,
    db: RPhotosDB,
    globe: State<Arc<GlobalContext>>,
    id: String,
) -> Option<Content<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let pho = photos.find(&id).first::<Photo>(&db).ok()?;
    let pho_path = globe.collection.get_raw_path(&pho);
    let mime = mime_guess::from_path(&pho_path).first_or_octet_stream();
    Some(Content(
        ContentType::new(mime.type_().to_string(), mime.subtype().to_string()),
        std::fs::read(pho_path).ok()?,
    ))
}

#[post("/photo", data = "<image>")]
pub fn upload(
    _user: LoggedIn,
    db: RPhotosDB,
    content_type: &ContentType,
    globe: State<Arc<GlobalContext>>,
    image: Data,
) -> Option<Flash<Redirect>> {
    let boundary = content_type.params().find(|(x, _)| *x == "boundary")?.1;
    let reader = image.open();
    let mut parts = multipart::server::Multipart::with_body(reader, boundary);
    let mut messages = vec![];
    let mut success = true;
    let mut story: Option<String> = None;
    while let Some(mut part) = parts.read_entry().ok()? {
        // This limits each image to 100MB but no limit for total images
        // This is intended for one user so as long as they are logged in,
        // So this should mostly reduce careless mistakes
        // Even a 100 MB limit may not make sense if we support video later.

        let mut image_buf = vec![];

        part.data.take(100 << 20).read_to_end(&mut image_buf).ok()?;
        match story {
            None => {
                //
                // Read a story
                //
                if part.headers.name.as_ref() == "description" {
                    use crate::schema::story::dsl as story_dsl;
                    let description = String::from_utf8(image_buf).ok()?;
                    let name = description.split("\n").next().unwrap_or("");
                    let name = slug::slugify(&name[..name.len().min(50)]);
                    diesel::insert_or_ignore_into(story_dsl::story)
                        .values((
                            story_dsl::name.eq(&name),
                            story_dsl::description.eq(description)
                        ))
                        .execute(&db.0)
                        .ok()?;
                    story = Some(name);
                } else {
                    // The first multipart must be the story
                    messages.push("The first part of an upload must be the story".to_string());
                    success = false;
                    break;
                }
            }
            Some(ref story_name) => {
                //
                // Read an image
                //
                if image_buf.len() == 100 << 20 {
                    success = false;
                    messages.push(format!(
                        "Image {} is too large: it needs to be under 100 MB per image",
                        part.headers.filename.unwrap_or("untitled".to_string())
                    ));
                } else {
                    println!("Read image {} bytes long", image_buf.len());
                    messages.push(match globe.collection.save_photo(&image_buf, &story_name) {
                        Ok((id, path)) => {
                            format!("Saved image with ID {} to {}", id, path.display())
                        }
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
        }
    }
    Some(if success {
        Flash::success(Redirect::to("/"), messages.join("\n"))
    } else {
        Flash::warning(Redirect::to("/"), messages.join("\n"))
    })
}
