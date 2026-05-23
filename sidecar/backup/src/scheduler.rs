//! Cron-style scheduler that drives WAL-G base backup cycles.

// FEATURE: B1

use std::error::Error;
use std::fmt;
use std::time::Duration;

/// A minimal scheduler that interprets a five-field cron expression and
/// computes the next fire time relative to a UTC anchor minute.
///
/// The scheduler is intentionally restricted to the cron patterns produced by
/// the backup CRD: full-set wildcards, single integers, step values
/// (`*/N`), and fixed lists (`a,b,c`). It is sufficient for the
/// `Backup` CRD `schedule` field and avoids pulling in a third-party crate.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupSchedule {
    minute: ScheduleField,
    hour: ScheduleField,
    day_of_month: ScheduleField,
    month: ScheduleField,
    day_of_week: ScheduleField,
}

impl BackupSchedule {
    /// Parse a five-field cron expression as used by the `Backup` CRD.
    pub fn parse(expression: &str) -> Result<Self, ScheduleError> {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return Err(ScheduleError::Empty);
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(ScheduleError::InvalidExpression(trimmed.to_string()));
        }

        let minute = ScheduleField::parse(parts[0], 0, 59)?;
        let hour = ScheduleField::parse(parts[1], 0, 23)?;
        let day_of_month = ScheduleField::parse(parts[2], 1, 31)?;
        let month = ScheduleField::parse(parts[3], 1, 12)?;
        let day_of_week = ScheduleField::parse(parts[4], 0, 6)?;

        Ok(Self {
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
        })
    }

    /// Find the next minute that satisfies the schedule strictly after `from_epoch_minute`.
    pub fn next_after(&self, from_epoch_minute: u64) -> u64 {
        let mut candidate = from_epoch_minute.saturating_add(1);
        loop {
            let (minute, hour, day_of_month, month, day_of_week) =
                decompose_epoch_minute(candidate);
            if self.matches(minute, hour, day_of_month, month, day_of_week) {
                return candidate;
            }
            candidate = candidate.saturating_add(1);
            // Search horizon: avoid pathological infinite loops for impossible
            // combinations. 64 days is more than enough for any real schedule.
            if candidate.saturating_sub(from_epoch_minute) > 64 * 24 * 60 {
                return candidate;
            }
        }
    }

    /// Time delta from `from_epoch_minute` to the next scheduled minute.
    pub fn duration_until_next(&self, from_epoch_minute: u64) -> Duration {
        let next = self.next_after(from_epoch_minute);
        Duration::from_secs((next.saturating_sub(from_epoch_minute)) * 60)
    }

    fn matches(
        &self,
        minute: u32,
        hour: u32,
        day_of_month: u32,
        month: u32,
        day_of_week: u32,
    ) -> bool {
        self.minute.matches(minute)
            && self.hour.matches(hour)
            && self.day_of_month.matches(day_of_month)
            && self.month.matches(month)
            && self.day_of_week.matches(day_of_week)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ScheduleField {
    Wildcard,
    Step(u32),
    Values(Vec<u32>),
}

impl ScheduleField {
    fn parse(spec: &str, min: u32, max: u32) -> Result<Self, ScheduleError> {
        if spec == "*" {
            return Ok(Self::Wildcard);
        }
        if let Some(step_str) = spec.strip_prefix("*/") {
            let step = parse_field_int(step_str)?;
            if step == 0 {
                return Err(ScheduleError::InvalidExpression(spec.to_string()));
            }
            return Ok(Self::Step(step));
        }
        let mut values = Vec::new();
        for piece in spec.split(',') {
            let value = parse_field_int(piece)?;
            if value < min || value > max {
                return Err(ScheduleError::OutOfRange { value, min, max });
            }
            values.push(value);
        }
        values.sort_unstable();
        values.dedup();
        Ok(Self::Values(values))
    }

    fn matches(&self, value: u32) -> bool {
        match self {
            Self::Wildcard => true,
            Self::Step(step) => value % step == 0,
            Self::Values(values) => values.contains(&value),
        }
    }
}

fn parse_field_int(value: &str) -> Result<u32, ScheduleError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| ScheduleError::InvalidExpression(value.to_string()))
}

/// Decompose a UTC epoch minute into (minute, hour, day_of_month, month, day_of_week).
///
/// Day-of-week uses the cron convention: Sunday == 0.
pub fn decompose_epoch_minute(epoch_minute: u64) -> (u32, u32, u32, u32, u32) {
    let day = epoch_minute / 1_440;
    let in_day = epoch_minute % 1_440;
    let hour = (in_day / 60) as u32;
    let minute = (in_day % 60) as u32;
    // 1970-01-01 was a Thursday (4).
    let day_of_week = ((day + 4) % 7) as u32;
    let (year, month, day_of_month) = epoch_day_to_ymd(day);
    let _ = year;
    (minute, hour, day_of_month, month, day_of_week)
}

fn epoch_day_to_ymd(days_since_epoch: u64) -> (u32, u32, u32) {
    // Howard Hinnant's civil_from_days algorithm, adapted to u64 days.
    let z = days_since_epoch as i64 + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = (y + i64::from(if month <= 2 { 1 } else { 0 })) as u32;
    (year, month, day)
}

/// Errors raised while parsing or driving the schedule.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ScheduleError {
    Empty,
    InvalidExpression(String),
    OutOfRange { value: u32, min: u32, max: u32 },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "schedule must not be empty"),
            Self::InvalidExpression(spec) => {
                write!(formatter, "invalid schedule expression: {spec}")
            }
            Self::OutOfRange { value, min, max } => {
                write!(formatter, "schedule field {value} outside [{min}, {max}]")
            }
        }
    }
}

impl Error for ScheduleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_empty_or_short_expression() {
        assert_eq!(BackupSchedule::parse(""), Err(ScheduleError::Empty));
        assert!(matches!(
            BackupSchedule::parse("0 12"),
            Err(ScheduleError::InvalidExpression(_))
        ));
    }

    #[test]
    fn parse_accepts_six_hour_wildcard_form() {
        let schedule = BackupSchedule::parse("0 */6 * * *").expect("valid schedule");
        // Epoch minute 0 == 1970-01-01 00:00 UTC.
        // Next fire after 00:00 is 06:00 -> 360 minutes.
        assert_eq!(schedule.next_after(0), 360);
        // After 06:00 the next fire is 12:00.
        assert_eq!(schedule.next_after(360), 720);
    }

    #[test]
    fn parse_accepts_fixed_minute_list() {
        let schedule = BackupSchedule::parse("0,30 * * * *").expect("valid schedule");
        assert_eq!(schedule.next_after(0), 30);
        assert_eq!(schedule.next_after(30), 60);
    }

    #[test]
    fn out_of_range_minute_is_rejected() {
        match BackupSchedule::parse("60 0 * * *") {
            Err(ScheduleError::OutOfRange { value, min, max }) => {
                assert_eq!(value, 60);
                assert_eq!(min, 0);
                assert_eq!(max, 59);
            }
            other => panic!("expected out-of-range error, got {other:?}"),
        }
    }

    #[test]
    fn duration_until_next_is_at_least_one_minute() {
        let schedule = BackupSchedule::parse("0 */6 * * *").expect("valid schedule");
        let duration = schedule.duration_until_next(0);
        assert_eq!(duration, Duration::from_secs(360 * 60));
    }

    #[test]
    fn decompose_epoch_minute_handles_known_dates() {
        // 1970-01-01 00:00 UTC was Thursday (cron 4).
        let (minute, hour, day_of_month, month, day_of_week) = decompose_epoch_minute(0);
        assert_eq!(minute, 0);
        assert_eq!(hour, 0);
        assert_eq!(day_of_month, 1);
        assert_eq!(month, 1);
        assert_eq!(day_of_week, 4);
    }
}
