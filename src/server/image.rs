use std::sync::Arc;

use super::{context::GlobalContext, LoggedIn, RPhotosDB};
use crate::models::Photo;
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
    use crate::schema::thumbnail::dsl::thumbnail;
    let db = db.0;
    let thumb = thumbnail.find(&id).first::<crate::models::Thumbnail>(&db)?;

    Ok(Content(
        rocket::http::ContentType::JPEG,
        thumb.content.clone(),
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
    while let Some(part) = parts.read_entry().ok()? {
        // This limits each image to 100MB but no limit for total images
        // This is intended for one user so as long as they are logged in,
        // So this should mostly reduce careless mistakes
        // Even a 100 MB limit may not make sense if we support video later.

        let mut image_slice = vec![];

        part.data.take(100 << 20).read_to_end(&mut image_slice).ok()?;
        match story {
            None => {
                //
                // Read a story
                //
                if part.headers.name.as_ref() == "description" {
                    use crate::schema::story::dsl as story_dsl;
                    let description = String::from_utf8(image_slice)
                        .ok()
                        .filter(|desc| !desc.trim().is_empty())
                        .unwrap_or_else(|| {
                            // Create a halfway decent title
                            chrono::Local::now().format("Uploaded %B %e, %Y").to_string()
                        });
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
                if image_slice.len() == 100 << 20 {
                    success = false;
                    messages.push(format!(
                        "Image {} is too large: it needs to be under 100 MB per image",
                        part.headers.filename.unwrap_or("untitled".to_string())
                    ));
                } else {
                    println!("Read image {} bytes long", image_slice.len());
                    messages.push(match globe.collection.manage(&db.0).index_photo(&image_slice, &story_name) {
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
        }
    }
    Some(if success {
        Flash::success(Redirect::to("/"), messages.join("\n"))
    } else {
        Flash::warning(Redirect::to("/"), messages.join("\n"))
    })
}
