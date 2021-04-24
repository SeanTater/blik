use super::{AnyhowRejectionExt, context::GlobalContext};
use super::BuilderExt;
use super::{error_response, not_found, Context};
use crate::collection::{get_scaled_jpeg, ImageLoadFailed};
use crate::models::Photo;
use anyhow::Result;
use diesel::prelude::*;
use rocket::{State, http::Cookies};
use warp::http::response::Builder;
use warp::http::{header, StatusCode};
use warp::reply::Response;
use warp::Rejection;

#[get("/img/<id>/<size>")]
pub fn show_image(globe: State<GlobalContext>, mut cookies: Cookies, id: String, size: usize) -> Option<String> {
    let token = cookies
        .get_private("auth_token")?;
    Some(format!("You wanted image {} scaled to {}, with token {}", id, size, token))
}

pub async fn show_image_old(
    id: String,
    size: u32,
    context: Context,
) -> Result<Response, Rejection> {
    use crate::schema::photos::dsl::photos;
    let tphoto = photos.find(&id).first::<Photo>(&context.db());
    if let Ok(tphoto) = tphoto {
        if context.is_authorized() || tphoto.is_public() {
            if size > 2000 {
                if context.is_authorized() {
                    use std::fs::File;
                    use std::io::Read;
                    // TODO: This should be done in a more async-friendly way.
                    let path = context.photos().get_raw_path(&tphoto);
                    let mut buf = Vec::new();
                    if File::open(path)
                        .map(|mut f| f.read_to_end(&mut buf))
                        .is_ok()
                    {
                        return Ok(Builder::new()
                            .status(StatusCode::OK)
                            .header(
                                header::CONTENT_TYPE,
                                mime::IMAGE_JPEG.as_ref(),
                            )
                            .far_expires()
                            .body(buf.into())
                            .or_reject()?);
                    } else {
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                    }
                }
            } else {
                let data = get_image_data(&context, &tphoto, size)
                    .await
                    .or_reject()?;
                return Ok(Builder::new()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime::IMAGE_JPEG.as_ref())
                    .far_expires()
                    .body(data.into())
                    .or_reject()?);
            }
        }
    }
    Ok(not_found(&context))
}

async fn get_image_data(
    context: &Context,
    photo: &Photo,
    size: u32,
) -> Result<Vec<u8>, ImageLoadFailed> {
    let p = context.photos().get_raw_path(photo);
    let r = photo.rotation;
    get_scaled_jpeg(p, r, size).await
}
