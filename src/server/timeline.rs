use super::LoggedIn;
use super::BlikDB;
use crate::templates;
use anyhow::Result;
use rocket::{request::FlashMessage, response::content::Html};

#[get("/")]
pub fn timeline(
    _user: LoggedIn,
    db: BlikDB,
    flash: Option<FlashMessage>,
) -> Result<Html<Vec<u8>>> {
    let ref db = db.0;

    let mut out = std::io::Cursor::new(vec![]);
    templates::timeline(
        &mut out,
        db,
        None,
        flash
    )?;
    Ok(Html(out.into_inner()))
}

#[get("/year/<year>")]
pub fn timeline_year(
    _user: LoggedIn,
    db: BlikDB,
    year: i32,
    flash: Option<FlashMessage>,
) -> Result<Html<Vec<u8>>> {
    let ref db = db.0;

    let mut out = std::io::Cursor::new(vec![]);
    templates::timeline(
        &mut out,
        db,
        Some(year),
        flash
    )?;
    Ok(Html(out.into_inner()))
}