use super::{Context, RenderRucte, context::GlobalContext};
use crate::templates;
use rocket::{Request, State, http::{Cookie, Cookies}, request::{Form, FromRequest, Outcome}, response::Redirect};
use rocket::response::content::Html;
use std::sync::Arc;
use anyhow::Result;

/// Represents that a user is logged in
pub struct LoggedIn;
impl<'a, 'r> FromRequest<'a, 'r> for LoggedIn {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match request.cookies().get_private("AUTH") {
            Some(_) => Outcome::Success(LoggedIn),
            None => Outcome::Forward(())
        }
    }
}

/// An HTML login page
#[get("/login")]
pub fn get_login(
    globe: State<Arc<GlobalContext>>
) -> Option<Html<Vec<u8>>> {
    let context = Context::new(globe.inner().clone());
    let mut out = std::io::Cursor::new(vec![]);
    templates::login(&mut out, &context, None).ok()?;
    Some(Html(out.into_inner()))
}

#[derive(FromForm)]
pub struct LoginForm { code: String }

#[post("/login", data = "<loginform>")]
pub fn post_login(
    mut cookies: Cookies,
    globe: State<Arc<GlobalContext>>,
    loginform: Form<LoginForm>
) -> rocket::response::Redirect {
    let code = loginform.code.parse().unwrap_or(0);
    if globe.use_login_token(code) {
        // Login successful
        cookies.add_private(Cookie::new("AUTH", ""));
    }
    rocket::response::Redirect::to("/")
}

#[post("/logout")]
pub fn logout(_user: LoggedIn, mut cookies: Cookies) -> Redirect {
    cookies.remove_private(Cookie::named("AUTH"));
    Redirect::to("/login")
}

#[get("/invite")]
pub fn invite(
    _user: LoggedIn,
    globe: State<Arc<GlobalContext>>,
) -> Option<Html<Vec<u8>>> {
    let context = Context::new(globe.inner().clone());
    let mut out = std::io::Cursor::new(vec![]);
    templates::invite(&mut out, &context, &globe.generate_login_token(15).to_string()).ok()?;
    Some(Html(out.into_inner()))
}
