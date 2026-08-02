//! Pure recurrence engine for the reminder system.
//!
//! All calculations use calendar dates in the configured local timezone. UTC
//! storage is converted to a local date before math, then converted back.

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ReminderStatus {
    Upcoming,
    Due,
    Overdue,
}

impl ReminderStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ReminderStatus::Upcoming => "upcoming",
            ReminderStatus::Due => "due",
            ReminderStatus::Overdue => "overdue",
        }
    }
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum RecurrenceError {
    #[error("invalid date: {0}")]
    InvalidDate(String),
    #[error("invalid anchor: {0}")]
    InvalidAnchor(String),
}

/// Derive the visible status of a reminder from its due date and optional snooze.
///
/// - Snoozed reminders report `upcoming` until `snoozed_until` (inclusive) has
///   passed, then re-derive from `due_date`.
/// - `overdue` = due date strictly before today.
/// - `due` = due date within 7 days inclusive of today, including today.
/// - `upcoming` = due date more than 7 days out.
#[allow(dead_code)]
pub fn derive_status(
    today: NaiveDate,
    due_date: NaiveDate,
    snoozed_until: Option<NaiveDate>,
) -> ReminderStatus {
    if let Some(snooze) = snoozed_until
        && today <= snooze
    {
        return ReminderStatus::Upcoming;
    }
    if due_date < today {
        ReminderStatus::Overdue
    } else if due_date.signed_duration_since(today).num_days() <= 7 {
        ReminderStatus::Due
    } else {
        ReminderStatus::Upcoming
    }
}

/// Compute a floating next due date: `completed_date + interval`.
///
/// Month arithmetic clamps to end of month (Jan 31 + 1 month = Feb 28/29).
#[allow(dead_code)]
pub fn next_due_floating(
    completed_date: NaiveDate,
    value: u32,
    unit: &str,
) -> Result<NaiveDate, RecurrenceError> {
    let value = value as i64;
    match unit {
        "day" => completed_date
            .checked_add_days(Days::new(value as u64))
            .ok_or_else(|| RecurrenceError::InvalidDate("overflow".to_string())),
        "week" => completed_date
            .checked_add_days(Days::new((value * 7) as u64))
            .ok_or_else(|| RecurrenceError::InvalidDate("overflow".to_string())),
        "month" => add_months(completed_date, value),
        "year" => add_months(completed_date, value * 12),
        _ => Err(RecurrenceError::InvalidDate(format!(
            "unknown unit: {}",
            unit
        ))),
    }
}

#[allow(dead_code)]
fn add_months(date: NaiveDate, months: i64) -> Result<NaiveDate, RecurrenceError> {
    let m = months
        .try_into()
        .map_err(|_| RecurrenceError::InvalidDate("overflow".to_string()))?;
    let target = date
        .checked_add_months(Months::new(m))
        .ok_or_else(|| RecurrenceError::InvalidDate("overflow".to_string()))?;
    let last_day = last_day_of_month(target.year(), target.month());
    if date.day() > last_day {
        NaiveDate::from_ymd_opt(target.year(), target.month(), last_day)
            .ok_or_else(|| RecurrenceError::InvalidDate("clamp failed".to_string()))
    } else {
        Ok(target)
    }
}

#[allow(dead_code)]
fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.map(|d| d.pred_opt().map(|p| p.day()).unwrap_or(28))
        .unwrap_or(28)
}

/// Compute the next fixed occurrence of a season anchor strictly after a given
/// reference date.
///
/// The anchor is "MM-DD". The result is the next anchor on or after the
/// reference date, but strictly after if the reference date is already the
/// anchor itself. For anchors on Feb 29 in non-leap years, resolve to Feb 28.
///
/// `fixed_interval_years` controls how many years to advance when the reference
/// date is already past the anchor in the current year.
#[allow(dead_code)]
pub fn next_due_fixed(
    reference: NaiveDate,
    anchor: &str,
    fixed_interval_years: u32,
) -> Result<NaiveDate, RecurrenceError> {
    let (month, day) = parse_anchor(anchor)?;
    let years = fixed_interval_years.max(1);

    let resolved_anchor = |year: i32| -> Result<NaiveDate, RecurrenceError> {
        let day = resolve_anchor_day(year, month, day);
        NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| RecurrenceError::InvalidAnchor(anchor.to_string()))
    };

    let current_year_anchor = resolved_anchor(reference.year())?;
    let mut candidate = if reference < current_year_anchor {
        current_year_anchor
    } else if reference == current_year_anchor {
        // Same day: next occurrence is the next scheduled cadence year.
        resolved_anchor(reference.year() + years as i32)?
    } else {
        // Past anchor in current year: next scheduled occurrence.
        let years_past =
            ((reference.year() - current_year_anchor.year()) as u32 / years + 1) * years;
        resolved_anchor(current_year_anchor.year() + years_past as i32)?
    };

    // Ensure strictly after reference in case the cadence math landed on it.
    while candidate <= reference {
        candidate = resolved_anchor(candidate.year() + years as i32)?;
    }
    Ok(candidate)
}

fn parse_anchor(anchor: &str) -> Result<(u32, u32), RecurrenceError> {
    let parts: Vec<&str> = anchor.split('-').collect();
    if parts.len() != 2 {
        return Err(RecurrenceError::InvalidAnchor(anchor.to_string()));
    }
    let month: u32 = parts[0]
        .parse()
        .map_err(|_| RecurrenceError::InvalidAnchor(anchor.to_string()))?;
    let day: u32 = parts[1]
        .parse()
        .map_err(|_| RecurrenceError::InvalidAnchor(anchor.to_string()))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(RecurrenceError::InvalidAnchor(anchor.to_string()));
    }
    Ok((month, day))
}

fn resolve_anchor_day(year: i32, month: u32, anchor_day: u32) -> u32 {
    if month == 2 && anchor_day == 29 && !is_leap_year(year) {
        28
    } else {
        anchor_day
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Convenience: parse an ISO date string to NaiveDate.
#[allow(dead_code)]
pub fn parse_date(s: &str) -> Result<NaiveDate, RecurrenceError> {
    s.parse::<NaiveDate>()
        .map_err(|e| RecurrenceError::InvalidDate(format!("{}: {}", s, e)))
}

/// Initial due date for a newly created task.
///
/// - If `last_done` is provided, compute the next occurrence after that date.
/// - Otherwise, for floating tasks use today + interval; for fixed tasks use
///   the next anchor after today.
#[allow(dead_code)]
pub fn initial_due_date(
    today: NaiveDate,
    last_done: Option<NaiveDate>,
    schedule_mode: &str,
    interval_value: Option<u32>,
    interval_unit: Option<&str>,
    season_anchor: Option<&str>,
    fixed_interval_years: u32,
) -> Result<NaiveDate, RecurrenceError> {
    match schedule_mode {
        "floating" => {
            let value = interval_value.unwrap_or(1);
            let unit = interval_unit.unwrap_or("month");
            let base = last_done.unwrap_or(today);
            next_due_floating(base, value, unit)
        }
        "fixed" => {
            let anchor = season_anchor.ok_or_else(|| {
                RecurrenceError::InvalidAnchor("missing season_anchor".to_string())
            })?;
            let base = last_done.unwrap_or(today);
            next_due_fixed(base, anchor, fixed_interval_years)
        }
        _ => Err(RecurrenceError::InvalidDate(format!(
            "unknown schedule mode: {}",
            schedule_mode
        ))),
    }
}

/// Convert a `DateTime<Local>` to its local calendar date.
#[allow(dead_code)]
pub fn to_local_date(dt: DateTime<Local>) -> NaiveDate {
    dt.date_naive()
}

/// Convert a local date to the start of that day in UTC.
#[allow(dead_code)]
pub fn from_local_date(date: NaiveDate) -> DateTime<Local> {
    date.and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floating_on_time() {
        let today = parse_date("2024-01-01").unwrap();
        let completed = parse_date("2024-06-15").unwrap();
        let next = next_due_floating(completed, 6, "month").unwrap();
        assert_eq!(next, parse_date("2024-12-15").unwrap());
        assert_eq!(derive_status(today, next, None), ReminderStatus::Upcoming);
    }

    #[test]
    fn test_floating_late_completion_shifts_schedule() {
        let today = parse_date("2024-01-01").unwrap();
        let completed = parse_date("2024-08-15").unwrap();
        let next = next_due_floating(completed, 6, "month").unwrap();
        assert_eq!(next, parse_date("2025-02-15").unwrap());
        assert_eq!(derive_status(today, next, None), ReminderStatus::Upcoming);
    }

    #[test]
    fn test_fixed_early_completion_no_shift() {
        // Pool closing Oct 1 against an Oct 15 anchor.
        let today = parse_date("2024-10-01").unwrap();
        let next = next_due_fixed(today, "10-15", 1).unwrap();
        assert_eq!(next, parse_date("2024-10-15").unwrap());
        // Due date is 14 days out, so status is upcoming per the 7-day rule.
        assert_eq!(derive_status(today, next, None), ReminderStatus::Upcoming);
        // But the fixed schedule did not pull earlier: it stayed on Oct 15.
    }

    #[test]
    fn test_fixed_on_anchor_goes_next_year() {
        let today = parse_date("2024-10-15").unwrap();
        let next = next_due_fixed(today, "10-15", 1).unwrap();
        assert_eq!(next, parse_date("2025-10-15").unwrap());
    }

    #[test]
    fn test_fixed_past_anchor_next_year() {
        let today = parse_date("2024-10-16").unwrap();
        let next = next_due_fixed(today, "10-15", 1).unwrap();
        assert_eq!(next, parse_date("2025-10-15").unwrap());
    }

    #[test]
    fn test_month_end_clamping() {
        let d = parse_date("2024-01-31").unwrap();
        let next = next_due_floating(d, 1, "month").unwrap();
        assert_eq!(next, parse_date("2024-02-29").unwrap());
    }

    #[test]
    fn test_leap_day_anchor_non_leap_year() {
        let today = parse_date("2023-01-01").unwrap();
        let next = next_due_fixed(today, "02-29", 1).unwrap();
        assert_eq!(next, parse_date("2023-02-28").unwrap());
    }

    #[test]
    fn test_leap_day_anchor_leap_year() {
        let today = parse_date("2024-01-01").unwrap();
        let next = next_due_fixed(today, "02-29", 1).unwrap();
        assert_eq!(next, parse_date("2024-02-29").unwrap());
    }

    #[test]
    fn test_fixed_every_two_years() {
        let today = parse_date("2024-03-01").unwrap();
        let next = next_due_fixed(today, "03-15", 2).unwrap();
        assert_eq!(next, parse_date("2024-03-15").unwrap());
    }

    #[test]
    fn test_fixed_every_two_years_after_anchor() {
        let today = parse_date("2024-03-16").unwrap();
        let next = next_due_fixed(today, "03-15", 2).unwrap();
        assert_eq!(next, parse_date("2026-03-15").unwrap());
    }

    #[test]
    fn test_snooze_delays_status() {
        let today = parse_date("2024-08-01").unwrap();
        let due = parse_date("2024-07-30").unwrap();
        assert_eq!(derive_status(today, due, None), ReminderStatus::Overdue);
        assert_eq!(
            derive_status(today, due, Some(parse_date("2024-08-05").unwrap())),
            ReminderStatus::Upcoming
        );
    }

    #[test]
    fn test_due_within_seven_days() {
        let today = parse_date("2024-08-01").unwrap();
        let due = parse_date("2024-08-07").unwrap();
        assert_eq!(derive_status(today, due, None), ReminderStatus::Due);
    }

    #[test]
    fn test_overdue_boundary() {
        let today = parse_date("2024-08-01").unwrap();
        let due = parse_date("2024-07-31").unwrap();
        assert_eq!(derive_status(today, due, None), ReminderStatus::Overdue);
    }

    #[test]
    fn test_initial_due_floating_without_last_done() {
        let today = parse_date("2024-08-01").unwrap();
        let due =
            initial_due_date(today, None, "floating", Some(3), Some("month"), None, 1).unwrap();
        assert_eq!(due, parse_date("2024-11-01").unwrap());
    }

    #[test]
    fn test_initial_due_fixed_without_last_done() {
        let today = parse_date("2024-08-01").unwrap();
        let due = initial_due_date(today, None, "fixed", None, None, Some("10-15"), 1).unwrap();
        assert_eq!(due, parse_date("2024-10-15").unwrap());
    }

    #[test]
    fn test_initial_due_fixed_with_last_done_this_year() {
        let last_done = parse_date("2024-05-01").unwrap();
        let due = initial_due_date(
            last_done,
            Some(last_done),
            "fixed",
            None,
            None,
            Some("10-15"),
            1,
        )
        .unwrap();
        assert_eq!(due, parse_date("2024-10-15").unwrap());
    }
}
