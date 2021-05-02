pub mod context;
mod image;
mod login;
mod photolink;
mod render_ructe;
mod urlstring;
mod views_by_date;

pub use self::context::{Context, ContextFilter};
pub use self::login::LoggedIn;
pub use self::photolink::PhotoLink;
use self::render_ructe::BuilderExt;
use self::views_by_date::*;
use super::DirOpt;
use crate::models::Photo;
use crate::templates::{self, Html, RenderRucte};
use anyhow::Result;
use chrono::Datelike;
use context::GlobalContext;
use diesel::prelude::*;
use log::info;
use rocket::http::ContentType;
use rocket::response::Content;
use rocket::response::Redirect;
use serde::Deserialize;
use std::sync::Arc;
use std::path::PathBuf;
use structopt::StructOpt;
use warp::http::{response::Builder, StatusCode};
use warp::reply::Response;
use warp::{self, Rejection};
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(flatten)]
    photos: DirOpt,
}

// pub async fn run_old(args: &Args) -> Result<()> {
//     #[rustfmt::skip]
//     let routes = warp::any()
//         .or(get().and(path("img")).and(param()).and(end()).and(s()).map(photo_details))
//         .or(get().and(path("random")).and(end()).and(s()).map(random_image))
//         .or(get().and(path("thisday")).and(end()).and(s()).map(on_this_day))
//         .or(get().and(path("next")).and(end()).and(s()).and(query()).map(next_image))
//         .or(get().and(path("prev")).and(end()).and(s()).and(query()).map(prev_image))
//         .or(path("ac").and(autocomplete::routes(s())))
//         .or(path("search").and(end()).and(get()).and(s()).and(query()).map(search))
//         .or(path("adm").and(admin::routes(s())));
// }
#[database("rphotosdb")]
pub struct RPhotosDB(SqliteConnection);

pub async fn run(args: &Args) -> anyhow::Result<()> {
    rocket::ignite()
        .mount(
            "/",
            routes![
                need_to_login,
                self::login::get_login,
                self::login::post_login,
                self::login::logout,
                self::login::invite,
                self::views_by_date::timeline,
                self::image::thumbnail,
                self::image::full,
                self::image::upload,
                self::static_file,
            ],
        )
        //.mount("/static", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/res")))
        .manage(Arc::new(GlobalContext::new(args)))
        .attach(rocket_contrib::helmet::SpaceHelmet::default())
        .attach(RPhotosDB::fairing())
        .launch();
    Ok(())
}

#[get("/", rank = 2)]
fn need_to_login() -> Redirect {
    Redirect::to("/login")
}

/// Handler for static files.
/// Create a response from the file data with a correct content type
/// and a far expires header (or a 404 if the file does not exist).
#[get("/static/<path..>")]
pub fn static_file(path: PathBuf) -> Option<Content<Vec<u8>>> {
    use templates::statics::StaticFile;
    path.into_os_string()
        .to_str()
        .and_then(StaticFile::get)
        .map(|data| {
            Content(
                ContentType::new(
                    data.mime.type_().as_str(),
                    data.mime.subtype().as_str(),
                ),
                data.content.to_vec(),
            )
        })
}

// fn photo_details(id: String, context: Context) -> Response {
//     use crate::schema::photos::dsl::photos;
//     let c = context.db();
//     if let Ok(tphoto) = photos.find(id).first::<Photo>(&c) {
//         if context.is_authorized() || tphoto.is_public() {
//             return Builder::new()
//                 .html(|o| {
//                     templates::details(
//                         o,
//                         &context,
//                         &tphoto
//                             .date
//                             .map(|d| {
//                                 vec![
//                                     Link::year(d.year()),
//                                     Link::month(d.year(), d.month() as i32),
//                                     Link::day(
//                                         d.year(),
//                                         d.month() as i32,
//                                         d.day() as i32,
//                                     ),
//                                     Link::prev(&tphoto.id),
//                                     Link::next(&tphoto.id),
//                                 ]
//                             })
//                             .unwrap_or_default(),
//                         &tphoto.load_people(&c).unwrap(),
//                         &tphoto.load_places(&c).unwrap(),
//                         &tphoto.load_tags(&c).unwrap(),
//                         &tphoto.load_position(&c),
//                         &tphoto.load_attribution(&c),
//                         &tphoto,
//                     )
//                 })
//                 .unwrap();
//         }
//     }
//     not_found(&context)
// }
#[derive(Debug, Default, Deserialize)]
pub struct ImgRange {
    pub from: Option<String>,
    pub to: Option<String>,
}