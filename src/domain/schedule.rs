use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ScheduleMode {
    Floating,
    Fixed,
}

impl fmt::Display for ScheduleMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleMode::Floating => write!(f, "floating"),
            ScheduleMode::Fixed => write!(f, "fixed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum IntervalUnit {
    Day,
    Week,
    Month,
    Year,
}

impl fmt::Display for IntervalUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntervalUnit::Day => write!(f, "day"),
            IntervalUnit::Week => write!(f, "week"),
            IntervalUnit::Month => write!(f, "month"),
            IntervalUnit::Year => write!(f, "year"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Schedule {
    Floating {
        interval_value: u32,
        interval_unit: IntervalUnit,
    },
    Fixed {
        season_anchor: String,
        fixed_interval_years: u32,
    },
}

impl Schedule {
    #[allow(dead_code)]
    pub fn mode(&self) -> ScheduleMode {
        match self {
            Schedule::Floating { .. } => ScheduleMode::Floating,
            Schedule::Fixed { .. } => ScheduleMode::Fixed,
        }
    }

    #[allow(dead_code)]
    pub fn to_db_mode(&self) -> String {
        self.mode().to_string()
    }
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ScheduleError {
    #[error("invalid season anchor: {0}")]
    InvalidAnchor(String),
}

/// Parse a season anchor like "10-15" into (month, day), validating the date.
#[allow(dead_code)]
pub fn parse_anchor(anchor: &str) -> Result<(u32, u32), ScheduleError> {
    let parts: Vec<&str> = anchor.split('-').collect();
    if parts.len() != 2 {
        return Err(ScheduleError::InvalidAnchor(anchor.to_string()));
    }
    let month: u32 = parts[0]
        .parse()
        .map_err(|_| ScheduleError::InvalidAnchor(anchor.to_string()))?;
    let day: u32 = parts[1]
        .parse()
        .map_err(|_| ScheduleError::InvalidAnchor(anchor.to_string()))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ScheduleError::InvalidAnchor(anchor.to_string()));
    }
    Ok((month, day))
}

/// Human-readable description of a schedule.
#[allow(dead_code)]
pub fn describe_schedule(
    mode: &str,
    interval_value: Option<i64>,
    interval_unit: Option<&str>,
    season_anchor: Option<&str>,
    fixed_interval_years: Option<i64>,
) -> String {
    match mode {
        "floating" => {
            let value = interval_value.unwrap_or(1) as u32;
            let unit = interval_unit.unwrap_or("month");
            let plural = if value == 1 { "" } else { "s" };
            format!("every {} {}{} after completion", value, unit, plural)
        }
        "fixed" => {
            let anchor = season_anchor.unwrap_or("?");
            let years = fixed_interval_years.unwrap_or(1) as u32;
            if years == 1 {
                format!("every year on {}", anchor)
            } else {
                format!("every {} years on {}", years, anchor)
            }
        }
        _ => "unknown schedule".to_string(),
    }
}
