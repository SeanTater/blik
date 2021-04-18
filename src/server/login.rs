use super::{BuilderExt, Context, RenderRucte};
use crate::templates;
use log::info;
use serde::Deserialize;
use warp::http::header;
use warp::http::response::Builder;
use warp::reply::Response;


pub fn get_login(context: Context, param: NextQ) -> Response {
    info!("Got request for login form.  Param: {:?}", param);
    let next = sanitize_next(param.next.as_ref().map(AsRef::as_ref));
    Builder::new()
        .html(|o| templates::login(o, &context, next, None))
        .unwrap()
}

#[derive(Debug, Default, Deserialize)]
pub struct NextQ {
    next: Option<String>,
}

pub fn post_login(context: Context, form: LoginForm) -> Response {
    let next = sanitize_next(form.next.as_ref().map(AsRef::as_ref));
    let code = form.code.parse().unwrap_or(0);
    if context.global.use_login_token(code) {
        // Login successful
        let token = context.make_token(&form.code).unwrap();
        return Builder::new()
            .header(
                header::SET_COOKIE,
                format!("EXAUTH={}; SameSite=Strict; HttpOnly", token),
            )
            .redirect(next.unwrap_or("/"));
    } else {
        // Login failed
        let message = Some("Login failed, please try again");
        Builder::new()
            .html(|o| templates::login(o, &context, next, message))
            .unwrap()
    }
}

/// The data submitted by the login form.
/// This does not derive Debug or Serialize, as the password is plain text.
#[derive(Deserialize)]
pub struct LoginForm {
    pub code: String,
    next: Option<String>,
}

fn sanitize_next(next: Option<&str>) -> Option<&str> {
    if let Some(next) = next {
        use regex::Regex;
        let re = Regex::new(r"^/([a-z0-9._-]+/?)*$").unwrap();
        if re.is_match(next) {
            return Some(next);
        }
    }
    None
}

#[test]
fn test_sanitize_bad_1() {
    assert_eq!(None, sanitize_next(Some("https://evil.org/")))
}

#[test]
fn test_sanitize_bad_2() {
    assert_eq!(None, sanitize_next(Some("//evil.org/")))
}
#[test]
fn test_sanitize_bad_3() {
    assert_eq!(None, sanitize_next(Some("/evil\"hack")))
}
#[test]
fn test_sanitize_bad_4() {
    assert_eq!(None, sanitize_next(Some("/evil'hack")))
}

#[test]
fn test_sanitize_good_1() {
    assert_eq!(Some("/foo/"), sanitize_next(Some("/foo/")))
}
#[test]
fn test_sanitize_good_2() {
    assert_eq!(Some("/2017/7/15"), sanitize_next(Some("/2017/7/15")))
}

pub fn logout(_context: Context) -> Response {
    Builder::new()
        .header(
            header::SET_COOKIE,
            "EXAUTH=; Max-Age=0; SameSite=Strict; HttpOnly",
        )
        .redirect("/")
}
