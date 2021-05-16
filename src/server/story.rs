use rocket::{request::{FlashMessage, Form}, response::{Flash, Redirect, content::Html}};
use super::{LoggedIn, BlikDB};
use crate::models::Story;
use crate::templates;
use anyhow::Result;

#[derive(FromForm)]
pub struct StoryForm {
    title: String,
    description: String
}

#[post("/story", data = "<story_form>")]
pub fn create(
    _user: LoggedIn,
    db: BlikDB,
    story_form: Form<StoryForm>
) -> Result<Flash<Redirect>> {
    let name = story_form.title.split("\n").next().unwrap_or("");
    let name = slug::slugify(&name[..name.len().min(50)]);
    Story::new(name, story_form.title.clone(), story_form.description.clone()).save(&db.0)?;
    Ok(Flash::success(Redirect::to("/"), "Created new story"))
}


#[get("/story/<story_name>")]
pub fn summary(
    _user: LoggedIn,
    db: BlikDB,
    flash: Option<FlashMessage>,
    story_name: String
) -> Result<Html<Vec<u8>>> {
    let ref db = db.0;

    let mut out = std::io::Cursor::new(vec![]);
    templates::story(
        &mut out,
        db,
        &Story::by_name(db, &story_name)?,
        flash
    )?;
    Ok(Html(out.into_inner()))
}