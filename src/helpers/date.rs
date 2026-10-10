use chrono::{Datelike, Duration, NaiveDate};
use thiserror::Error;

/// Weekly note timing options.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum When {
    LastWeek,
    ThisWeek,
    NextWeek,
}

/// Daily note timing options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DailyWhen {
    Yesterday,
    Today,
    Tomorrow,
    Date(NaiveDate),
}

/// Errors from invalid when options.
#[derive(Error, Debug, PartialEq)]
#[error("invalid When option, must be one of |thisWeek|nextWeek|lastWeek|")]
pub struct InvalidWhenError;

#[derive(Error, Debug, PartialEq)]
#[error(
    "invalid when option, must be yesterday, today, tomorrow, or a valid YYYY-MM-DD date (years 0001–9999)"
)]
pub struct InvalidDailyWhenError;

impl When {
    /// Parse a when option from a string.
    pub fn from_str(s: &str) -> Result<Self, InvalidWhenError> {
        match s {
            "lastWeek" => Ok(When::LastWeek),
            "thisWeek" => Ok(When::ThisWeek),
            "nextWeek" => Ok(When::NextWeek),
            _ => Err(InvalidWhenError),
        }
    }
}

impl DailyWhen {
    /// Parse a daily when option from a string.
    pub fn from_str(s: &str) -> Result<Self, InvalidDailyWhenError> {
        match s {
            "yesterday" => Ok(DailyWhen::Yesterday),
            "today" => Ok(DailyWhen::Today),
            "tomorrow" => Ok(DailyWhen::Tomorrow),
            _ => {
                let bytes = s.as_bytes();
                if bytes.len() != 10
                    || bytes.iter().enumerate().any(|(index, byte)| {
                        if index == 4 || index == 7 {
                            *byte != b'-'
                        } else {
                            !byte.is_ascii_digit()
                        }
                    })
                {
                    return Err(InvalidDailyWhenError);
                }
                let date =
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| InvalidDailyWhenError)?;
                if date.year() == 0 {
                    return Err(InvalidDailyWhenError);
                }
                Ok(DailyWhen::Date(date))
            }
        }
    }
}

/// Returns the Monday of the week relative to the given date.
///
/// Weeks start on Monday. For `ThisWeek`, returns the Monday of the current week.
/// For `LastWeek` and `NextWeek`, shifts by 7 days.
pub fn date_from_when(date: NaiveDate, when: When) -> NaiveDate {
    let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);

    match when {
        When::LastWeek => monday - Duration::days(7),
        When::NextWeek => monday + Duration::days(7),
        When::ThisWeek => monday,
    }
}

/// Returns the relative or explicitly selected date for daily notes.
pub fn date_from_daily_when(date: NaiveDate, when: DailyWhen) -> NaiveDate {
    match when {
        DailyWhen::Yesterday => date - Duration::days(1),
        DailyWhen::Tomorrow => date + Duration::days(1),
        DailyWhen::Today => date,
        DailyWhen::Date(date) => date,
    }
}

/// Returns the path part and filename for a note.
///
/// Path part is `YYYY/MM`, filename is `YYYY-MM-DD-{suffix}.{ext}`.
pub fn name_from_date(date: NaiveDate, suffix: &str, ext: &str) -> (String, String) {
    let path_part = format!("{:04}/{:02}", date.year(), date.month());
    let file_name = format!(
        "{:04}-{:02}-{:02}-{}.{}",
        date.year(),
        date.month(),
        date.day(),
        suffix,
        ext
    );
    (path_part, file_name)
}

/// Returns a formatted date string for note headers.
///
/// Format: "Monday 28 July 2025"
pub fn date_for_header(date: NaiveDate) -> String {
    format!(
        "{} {:02} {} {:04}",
        weekday_full_name(date.weekday()),
        date.day(),
        month_name(date.month()),
        date.year()
    )
}

fn weekday_full_name(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
}

/// Generates a batch of dates spaced 7 days apart.
pub fn get_batch_dates(start_date: NaiveDate, batch_size: usize) -> Vec<NaiveDate> {
    (0..batch_size)
        .map(|i| start_date + Duration::days(i as i64 * 7))
        .collect()
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_from_when_this_week() {
        let wed = NaiveDate::from_ymd_opt(2025, 7, 30).unwrap(); // Wednesday
        let mon = date_from_when(wed, When::ThisWeek);
        assert_eq!(mon, NaiveDate::from_ymd_opt(2025, 7, 28).unwrap());
    }

    #[test]
    fn test_date_from_when_last_week() {
        let wed = NaiveDate::from_ymd_opt(2025, 7, 30).unwrap();
        let mon = date_from_when(wed, When::LastWeek);
        assert_eq!(mon, NaiveDate::from_ymd_opt(2025, 7, 21).unwrap());
    }

    #[test]
    fn test_date_from_when_next_week() {
        let wed = NaiveDate::from_ymd_opt(2025, 7, 30).unwrap();
        let mon = date_from_when(wed, When::NextWeek);
        assert_eq!(mon, NaiveDate::from_ymd_opt(2025, 8, 4).unwrap());
    }

    #[test]
    fn test_date_from_when_monday() {
        let mon = NaiveDate::from_ymd_opt(2025, 7, 28).unwrap();
        let result = date_from_when(mon, When::ThisWeek);
        assert_eq!(result, mon);
    }

    #[test]
    fn test_date_from_daily_when() {
        let today = NaiveDate::from_ymd_opt(2025, 7, 28).unwrap();
        assert_eq!(date_from_daily_when(today, DailyWhen::Today), today);
        assert_eq!(
            date_from_daily_when(today, DailyWhen::Yesterday),
            NaiveDate::from_ymd_opt(2025, 7, 27).unwrap()
        );
        assert_eq!(
            date_from_daily_when(today, DailyWhen::Tomorrow),
            NaiveDate::from_ymd_opt(2025, 7, 29).unwrap()
        );
    }

    #[test]
    fn test_name_from_date() {
        let date = NaiveDate::from_ymd_opt(2025, 7, 28).unwrap();
        let (path, name) = name_from_date(date, "Weekly-log", "md");
        assert_eq!(path, "2025/07");
        assert_eq!(name, "2025-07-28-Weekly-log.md");
    }

    #[test]
    fn test_date_for_header() {
        let date = NaiveDate::from_ymd_opt(2025, 7, 28).unwrap();
        assert_eq!(date_for_header(date), "Monday 28 July 2025");
    }

    #[test]
    fn test_get_batch_dates() {
        let start = NaiveDate::from_ymd_opt(2025, 7, 28).unwrap();
        let dates = get_batch_dates(start, 3);
        assert_eq!(dates.len(), 3);
        assert_eq!(dates[0], start);
        assert_eq!(dates[1], NaiveDate::from_ymd_opt(2025, 8, 4).unwrap());
        assert_eq!(dates[2], NaiveDate::from_ymd_opt(2025, 8, 11).unwrap());
    }

    #[test]
    fn test_when_from_str() {
        assert_eq!(When::from_str("thisWeek").unwrap(), When::ThisWeek);
        assert_eq!(When::from_str("lastWeek").unwrap(), When::LastWeek);
        assert_eq!(When::from_str("nextWeek").unwrap(), When::NextWeek);
        assert!(When::from_str("invalid").is_err());
    }

    #[test]
    fn explicit_daily_dates_are_independent_of_today() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        for value in [
            "0001-01-01",
            "2024-02-29",
            "2025-12-31",
            "2027-01-01",
            "9999-12-31",
        ] {
            let expected = NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap();
            let when = DailyWhen::from_str(value).unwrap();
            assert_eq!(when, DailyWhen::Date(expected));
            assert_eq!(date_from_daily_when(today, when), expected);
        }
    }

    #[test]
    fn rejects_invalid_explicit_daily_dates() {
        for value in [
            "",
            "2025-02-29",
            "2026-04-31",
            "2026-00-10",
            "2026-13-01",
            "2026-01-00",
            "2026-1-05",
            "2026-01-5",
            "26-01-05",
            "0000-01-01",
            "+2026-01-01",
            "10000-01-01",
            "2026/01/01",
            " 2026-01-01",
            "2026-01-01 ",
            "2026-01-01T00:00:00",
            "２０２６-01-01",
        ] {
            assert!(DailyWhen::from_str(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn test_daily_when_from_str() {
        assert_eq!(DailyWhen::from_str("today").unwrap(), DailyWhen::Today);
        assert_eq!(
            DailyWhen::from_str("yesterday").unwrap(),
            DailyWhen::Yesterday
        );
        assert_eq!(
            DailyWhen::from_str("tomorrow").unwrap(),
            DailyWhen::Tomorrow
        );
        assert!(DailyWhen::from_str("invalid").is_err());
    }
}
