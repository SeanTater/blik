mod image;
mod login;
mod photolink;
mod render_ructe;
mod urlstring;
mod timeline;
mod context;

pub use self::login::LoggedIn;
pub use self::photolink::PhotoLink;
use self::context::GlobalContext;
use super::DirOpt;
use crate::templates;
use diesel::prelude::*;
use rocket::http::ContentType;
use rocket::response::Content;
use rocket::response::Redirect;
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(flatten)]
    photos: DirOpt,
}
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
                self::timeline::timeline,
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
