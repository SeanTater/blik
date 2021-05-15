use super::LoggedIn;
use super::{PhotoLink, RPhotosDB};
use crate::models::*;
use crate::templates;
use anyhow::Result;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use rocket::{request::FlashMessage, response::content::Html};

#[get("/")]
pub fn timeline(
    _user: LoggedIn,
    db: RPhotosDB,
    flash: Option<FlashMessage>,
) -> Result<Html<Vec<u8>>> {
    use crate::schema::photos::dsl as p;
    let ref db = db.0;

    let mut out = std::io::Cursor::new(vec![]);
    templates::index(
        &mut out,
        "All photos",
        flash,
        db
    )?;
    Ok(Html(out.into_inner()))
}
