use super::PhotoLink;
use super::splitlist::links_by_time;
use {Group, Link};
use chrono::Duration as ChDuration;
use chrono::naive::{NaiveDate, NaiveDateTime};
use diesel::expression::sql_literal::SqlLiteral;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use models::Photo;
use nickel::{MiddlewareResult, QueryString, Request, Response};
use nickel::extensions::response::Redirect;
use nickel_diesel::DieselRequestExtensions;
use nickel_jwt_session::SessionRequestExtensions;
use server::nickelext::MyResponse;
use templates;
use time;


pub fn all_years<'mw>(
    req: &mut Request,
    res: Response<'mw>,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, grade};
    let c: &PgConnection = &req.db_conn();

    let groups: Vec<Group> = SqlLiteral::new(format!(
        "select cast(extract(year from date) as int) y, count(*) c \
         from photos{} group by y order by y desc nulls last",
        if req.authorized_user().is_none() {
            " where is_public"
        } else {
            ""
        }
    )).load::<(Option<i32>, i64)>(c)
        .unwrap()
        .iter()
        .map(|&(year, count)| {
            let q = Photo::query(req.authorized_user().is_some())
                .order((grade.desc().nulls_last(), date.asc()))
                .limit(1);
            let photo = if let Some(year) = year {
                q.filter(
                    date.ge(NaiveDate::from_ymd(year, 1, 1).and_hms(0, 0, 0)),
                ).filter(date.lt(
                        NaiveDate::from_ymd(year + 1, 1, 1).and_hms(0, 0, 0),
                    ))
            } else {
                q.filter(date.is_null())
            };
            Group {
                title: year.map(|y| format!("{}", y))
                    .unwrap_or_else(|| "-".to_string()),
                url: format!("/{}/", year.unwrap_or(0)),
                count: count,
                photo: photo.first::<Photo>(c).unwrap(),
            }
        })
        .collect();

    res.ok(|o| templates::groups(o, req, "All photos", &[], &groups))
}

pub fn months_in_year<'mw>(
    req: &mut Request,
    res: Response<'mw>,
    year: i32,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, grade};
    let c: &PgConnection = &req.db_conn();

    let title: String = format!("Photos from {}", year);
    let groups: Vec<Group> = SqlLiteral::new(format!(
        "select cast(extract(month from date) as int) m, count(*) c \
         from photos where extract(year from date)={}{} \
         group by m order by m desc",
        year,
        if req.authorized_user().is_none() {
            " and is_public"
        } else {
            ""
        }
    )).load::<(Option<i32>, i64)>(c)
        .unwrap()
        .iter()
        .map(|&(month, count)| {
            let month = month.map(|y| y as u32).unwrap_or(0);
            let fromdate =
                NaiveDate::from_ymd(year, month, 1).and_hms(0, 0, 0);
            let todate = if month == 12 {
                NaiveDate::from_ymd(year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd(year, month + 1, 1)
            }.and_hms(0, 0, 0);
            let photo = Photo::query(req.authorized_user().is_some())
                .filter(date.ge(fromdate))
                .filter(date.lt(todate))
                .order((grade.desc().nulls_last(), date.asc()))
                .limit(1)
                .first::<Photo>(c)
                .unwrap();

            Group {
                title: monthname(month).to_string(),
                url: format!("/{}/{}/", year, month),
                count: count,
                photo: photo,
            }
        })
        .collect();

    if groups.is_empty() {
        res.not_found("No such image")
    } else {
        res.ok(|o| templates::groups(o, req, &title, &[], &groups))
    }
}

pub fn days_in_month<'mw>(
    req: &mut Request,
    res: Response<'mw>,
    year: i32,
    month: u32,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, grade};
    let c: &PgConnection = &req.db_conn();

    let lpath: Vec<Link> = vec![Link::year(year)];
    let title: String = format!("Photos from {} {}", monthname(month), year);
    let groups: Vec<Group> = SqlLiteral::new(format!(
        "select cast(extract(day from date) as int) d, count(*) c \
         from photos where extract(year from date)={} \
         and extract(month from date)={}{} group by d order by d desc",
        year,
        month,
        if req.authorized_user().is_none() {
            " and is_public"
        } else {
            ""
        }
    )).load::<(Option<i32>, i64)>(c)
        .unwrap()
        .iter()
        .map(|&(day, count)| {
            let day = day.map(|y| y as u32).unwrap_or(0);
            let fromdate =
                NaiveDate::from_ymd(year, month, day).and_hms(0, 0, 0);
            let photo = Photo::query(req.authorized_user().is_some())
                .filter(date.ge(fromdate))
                .filter(date.lt(fromdate + ChDuration::days(1)))
                .order((grade.desc().nulls_last(), date.asc()))
                .limit(1)
                .first::<Photo>(c)
                .unwrap();

            Group {
                title: format!("{}", day),
                url: format!("/{}/{}/{}", year, month, day),
                count: count,
                photo: photo,
            }
        })
        .collect();

    if groups.is_empty() {
        res.not_found("No such image")
    } else {
        res.ok(|o| templates::groups(o, req, &title, &lpath, &groups))
    }
}

pub fn all_null_date<'mw>(
    req: &mut Request,
    res: Response<'mw>,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, path};

    let c: &PgConnection = &req.db_conn();
    res.ok(|o| {
        templates::index(
            o,
            req,
            "Photos without a date",
            &[],
            &Photo::query(req.authorized_user().is_some())
                .filter(date.is_null())
                .order(path.asc())
                .limit(500)
                .load(c)
                .unwrap()
                .iter()
                .map(PhotoLink::from)
                .collect::<Vec<_>>(),
        )
    })
}

pub fn all_for_day<'mw>(
    req: &mut Request,
    res: Response<'mw>,
    year: i32,
    month: u32,
    day: u32,
) -> MiddlewareResult<'mw> {
    let thedate = NaiveDate::from_ymd(year, month, day).and_hms(0, 0, 0);
    use schema::photos::dsl::date;

    let photos = Photo::query(req.authorized_user().is_some())
        .filter(date.ge(thedate))
        .filter(date.lt(thedate + ChDuration::days(1)));
    let links = links_by_time(req, photos);

    if links.is_empty() {
        res.not_found("No such image")
    } else {
        res.ok(|o| {
            templates::index(
                o,
                req,
                &format!("Photos from {} {} {}", day, monthname(month), year),
                &[Link::year(year), Link::month(year, month)],
                &links,
            )
        })
    }
}

pub fn on_this_day<'mw>(
    req: &mut Request,
    res: Response<'mw>,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, grade};
    let c: &PgConnection = &req.db_conn();

    let (month, day) = {
        let now = time::now();
        (now.tm_mon as u32 + 1, now.tm_mday as u32)
    };
    res.ok(|o| {
        templates::groups(
            o,
            req,
            &format!("Photos from {} {}", day, monthname(month)),
            &[],
            &SqlLiteral::new(format!(
                "select extract(year from date) y, count(*) c \
                 from photos where extract(month from date)={} \
                 and extract(day from date)={}{} group by y order by y desc",
                month,
                day,
                if req.authorized_user().is_none() {
                    " and is_public"
                } else {
                    ""
                }
            )).load::<(Option<f64>, i64)>(c)
                .unwrap()
                .iter()
                .map(|&(year, count)| {
                    let year = year.map(|y| y as i32).unwrap_or(0);
                    let fromdate = NaiveDate::from_ymd(
                        year,
                        month as u32,
                        day,
                    ).and_hms(0, 0, 0);
                    let photo = Photo::query(req.authorized_user().is_some())
                        .filter(date.ge(fromdate))
                        .filter(date.lt(fromdate + ChDuration::days(1)))
                        .order((grade.desc().nulls_last(), date.asc()))
                        .limit(1)
                        .first::<Photo>(c)
                        .unwrap();

                    Group {
                        title: format!("{}", year),
                        url: format!("/{}/{}/{}", year, month, day),
                        count: count,
                        photo: photo,
                    }
                })
                .collect::<Vec<_>>(),
        )
    })
}

pub fn next_image<'mw>(
    req: &mut Request,
    res: Response<'mw>,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, id};
    let q = Photo::query(req.authorized_user().is_some())
        .select(id)
        .filter(date.gt(from_date(req)))
        .order(date);
    let c: &PgConnection = &req.db_conn();
    if let Ok(photo) = q.first::<i32>(c) {
        res.redirect(format!("/img/{}", photo)) // to photo_details
    } else {
        res.not_found("No such image")
    }
}

pub fn prev_image<'mw>(
    req: &mut Request,
    res: Response<'mw>,
) -> MiddlewareResult<'mw> {
    use schema::photos::dsl::{date, id};
    let q = Photo::query(req.authorized_user().is_some())
        .select(id)
        .filter(date.lt(from_date(req)))
        .order(date.desc().nulls_last());
    let c: &PgConnection = &req.db_conn();
    if let Ok(photo) = q.first::<i32>(c) {
        res.redirect(format!("/img/{}", photo)) // to photo_details
    } else {
        res.not_found("No such image")
    }
}

fn from_date(req: &mut Request) -> Option<NaiveDateTime> {
    query_date(req, "from")
}
pub fn query_date(req: &mut Request, name: &str) -> Option<NaiveDateTime> {
    req.query()
        .get(name)
        .and_then(|s| s.parse().ok())
        .and_then(|i: i32| {
            use schema::photos::dsl::{date, photos};
            let c: &PgConnection = &req.db_conn();
            photos.find(i).select(date).first(c).unwrap_or(None)
        })
}

pub fn monthname(n: u32) -> &'static str {
    match n {
        1 => "january",
        2 => "february",
        3 => "march",
        4 => "april",
        5 => "may",
        6 => "june",
        7 => "july",
        8 => "august",
        9 => "september",
        10 => "october",
        11 => "november",
        12 => "december",
        _ => "non-month",
    }
}
