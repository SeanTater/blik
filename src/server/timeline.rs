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
    templates::index(
        &mut out,
        db,
        flash
    )?;
    Ok(Html(out.into_inner()))
}
