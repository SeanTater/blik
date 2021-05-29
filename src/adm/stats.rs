use crate::schema::media::dsl::media;
use anyhow::Result;
use diesel::expression::dsl::{count_star, sql};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable};
use diesel::sqlite::SqliteConnection;

pub fn show_stats(db: &SqliteConnection) -> Result<()> {
    println!(
        "There are {} photos in total.",
        media.select(count_star()).first::<i64>(db)?,
    );

    println!(
        "Count per year: {:?}",
        media
            .select(sql::<(Nullable<Double>, BigInt)>(
                "strftime('%Y', date) y, count(*)"
            ))
            .group_by(sql::<Nullable<Double>>("y"))
            .order(sql::<Nullable<Double>>("y").desc())
            .load::<(Option<f64>, i64)>(db)?
            .iter()
            .map(|&(y, n)| format!("{}: {}", y.unwrap_or(0.0), n))
            .collect::<Vec<_>>(),
    );

    Ok(())
}
