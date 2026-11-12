//! Calendar days in local time, for reading "last week" or "в пятницу".

use crate::meeting::query::{iso_local, parse_time};

/// A local calendar day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Day {
    pub(super) year: i64,
    pub(super) month: u32,
    pub(super) day: u32,
}

impl Day {
    pub(super) fn new(year: i64, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// The local day `ms` falls on.
    pub(super) fn of(ms: u64) -> Self {
        let iso = iso_local(ms);
        let field = |range: std::ops::Range<usize>| {
            iso.get(range)
                .and_then(|text| text.parse::<i64>().ok())
                .unwrap_or(1)
        };
        Self::new(field(0..4), field(5..7) as u32, field(8..10) as u32)
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub(super) fn days(self) -> i64 {
        let year = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    pub(super) fn from_days(days: i64) -> Self {
        let days = days + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
        let month = if month_index < 10 {
            month_index + 3
        } else {
            month_index - 9
        } as u32;
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self::new(year, month, day)
    }

    pub(super) fn plus(self, days: i64) -> Self {
        Self::from_days(self.days() + days)
    }

    /// Monday is 0; 1970-01-01 was a Thursday.
    pub(super) fn weekday(self) -> i64 {
        (self.days() + 3).rem_euclid(7)
    }

    /// Local midnight starting this day.
    pub(super) fn midnight(self, now_ms: u64) -> Option<u64> {
        parse_time(
            &format!("{:04}-{:02}-{:02}", self.year, self.month, self.day),
            now_ms,
            false,
        )
        .ok()
    }
}

pub(super) fn span(start: Day, end: Day, now_ms: u64) -> Option<(u64, u64)> {
    Some((start.midnight(now_ms)?, end.midnight(now_ms)?))
}
