mod admin;
mod autocomplete;
pub mod context;
mod image;
mod login;
mod photolink;
mod render_ructe;
mod urlstring;
mod views_by_date;

pub use self::context::{Context, ContextFilter};
pub use self::photolink::PhotoLink;
pub use self::login::LoggedIn;
use self::render_ructe::BuilderExt;
use self::views_by_date::*;
use super::DirOpt;
use crate::models::Photo;
use crate::pidfiles::handle_pid_file;
use crate::templates::{self, Html, RenderRucte};
use anyhow::Result;
use chrono::Datelike;
use context::GlobalContext;
use diesel::prelude::*;
use log::info;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};
use structopt::StructOpt;
use warp::filters::path::Tail;
use warp::http::{header, response::Builder, StatusCode};
use warp::reply::Response;
use warp::{self, Filter, Rejection, Reply};
use std::sync::Arc;
use rocket::response::Redirect;
use rocket::response::Content;
use rocket::http::ContentType;
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(flatten)]
    photos: DirOpt,

    /// Write (and read, if --replace) a pid file with the name
    /// given as <PIDFILE>.
    #[structopt(long)]
    pidfile: Option<String>,
    /// Kill old server (identified by pid file) before running.
    #[structopt(long, short)]
    replace: bool,
    /// Socket addess for rphotos to listen on.
    #[structopt(
        long,
        env = "RPHOTOS_LISTEN",
        default_value = "127.0.0.1:6767"
    )]
    listen: SocketAddr,
    /// Signing key for jwt
    #[structopt(long, env = "JWT_KEY", hide_env_values = true)]
    jwt_key: String,
}

// pub async fn run_old(args: &Args) -> Result<()> {
//     if let Some(pidfile) = &args.pidfile {
//         handle_pid_file(&pidfile, args.replace).unwrap()
//     }
//     use warp::filters::query::query;
//     use warp::path::{end, param};
//     use warp::{body, get, path, post};
//     let static_routes = path("static")
//         .and(get())
//         .and(path::tail())
//         .and_then(static_file);
//     #[rustfmt::skip]
//     let routes = warp::any()
//         //.and(static_routes)
//         //.or(get().and(path("login")).and(end()).and(s()).and(query()).map(login::get_login))
//         //.or(post().and(path("login")).and(end()).and(s()).and(body::form()).map(login::post_login))
//         //.or(path("logout").and(end()).and(s()).map(login::logout))
//         //.or(get().and(end()).and(s()).map(all_years))
//         .or(get().and(path("img")).and(param()).and(end()).and(s()).map(photo_details))
//         .or(get().and(path("img")).and(param()).and(param()).and(end()).and(s()).and_then(image::show_image))
//         //.or(get().and(path("0")).and(end()).and(s()).map(all_null_date))
//         //.or(get().and(param()).and(end()).and(s()).map(months_in_year))
//         //.or(get().and(param()).and(param()).and(end()).and(s()).map(days_in_month))
//         //.or(get().and(param()).and(param()).and(param()).and(end()).and(query()).and(s()).map(all_for_day))
//         .or(path("person").and(person_routes(s())))
//         .or(path("place").and(place_routes(s())))
//         .or(path("tag").and(tag_routes(s())))
//         .or(get().and(path("random")).and(end()).and(s()).map(random_image))
//         .or(get().and(path("thisday")).and(end()).and(s()).map(on_this_day))
//         .or(get().and(path("next")).and(end()).and(s()).and(query()).map(next_image))
//         .or(get().and(path("prev")).and(end()).and(s()).and(query()).map(prev_image))
//         .or(path("ac").and(autocomplete::routes(s())))
//         .or(path("search").and(end()).and(get()).and(s()).and(query()).map(search))
//         //.or(path("api").and(api::routes(s())))
//         .or(path("adm").and(admin::routes(s())));
//     warp::serve(routes.recover(customize_error))
//         .run(args.listen)
//         .await;
//     Ok(())
// }
#[database("rphotosdb")]
pub struct RPhotosDB(SqliteConnection);

pub async fn run(args: &Args) -> anyhow::Result<()> {
    rocket::ignite()
    .mount("/", routes![
        need_to_login,
        self::login::get_login,
        self::login::post_login,
        self::login::logout,
        self::views_by_date::timeline,
        self::image::thumbnail,
        self::image::full,
        self::static_file,
    ])
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

fn not_found(context: &Context) -> Response {
    Builder::new()
        .status(StatusCode::NOT_FOUND)
        .html(|o| {
            templates::not_found(
                o,
                context,
                StatusCode::NOT_FOUND,
                "The resource you requested could not be located.",
            )
        })
        .unwrap()
}

fn redirect_to_img(image: &str) -> Response {
    redirect(&format!("/img/{}", image))
}

fn redirect(url: &str) -> Response {
    Builder::new().redirect(url)
}

fn permission_denied() -> Result<Response, Rejection> {
    error_response(StatusCode::UNAUTHORIZED)
}

fn error_response(err: StatusCode) -> Result<Response, Rejection> {
    Builder::new()
        .status(err)
        .html(|o| templates::error(o, err, "Sorry about this."))
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
        .map(|data| Content(ContentType::new(
            data.mime.type_().as_str(),
            data.mime.subtype().as_str()
        ), data.content.to_vec()))
}

fn random_image(context: Context) -> Response {
    use crate::schema::photos::dsl::id;
    use diesel::expression::dsl::sql;
    use diesel::sql_types::Integer;
    if let Ok(photo) = Photo::query(context.is_authorized())
        .select(id)
        .limit(1)
        .order(sql::<Integer>("random()"))
        .first::<String>(&context.db())
    {
        info!("Random: {:?}", photo);
        redirect_to_img(&photo)
    } else {
        not_found(&context)
    }
}

fn photo_details(id: String, context: Context) -> Response {
    use crate::schema::photos::dsl::photos;
    let c = context.db();
    if let Ok(tphoto) = photos.find(id).first::<Photo>(&c) {
        if context.is_authorized() || tphoto.is_public() {
            return Builder::new()
                .html(|o| {
                    templates::details(
                        o,
                        &context,
                        &tphoto
                            .date
                            .map(|d| {
                                vec![
                                    Link::year(d.year()),
                                    Link::month(d.year(), d.month() as i32),
                                    Link::day(
                                        d.year(),
                                        d.month() as i32,
                                        d.day() as i32,
                                    ),
                                    Link::prev(&tphoto.id),
                                    Link::next(&tphoto.id),
                                ]
                            })
                            .unwrap_or_default(),
                        &tphoto.load_people(&c).unwrap(),
                        &tphoto.load_places(&c).unwrap(),
                        &tphoto.load_tags(&c).unwrap(),
                        &tphoto.load_position(&c),
                        &tphoto.load_attribution(&c),
                        &tphoto,
                    )
                })
                .unwrap();
        }
    }
    not_found(&context)
}

pub type Link = Html<String>;

impl Link {
    fn year(year: i32) -> Self {
        Html(format!(
            "<a href='/{0}/' title='Images from {0}' accessKey='y'>{0}</a>",
            year,
        ))
    }
    fn month(year: i32, month: i32) -> Self {
        Html(format!(
            "<a href='/{0}/{1}/' title='Images from {2} {0}' \
             accessKey='m'>{1}</a>",
            year,
            month,
            monthname(month),
        ))
    }
    fn day(year: i32, month: i32, day: i32) -> Self {
        Html(format!(
            "<a href='/{0}/{1}/{2}' title='Images from {2} {3} {0}' \
             accessKey='d'>{2}</a>",
            year,
            month,
            day,
            monthname(month),
        ))
    }
    fn prev(from: &str) -> Self {
        Html(format!(
            "<a href='/prev?from={}' title='Previous image (by time)'>\
             \u{2190}</a>",
            from,
        ))
    }
    fn next(from: &str) -> Self {
        Html(format!(
            "<a href='/next?from={}' title='Next image (by time)' \
             accessKey='n'>\u{2192}</a>",
            from,
        ))
    }
}
#[derive(Debug, Default, Deserialize)]
pub struct ImgRange {
    pub from: Option<String>,
    pub to: Option<String>,
}
/// Make anything that can be an anyhow Error into a Rejection
#[derive(Debug)]
struct AnyhowRejection(anyhow::Error);
impl warp::reject::Reject for AnyhowRejection {}
impl<X> From<X> for AnyhowRejection
where
    X: Into<anyhow::Error>,
{
    fn from(x: X) -> Self {
        AnyhowRejection(x.into())
    }
}

/// Make any error into a rejection
trait AnyhowRejectionExt<X> {
    fn or_reject(self) -> Result<X, AnyhowRejection>;
}
impl<X, Y> AnyhowRejectionExt<X> for Result<X, Y>
where
    Y: Into<anyhow::Error>,
{
    fn or_reject(self) -> Result<X, AnyhowRejection> {
        self.map_err(AnyhowRejection::from)
    }
}

type WarpResult = Result<Response, Rejection>;
