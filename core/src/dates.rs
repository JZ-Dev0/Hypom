use chrono::{NaiveDate, Local, Datelike};

/// Date order preference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateOrder {
    Dmy,
    Mdy,
}

impl Default for DateOrder {
    fn default() -> Self {
        Self::Dmy
    }
}

/// Try to parse a date string in the supported formats
pub fn parse_date(s: &str, order: &DateOrder) -> Option<NaiveDate> {
    let s = s.trim();
    
    // Try ISO format first: aaaa-mm-dd
    if let Some(d) = try_iso(s) {
        return Some(d);
    }

    // Try dd/mm/aaaa or dd-mm-aaaa
    if let Some(d) = try_dmy_full(s, order) {
        return Some(d);
    }

    // Try dd/mm (without year)
    if let Some(d) = try_dmy_short(s, order) {
        return Some(d);
    }

    None
}

fn try_iso(s: &str) -> Option<NaiveDate> {
    // aaaa-mm-dd
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn try_dmy_full(s: &str, order: &DateOrder) -> Option<NaiveDate> {
    // Try with / separator
    if let Some(d) = try_dmy_with_sep(s, '/', order) {
        return Some(d);
    }
    // Try with - separator
    if let Some(d) = try_dmy_with_sep(s, '-', order) {
        return Some(d);
    }
    None
}

fn try_dmy_with_sep(s: &str, sep: char, order: &DateOrder) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != 3 {
        return None;
    }
    let (day, month, year) = match order {
        DateOrder::Dmy => {
            let d: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let y: i32 = parts[2].parse().ok()?;
            (d, m, y)
        }
        DateOrder::Mdy => {
            let m: u32 = parts[0].parse().ok()?;
            let d: u32 = parts[1].parse().ok()?;
            let y: i32 = parts[2].parse().ok()?;
            (d, m, y)
        }
    };
    NaiveDate::from_ymd_opt(year, month, day)
}

fn try_dmy_short(s: &str, order: &DateOrder) -> Option<NaiveDate> {
    let sep = if s.contains('/') {
        '/'
    } else if s.contains('-') {
        '-'
    } else {
        return None;
    };

    let parts: Vec<&str> = s.split(sep).collect();
    if parts.len() != 2 {
        return None;
    }

    let (day, month) = match order {
        DateOrder::Dmy => {
            let d: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            (d, m)
        }
        DateOrder::Mdy => {
            let m: u32 = parts[0].parse().ok()?;
            let d: u32 = parts[1].parse().ok()?;
            (d, m)
        }
    };

    let year = Local::now().year();
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Calculate distance from today (in days). Positive = future, negative = past.
pub fn days_from_today(date: NaiveDate) -> i64 {
    let today = Local::now().date_naive();
    (date - today).num_days()
}

/// Sort key for "cercana" (closest): future dates first by proximity, then past dates by proximity
pub fn closest_sort_key(date: NaiveDate) -> (i32, i64) {
    let days = days_from_today(date);
    if days >= 0 {
        // Future: sort by ascending distance (closest future first)
        (0, days)
    } else {
        // Past: sort by ascending absolute distance (most recent past first)
        (1, -days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_date() {
        let d = parse_date("2026-11-12", &DateOrder::Dmy).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    }

    #[test]
    fn test_dmy_slash() {
        let d = parse_date("12/11/2026", &DateOrder::Dmy).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    }

    #[test]
    fn test_dmy_dash() {
        let d = parse_date("12-11-2026", &DateOrder::Dmy).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    }

    #[test]
    fn test_short_date() {
        let d = parse_date("12/11", &DateOrder::Dmy).unwrap();
        let year = Local::now().year();
        assert_eq!(d, NaiveDate::from_ymd_opt(year, 11, 12).unwrap());
    }

    #[test]
    fn test_invalid_date() {
        assert!(parse_date("32/13/2026", &DateOrder::Dmy).is_none());
    }

    #[test]
    fn test_not_a_date() {
        assert!(parse_date("hello", &DateOrder::Dmy).is_none());
    }
}
