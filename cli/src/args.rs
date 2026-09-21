//! Reading a date and a window off the command line.

use chrono::NaiveDate;
use sq_core::prelude::*;

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| Error::Invalid(format!("bad date {s:?}: {e}")))
}

/// Parse two optional dates or return the default range.
pub fn parse_range(args: &[String], default: DateRange) -> Result<DateRange> {
    match args {
        [] => Ok(default),
        [from, to] => Ok(DateRange::new(parse_date(from)?, parse_date(to)?)),
        _ => Err(Error::Invalid("usage: <command> [from] [to]".into())),
    }
}
