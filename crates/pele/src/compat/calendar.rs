//! Host calendar boundary. Portable default is proleptic Gregorian UTC.
pub(crate) mod date_parser;
use serde::{Deserialize, Serialize};

/// Normalized civil time; months are 1..12. Constructor year quirks and field
/// overflow belong to the diagram's date logic, before invoking a host calendar.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CivilDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarFields {
    pub civil: CivilDateTime,
    /// Sunday=0, matching JavaScript Date.getDay().
    pub weekday: u8,
    /// Minutes east of UTC; opposite sign to Date.getTimezoneOffset().
    pub utc_offset_minutes: i32,
}
const DAY: i64 = 86_400_000;
const TIME_CLIP: f64 = 8.64e15;

pub(crate) fn utc_fields(instant: f64) -> Option<CalendarFields> {
    if instant.abs() > TIME_CLIP {
        return None;
    }
    coordinate_fields(instant)
}
fn coordinate_fields(instant: f64) -> Option<CalendarFields> {
    if !instant.is_finite() || instant.abs() > TIME_CLIP + 2.0 * DAY as f64 {
        return None;
    }
    let instant = instant.trunc() as i64;
    let days = instant.div_euclid(DAY);
    let remainder = instant.rem_euclid(DAY);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Some(CalendarFields {
        civil: CivilDateTime {
            year: year as i32,
            month: month as u8,
            day: day as u8,
            hour: (remainder / 3_600_000) as u8,
            minute: (remainder / 60_000 % 60) as u8,
            second: (remainder / 1000 % 60) as u8,
            millisecond: (remainder % 1000) as u16,
        },
        weekday: (days + 4).rem_euclid(7) as u8,
        utc_offset_minutes: 0,
    })
}
pub(crate) fn utc_instant(c: CivilDateTime) -> Option<f64> {
    civil_coordinate(c).filter(|v| v.abs() <= TIME_CLIP)
}
fn civil_coordinate(c: CivilDateTime) -> Option<f64> {
    if i64::from(c.year).abs() > 300_000
        || c.month == 0
        || c.month > 12
        || c.day == 0
        || c.hour > 23
        || c.minute > 59
        || c.second > 59
        || c.millisecond > 999
    {
        return None;
    }
    let leap =
        c.year.rem_euclid(4) == 0 && (c.year.rem_euclid(100) != 0 || c.year.rem_euclid(400) == 0);
    let last = match c.month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if c.day > last {
        return None;
    }
    let year = i64::from(c.year) - i64::from(c.month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = i64::from(c.month) + if c.month > 2 { -3 } else { 9 };
    let doy = (153 * month + 2) / 5 + i64::from(c.day) - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    let instant = days * DAY
        + i64::from(c.hour) * 3_600_000
        + i64::from(c.minute) * 60_000
        + i64::from(c.second) * 1000
        + i64::from(c.millisecond);
    ((instant as f64).abs() <= TIME_CLIP + 2.0 * DAY as f64).then_some(instant as f64)
}

/// Proleptic Gregorian local fields from an instant and host timezone offset.
/// Offsets retain historical seconds; axis-format minutes truncate as JS Date.
pub fn fields_at_offset(instant: f64, offset_seconds: i32) -> Option<CalendarFields> {
    if !instant.is_finite() || instant.abs() > TIME_CLIP || offset_seconds.unsigned_abs() > 86_400 {
        return None;
    }
    let mut fields = coordinate_fields(instant.trunc() + f64::from(offset_seconds) * 1000.0)?;
    fields.utc_offset_minutes = offset_seconds / 60;
    Some(fields)
}
/// JavaScript-compatible local-time resolution: choose the first repeated time;
/// advance through a gap using its preceding offset. Civil arithmetic stays Rust.
pub fn instant_in_zone<E>(
    civil: CivilDateTime,
    mut offset: impl FnMut(f64) -> Result<Option<i32>, E>,
) -> Result<Option<f64>, E> {
    let Some(local) = civil_coordinate(civil) else {
        return Ok(None);
    };
    let mut offsets = Vec::new();
    for delta in [
        -172_800_000.0,
        -86_400_000.0,
        0.0,
        86_400_000.0,
        172_800_000.0,
    ] {
        let Some(value) = offset((local + delta).clamp(-TIME_CLIP, TIME_CLIP))? else {
            return Ok(None);
        };
        if value.unsigned_abs() > 86_400 {
            return Ok(None);
        }
        if !offsets.contains(&value) {
            offsets.push(value);
        }
    }
    let mut first: Option<f64> = None;
    for value in &offsets {
        let candidate = local - f64::from(*value) * 1000.0;
        if candidate.abs() > TIME_CLIP {
            continue;
        }
        if offset(candidate)? == Some(*value) && first.is_none_or(|previous| candidate < previous) {
            first = Some(candidate);
        }
    }
    if first.is_some() {
        return Ok(first);
    }
    let candidate = local - f64::from(offsets[0]) * 1000.0;
    Ok((candidate.abs() <= TIME_CLIP).then_some(candidate))
}
