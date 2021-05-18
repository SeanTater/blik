mod image;
mod login;
mod urlstring;
mod timeline;
mod context;
mod story;

pub use self::login::LoggedIn;
use self::context::GlobalContext;
use super::DirOpt;
use crate::templates;
use diesel::prelude::*;
use rocket::http::ContentType;
use rocket::response::Content;
use rocket::response::Redirect;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(flatten)]
    photos: DirOpt,
}
#[database("blikdb")]
pub struct BlikDB(SqliteConnection);

pub async fn run(args: &Args) -> anyhow::Result<()> {
    let globe = GlobalContext::new(args);

    if !Path::new("Rocket.toml").exists() {
        println!("Blik is unconfigured. Generating a new configuration automatically.");
        let code: Vec<u8> = (0..32).into_iter().map(|_| rand::random()).collect();
        let code = base64::encode(code);
        
        // Write a new Rocket.toml that configures the database locations and keeps the secret keys static
        std::fs::write(Path::new("Rocket.toml"), format!("[global.databases]
# This is the location of the metadata database. By convention it is always blik.db so don't change it
blikdb = {{ url = \"blik.db\" }}

[production]
# These secret keys are automatically generated random 256 bit numbers as base64
# They are stored here so when blik restarts, the cookies it made will be encrypted with the same
# keys. Otherwise everyone would be logged out when blik restarts.
# For the same effect (e.g. if your token was compromised) you can change this key, which will log
# everyone out. When blik starts again it will print to the terminal a new token to log in with. 

secret_key = \"{code}\"

# Rocket has attractive and comprehensive documentation about the settings available here:
# https://rocket.rs/v0.4/guide/configuration/#extras
# But in most cases these two settings are what you would want to override:
# address = \"0.0.0.0\"  # by default, listen everwhere
# port = 8000            # listen on port 8000


# In most cases you will use production mode so you don't need to use the following sections.
# But they are almost exactly the same as the production section, and you select which one
# using the ROCKET_ENV variable. For example `ROCKET_ENV=staging blik` would use staging, but just
# `blik` will use production.
[development]
secret_key = \"{code}\"
[staging]
secret_key = \"{code}\"

", code=code))?;
        crate::dbopt::initial_setup()?;
        println!("Done! You can see your new app at localhost:8000.");
        let url = format!("http://localhost:8000/login/{}", globe.generate_login_token(15));
        match webbrowser::open(&url) {
            Ok(_) => println!("Your browser should open to {} in just a sec!", url),
            Err(x) => println!("Couldn't launch your browser. Please navigate to {}", url)
        }
    } else {
        let code = globe.generate_login_token(15);
        println!("You can login with code {} in the next 15 minutes", code);
    }
    rocket::ignite()
        .mount(
            "/",
            routes![
                need_to_login,
                self::login::get_login,
                self::login::login_via_url,
                self::login::post_login,
                self::login::logout,
                self::login::invite,
                self::timeline::timeline,
                self::image::thumbnail,
                self::image::full,
                self::image::upload,
                self::story::create,
                self::story::summary,
                self::static_file,
            ],
        )
        .manage(Arc::new(globe))
        .attach(rocket_contrib::helmet::SpaceHelmet::default())
        .attach(BlikDB::fairing())
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
    let path_str = path.as_os_str().to_str()?;
    let data = StaticFile::get(path_str)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();  
    Some(Content(
        ContentType::new(
            mime.type_().to_string(),
            mime.subtype().to_string(),
        ),
        data.content.to_vec(),
    ))
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
