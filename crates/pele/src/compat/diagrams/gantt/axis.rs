//! Pele gantt/axis.ts d3-time/calendar/tick-format translation, 458322a5 (MIT, kepano).
use super::dates::{Calendar, DAY, MONTHS, WEEKDAYS};
use regex::Regex;
use std::sync::OnceLock;
const WEEK: f64 = 7. * DAY;
const YEAR: f64 = 365. * DAY;
#[derive(Clone, Copy)]
struct Interval {
    unit: &'static str,
    step: f64,
    week_start: u8,
}
fn cycle(unit: &str) -> f64 {
    match unit {
        "second" | "minute" => 60.,
        "hour" => 24.,
        "day" => 31.,
        "month" => 12.,
        _ => f64::INFINITY,
    }
}
fn unit_ms(unit: &str) -> f64 {
    match unit {
        "millisecond" => 1.,
        "second" => 1000.,
        "minute" => 60000.,
        "hour" => 3600000.,
        "day" => DAY,
        "week" => WEEK,
        "month" => 30. * DAY,
        _ => YEAR,
    }
}
fn floor_base(cal: &mut Calendar<'_>, d: f64, unit: &str, week_start: u8) -> f64 {
    let Some(f) = cal.fields(d) else {
        return f64::NAN;
    };
    let c = f.civil;
    match unit {
        "millisecond" => d,
        "second" => d - c.millisecond as f64,
        "minute" => d - c.millisecond as f64 - c.second as f64 * 1000.,
        "hour" => d - c.millisecond as f64 - c.second as f64 * 1000. - c.minute as f64 * 60000.,
        "day" => cal.today(d),
        "week" => {
            let shifted = cal.add_days(d, -((f.weekday as i32 + 7 - week_start as i32) % 7) as f64);
            cal.today(shifted)
        }
        "month" => cal.make(
            c.year as f64,
            c.month as f64 - 1.,
            1.,
            0.,
            0.,
            0.,
            0.,
            false,
        ),
        _ => cal.make(c.year as f64, 0., 1., 0., 0., 0., 0., false),
    }
}
fn offset_base(cal: &mut Calendar<'_>, d: f64, unit: &str, n: f64) -> f64 {
    match unit {
        "millisecond" => super::dates::clip(d + n),
        "second" => super::dates::clip(d + n * 1000.),
        "minute" => super::dates::clip(d + n * 60000.),
        "hour" => super::dates::clip(d + n * 3600000.),
        "day" => cal.add_days(d, n),
        "week" => cal.add_days(d, n * 7.),
        _ => {
            let Some(f) = cal.fields(d) else {
                return f64::NAN;
            };
            let c = f.civil;
            cal.make(
                c.year as f64 + if unit == "year" { n } else { 0. },
                c.month as f64 - 1. + if unit == "month" { n } else { 0. },
                c.day as f64,
                c.hour as f64,
                c.minute as f64,
                c.second as f64,
                c.millisecond as f64,
                false,
            )
        }
    }
}
fn field(cal: &mut Calendar<'_>, d: f64, unit: &str) -> f64 {
    cal.fields(d).map_or(f64::NAN, |f| match unit {
        "second" => f.civil.second as f64,
        "minute" => f.civil.minute as f64,
        "hour" => f.civil.hour as f64,
        "day" => f.civil.day as f64 - 1.,
        "month" => f.civil.month as f64 - 1.,
        _ => 0.,
    })
}
fn span(cal: &mut Calendar<'_>, start: f64, end: f64, unit: f64) -> f64 {
    let a = cal.fields(start).map_or(0, |f| f.utc_offset_minutes);
    let b = cal.fields(end).map_or(0, |f| f.utc_offset_minutes);
    ((end - start + (b - a) as f64 * 60000.) / unit).floor()
}
impl Interval {
    fn floor(&self, cal: &mut Calendar<'_>, date: f64) -> f64 {
        if self.step <= 1. {
            return floor_base(cal, date, self.unit, self.week_start);
        }
        match self.unit {
            "millisecond" => (date / self.step).floor() * self.step,
            "year" => {
                let Some(f) = cal.fields(date) else {
                    return f64::NAN;
                };
                let d = cal.make(
                    (f.civil.year as f64 / self.step).floor() * self.step,
                    f.civil.month as f64 - 1.,
                    f.civil.day as f64,
                    f.civil.hour as f64,
                    f.civil.minute as f64,
                    f.civil.second as f64,
                    f.civil.millisecond as f64,
                    false,
                );
                floor_base(cal, d, "year", 0)
            }
            "week" => {
                let epoch = floor_base(cal, 0., "week", self.week_start);
                let d = floor_base(cal, date, "week", self.week_start);
                let shift = -span(cal, epoch, d, WEEK).rem_euclid(self.step);
                offset_base(cal, d, "week", shift)
            }
            _ => {
                let mut d = floor_base(cal, date, self.unit, self.week_start);
                while d.is_finite() && field(cal, d, self.unit) % self.step != 0. {
                    d = floor_base(cal, d - 1., self.unit, self.week_start)
                }
                d
            }
        }
    }
    fn offset(&self, cal: &mut Calendar<'_>, date: f64, n: f64) -> f64 {
        if self.step <= 1. {
            return offset_base(cal, date, self.unit, n);
        }
        if matches!(self.unit, "millisecond" | "year" | "week") {
            return offset_base(cal, date, self.unit, n * self.step);
        }
        let mut d = date;
        for _ in 0..n as usize {
            loop {
                d = offset_base(cal, d, self.unit, 1.);
                if !d.is_finite() || field(cal, d, self.unit) % self.step == 0. {
                    break;
                }
            }
        }
        d
    }
}
fn range(
    cal: &mut Calendar<'_>,
    interval: Interval,
    start: f64,
    stop: f64,
    limit: usize,
) -> Option<Vec<f64>> {
    let mut d = interval.floor(cal, start - 1.);
    d = interval.offset(cal, d, 1.);
    d = interval.floor(cal, d);
    let mut out = vec![];
    let mut previous = f64::NEG_INFINITY;
    while d <= stop && d > previous {
        if out.len() >= limit {
            return None;
        }
        previous = d;
        out.push(d);
        d = interval.offset(cal, d, 1.);
        d = interval.floor(cal, d)
    }
    Some(out)
}
fn tick_step(start: f64, stop: f64, count: f64) -> f64 {
    let step = (stop - start) / count.max(0.);
    let power = step.log10().floor();
    let error = step / 10f64.powf(power);
    let factor = if error >= 50f64.sqrt() {
        10.
    } else if error >= 10f64.sqrt() {
        5.
    } else if error >= std::f64::consts::SQRT_2 {
        2.
    } else {
        1.
    };
    10f64.powf(power) * factor
}
pub fn auto_ticks(cal: &mut Calendar<'_>, start: f64, stop: f64, count: f64) -> Vec<f64> {
    let auto = [
        ("second", 1.),
        ("second", 5.),
        ("second", 15.),
        ("second", 30.),
        ("minute", 1.),
        ("minute", 5.),
        ("minute", 15.),
        ("minute", 30.),
        ("hour", 1.),
        ("hour", 3.),
        ("hour", 6.),
        ("hour", 12.),
        ("day", 1.),
        ("day", 2.),
        ("week", 1.),
        ("month", 1.),
        ("month", 3.),
        ("year", 1.),
    ];
    let target = (stop - start) / count;
    let i = auto
        .iter()
        .position(|(u, s)| unit_ms(u) * s > target)
        .unwrap_or(auto.len());
    let (unit, step) = if i == auto.len() {
        ("year", tick_step(start / YEAR, stop / YEAR, count).max(1.))
    } else if i == 0 {
        ("millisecond", tick_step(start, stop, count).max(1.))
    } else {
        auto[if target / (unit_ms(auto[i - 1].0) * auto[i - 1].1)
            < unit_ms(auto[i].0) * auto[i].1 / target
        {
            i - 1
        } else {
            i
        }]
    };
    range(
        cal,
        Interval {
            unit,
            step,
            week_start: 0,
        },
        start,
        stop,
        (4. * count + 8.) as usize,
    )
    .unwrap_or_default()
}
pub fn interval_ticks(
    cal: &mut Calendar<'_>,
    interval: &str,
    weekday: &str,
    start: f64,
    stop: f64,
    limit: usize,
) -> Option<Vec<f64>> {
    static R: OnceLock<Regex> = OnceLock::new();
    let cap = R
        .get_or_init(|| {
            Regex::new(r"^([1-9][0-9]*)(millisecond|second|minute|hour|day|week|month)$").unwrap()
        })
        .captures(interval)?;
    let step = cap[1].parse::<f64>().ok()?;
    let unit = match &cap[2] {
        "millisecond" => "millisecond",
        "second" => "second",
        "minute" => "minute",
        "hour" => "hour",
        "day" => "day",
        "week" => "week",
        _ => "month",
    };
    let week_start = WEEKDAYS
        .iter()
        .position(|d| d.to_lowercase() == weekday)
        .unwrap_or(0) as u8;
    if (stop - start) / (step.min(cycle(unit)) * unit_ms(unit)) > limit as f64 {
        return None;
    }
    range(
        cal,
        Interval {
            unit,
            step,
            week_start,
        },
        start,
        stop,
        limit,
    )
}
fn pad(value: i64, fill: &str, width: usize) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.abs().to_string();
    format!(
        "{sign}{}{digits}",
        fill.repeat(width.saturating_sub(digits.len()))
    )
}
fn week_of_year(cal: &mut Calendar<'_>, date: f64, week_start: u8) -> i64 {
    let year_start = floor_base(cal, date, "year", 0);
    let a = floor_base(cal, year_start - 1., "week", week_start);
    let b = floor_base(cal, date, "week", week_start);
    span(cal, a, b, WEEK) as i64
}
fn iso_thursday(cal: &mut Calendar<'_>, date: f64) -> f64 {
    let Some(f) = cal.fields(date) else {
        return f64::NAN;
    };
    let okay = f.weekday >= 4 || f.weekday == 0;
    let mut d = floor_base(cal, if okay { date } else { date - 1. }, "week", 4);
    if !okay {
        d = offset_base(cal, d, "week", 1.);
        d = floor_base(cal, d, "week", 4)
    }
    d
}
pub fn time_format(cal: &mut Calendar<'_>, specifier: &str, date: f64) -> String {
    let chars: Vec<char> = specifier.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1;
        let mut c = chars.get(i).copied().unwrap_or('\0');
        let fill = match c {
            '-' => Some(""),
            '_' => Some(" "),
            '0' => Some("0"),
            _ => None,
        };
        if fill.is_some() {
            i += 1;
            c = chars.get(i).copied().unwrap_or('\0')
        }
        let fill = fill.unwrap_or(if c == 'e' { " " } else { "0" });
        out += &directive(cal, c, date, fill);
        i += 1
    }
    out
}
fn directive(cal: &mut Calendar<'_>, token: char, date: f64, p: &str) -> String {
    let Some(f) = cal.fields(date) else {
        return "NaN".into();
    };
    let c = f.civil;
    match token {
        'a' => WEEKDAYS[f.weekday as usize][..3].into(),
        'A' => WEEKDAYS[f.weekday as usize].into(),
        'b' => MONTHS[c.month as usize - 1][..3].into(),
        'B' => MONTHS[c.month as usize - 1].into(),
        'c' => time_format(cal, "%x, %X", date),
        'd' | 'e' => pad(c.day as i64, p, 2),
        'f' => format!("{}000", pad(c.millisecond as i64, p, 3)),
        'g' | 'G' => {
            let d = iso_thursday(cal, date);
            let y = cal.fields(d).map_or(0, |f| f.civil.year as i64);
            if token == 'g' {
                pad(y % 100, p, 2)
            } else {
                pad(y % 10000, p, 4)
            }
        }
        'H' => pad(c.hour as i64, p, 2),
        'I' => pad(if c.hour % 12 == 0 { 12 } else { c.hour % 12 } as i64, p, 2),
        'j' => {
            let a = floor_base(cal, date, "year", 0);
            let b = floor_base(cal, date, "day", 0);
            pad(1 + span(cal, a, b, DAY) as i64, p, 3)
        }
        'L' => pad(c.millisecond as i64, p, 3),
        'm' => pad(c.month as i64, p, 2),
        'M' => pad(c.minute as i64, p, 2),
        'p' => if c.hour >= 12 { "PM" } else { "AM" }.into(),
        'q' => ((c.month as usize - 1) / 3 + 1).to_string(),
        'Q' => date.to_string(),
        's' => (date / 1000.).floor().to_string(),
        'S' => pad(c.second as i64, p, 2),
        'u' => if f.weekday == 0 { 7 } else { f.weekday }.to_string(),
        'U' => pad(week_of_year(cal, date, 0), p, 2),
        'V' => {
            let th = iso_thursday(cal, date);
            let ys = floor_base(cal, th, "year", 0);
            let first = floor_base(cal, ys, "week", 4);
            let v = span(cal, first, th, WEEK) as i64
                + if cal.fields(ys).is_some_and(|f| f.weekday == 4) {
                    1
                } else {
                    0
                };
            pad(v, p, 2)
        }
        'w' => f.weekday.to_string(),
        'W' => pad(week_of_year(cal, date, 1), p, 2),
        'x' => time_format(cal, "%-m/%-d/%Y", date),
        'X' => time_format(cal, "%-I:%M:%S %p", date),
        'y' => pad(c.year as i64 % 100, p, 2),
        'Y' => pad(c.year as i64 % 10000, p, 4),
        'Z' => {
            let z = f.utc_offset_minutes;
            format!(
                "{}{}{}",
                if z < 0 { '-' } else { '+' },
                pad((z.abs() / 60) as i64, "0", 2),
                pad((z.abs() % 60) as i64, "0", 2)
            )
        }
        '%' => "%".into(),
        '\0' => String::new(),
        _ => token.to_string(),
    }
}
