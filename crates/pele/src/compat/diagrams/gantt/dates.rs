//! Pele gantt/dates.ts (dayjs compatibility), 458322a5 (MIT, kepano).
//! Calendar normalization is Gregorian; timezone/DST conversion belongs to host resources.
use crate::compat::{CalendarFields, CivilDateTime, PeleError, RenderResources};
use regex::Regex;
use std::sync::OnceLock;
pub const DAY: f64 = 86400000.;
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
pub fn clip(ms: f64) -> f64 {
    if ms.is_finite() && ms.abs() <= 8.64e15 {
        ms.trunc()
    } else {
        f64::NAN
    }
}
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
pub fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (
        (y + if month <= 2 { 1 } else { 0 }) as i32,
        month as u8,
        day as u8,
    )
}
pub fn utc_ms(c: CivilDateTime) -> f64 {
    days_from_civil(c.year as i64, c.month as i64, c.day as i64) as f64 * DAY
        + c.hour as f64 * 3600000.
        + c.minute as f64 * 60000.
        + c.second as f64 * 1000.
        + c.millisecond as f64
}
pub fn utc_fields(ms: f64) -> Option<CalendarFields> {
    let ms = clip(ms);
    if !ms.is_finite() {
        return None;
    }
    let days = (ms / DAY).floor() as i64;
    let rem = ms.rem_euclid(DAY) as i64;
    let (year, month, day) = civil_from_days(days);
    Some(CalendarFields {
        civil: CivilDateTime {
            year,
            month,
            day,
            hour: (rem / 3600000) as u8,
            minute: (rem / 60000 % 60) as u8,
            second: (rem / 1000 % 60) as u8,
            millisecond: (rem % 1000) as u16,
        },
        weekday: (days + 4).rem_euclid(7) as u8,
        utc_offset_minutes: 0,
    })
}
pub fn normalized(
    year: f64,
    month0: f64,
    day: f64,
    hour: f64,
    minute: f64,
    second: f64,
    ms: f64,
) -> Option<CivilDateTime> {
    if [year, month0, day, hour, minute, second, ms]
        .iter()
        .any(|n| !n.is_finite() || n.abs() > 1e12)
    {
        return None;
    }
    let y = year.trunc() as i64;
    let m = month0.trunc() as i64;
    let y = y + m.div_euclid(12);
    let month = m.rem_euclid(12) + 1;
    let value = days_from_civil(y, month, 1) as f64 * DAY
        + (day.trunc() - 1.) * DAY
        + hour.trunc() * 3600000.
        + minute.trunc() * 60000.
        + second.trunc() * 1000.
        + ms.trunc();
    utc_fields(value).map(|f| f.civil)
}
pub struct Calendar<'a> {
    pub resources: &'a mut dyn RenderResources,
    pub error: Option<PeleError>,
}
impl<'a> Calendar<'a> {
    pub fn new(resources: &'a mut dyn RenderResources) -> Self {
        Self {
            resources,
            error: None,
        }
    }
    pub fn fields(&mut self, ms: f64) -> Option<CalendarFields> {
        if !clip(ms).is_finite() {
            return None;
        }
        match self.resources.calendar_fields(ms) {
            Ok(f) => f,
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(e)
                }
                None
            }
        }
    }
    pub fn local(&mut self, c: CivilDateTime) -> f64 {
        match self.resources.calendar_instant(c) {
            Ok(ms) => ms.map(clip).unwrap_or(f64::NAN),
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(e)
                }
                f64::NAN
            }
        }
    }
    pub fn make(
        &mut self,
        y: f64,
        m: f64,
        d: f64,
        h: f64,
        minute: f64,
        s: f64,
        ms: f64,
        constructor: bool,
    ) -> f64 {
        let y = if constructor && (0. ..100.).contains(&y) {
            y.trunc() + 1900.
        } else {
            y
        };
        normalized(y, m, d, h, minute, s, ms).map_or(f64::NAN, |c| self.local(c))
    }
    pub fn today(&mut self, now: f64) -> f64 {
        let Some(f) = self.fields(now) else {
            return f64::NAN;
        };
        self.make(
            f.civil.year as f64,
            f.civil.month as f64 - 1.,
            f.civil.day as f64,
            0.,
            0.,
            0.,
            0.,
            false,
        )
    }
    pub fn add_days(&mut self, date: f64, amount: f64) -> f64 {
        let Some(f) = self.fields(date) else {
            return f64::NAN;
        };
        let c = f.civil;
        self.make(
            c.year as f64,
            c.month as f64 - 1.,
            c.day as f64 + amount,
            c.hour as f64,
            c.minute as f64,
            c.second as f64,
            c.millisecond as f64,
            false,
        )
    }
    pub fn add(&mut self, date: f64, amount: f64, unit: &str) -> f64 {
        match unit {
            "d" => self.add_days(date, (amount + 0.5).floor()),
            "w" => self.add_days(date, (7. * amount + 0.5).floor()),
            "M" | "y" => {
                let Some(f) = self.fields(date) else {
                    return f64::NAN;
                };
                let c = f.civil;
                let y = c.year as f64 + if unit == "y" { amount } else { 0. };
                let m = c.month as f64 - 1. + if unit == "M" { amount } else { 0. };
                let Some(mut first) = normalized(
                    y,
                    m,
                    1.,
                    c.hour as f64,
                    c.minute as f64,
                    c.second as f64,
                    c.millisecond as f64,
                ) else {
                    return f64::NAN;
                };
                let next =
                    normalized(first.year as f64, first.month as f64, 0., 0., 0., 0., 0.).unwrap();
                first.day = c.day.min(next.day);
                self.local(first)
            }
            _ => clip(
                date + amount
                    * match unit {
                        "h" => 3600000.,
                        "m" => 60000.,
                        "s" => 1000.,
                        _ => 1.,
                    },
            ),
        }
    }
    pub fn iso(&mut self, date: f64) -> String {
        let Some(f) = self.fields(date) else {
            return "Invalid Date".into();
        };
        format!(
            "{}-{:02}-{:02}",
            pad(f.civil.year as i64, 4),
            f.civil.month,
            f.civil.day
        )
    }
    pub fn format(&mut self, date: f64, template: &str) -> Option<String> {
        let Some(f) = self.fields(date) else {
            return Some("Invalid Date".into());
        };
        let c = f.civil;
        let template = if template.is_empty() {
            "YYYY-MM-DDTHH:mm:ssZ"
        } else {
            template
        };
        if template == "YYYY-MM-DD" {
            return Some(self.iso(date));
        }
        let advanced = adv_re()
            .replace_all(template, |caps: &regex::Captures<'_>| {
                let t = &caps[0];
                match t {
                    "Q" => (c.month as usize).div_ceil(3).to_string(),
                    "Do" => format!("[{}]", ordinal(c.day as i64)),
                    "GGGG" => {
                        let t = self.add_days(
                            date,
                            4. - if f.weekday == 0 { 7. } else { f.weekday as f64 },
                        );
                        self.fields(t)
                            .map_or(String::new(), |f| f.civil.year.to_string())
                    }
                    "W" => self.iso_week(date).to_string(),
                    "WW" => pad(self.iso_week(date), 2),
                    "k" => if c.hour == 0 { 24 } else { c.hour }.to_string(),
                    "kk" => pad(if c.hour == 0 { 24 } else { c.hour } as i64, 2),
                    "X" => (date / 1000.).floor().to_string(),
                    "x" => date.to_string(),
                    _ => t.into(),
                }
            })
            .into_owned();
        for m in adv_re().find_iter(template) {
            if matches!(m.as_str(), "gggg" | "wo" | "w" | "ww" | "z" | "zzz") {
                return None;
            }
        }
        let zone = zone(f.utc_offset_minutes);
        Some(
            fmt_re()
                .replace_all(&advanced, |caps: &regex::Captures<'_>| {
                    if let Some(l) = caps.get(1) {
                        return l.as_str().into();
                    }
                    match &caps[0] {
                        "YY" => {
                            let s = c.year.to_string();
                            s[s.len().saturating_sub(2)..].into()
                        }
                        "YYYY" => pad(c.year as i64, 4),
                        "M" => c.month.to_string(),
                        "MM" => pad(c.month as i64, 2),
                        "MMM" => MONTHS[c.month as usize - 1][..3].into(),
                        "MMMM" => MONTHS[c.month as usize - 1].into(),
                        "D" => c.day.to_string(),
                        "DD" => pad(c.day as i64, 2),
                        "d" => f.weekday.to_string(),
                        "dd" => WEEKDAYS[f.weekday as usize][..2].into(),
                        "ddd" => WEEKDAYS[f.weekday as usize][..3].into(),
                        "dddd" => WEEKDAYS[f.weekday as usize].into(),
                        "H" => c.hour.to_string(),
                        "HH" => pad(c.hour as i64, 2),
                        "h" => if c.hour % 12 == 0 { 12 } else { c.hour % 12 }.to_string(),
                        "hh" => pad(if c.hour % 12 == 0 { 12 } else { c.hour % 12 } as i64, 2),
                        "a" => if c.hour < 12 { "am" } else { "pm" }.into(),
                        "A" => if c.hour < 12 { "AM" } else { "PM" }.into(),
                        "m" => c.minute.to_string(),
                        "mm" => pad(c.minute as i64, 2),
                        "s" => c.second.to_string(),
                        "ss" => pad(c.second as i64, 2),
                        "SSS" => pad(c.millisecond as i64, 3),
                        "Z" => zone.clone(),
                        _ => zone.replace(':', ""),
                    }
                })
                .into_owned(),
        )
    }
    pub fn iso_week(&mut self, date: f64) -> i64 {
        let Some(f) = self.fields(date) else { return 0 };
        let th = self.add_days(
            date,
            4. - if f.weekday == 0 { 7. } else { f.weekday as f64 },
        );
        let Some(t) = self.fields(th) else { return 0 };
        let jan = self.make(t.civil.year as f64, 0., 1., 0., 0., 0., 0., true);
        let Some(j) = self.fields(jan) else { return 0 };
        let weekday = if j.weekday == 0 { 7. } else { j.weekday as f64 };
        let first = self.add_days(jan, 4. - weekday + if weekday > 4. { 7. } else { 0. });
        let offset = self.fields(first).map_or(0, |f| f.utc_offset_minutes) - t.utc_offset_minutes;
        let weeks = (th - first - offset as f64 * 60000.) / (7. * DAY);
        weeks.trunc() as i64 + 1
    }
    pub fn parse_strict(&mut self, input: &str, template: &str, now: f64) -> Option<f64> {
        let date = self.loose(input, template, now)?;
        if date.is_finite() && self.format(date, template).as_deref() == Some(input) {
            Some(date)
        } else {
            None
        }
    }
    fn loose(&mut self, input: &str, template: &str, now: f64) -> Option<f64> {
        if template == "x" || template == "X" {
            return Some(clip(
                js_number(input) * if template == "X" { 1000. } else { 1. },
            ));
        }
        if local_re().replace_all(template, "").contains(['L', 'l']) {
            return None;
        }
        let tokens: Vec<&str> = tokens_re()
            .find_iter(template)
            .map(|m| m.as_str())
            .collect();
        if tokens.is_empty() {
            return None;
        }
        let mut f = Fields::default();
        let mut rest = input.to_string();
        let mut start = 0usize;
        for token in tokens {
            let spec = match token {
                "A" | "a" | "MMM" | "MMMM" | "Do" => Some(0),
                "S" | "Q" => Some(1),
                "SS" | "DD" | "MM" | "YY" | "ww" => Some(2),
                "SSS" => Some(3),
                "YYYY" => Some(4),
                "Y" => Some(5),
                "Z" | "ZZ" => Some(6),
                "s" | "ss" | "m" | "mm" | "H" | "h" | "HH" | "hh" | "D" | "w" | "M" => Some(7),
                _ => None,
            };
            let Some(spec) = spec else {
                start += token
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .encode_utf16()
                    .count();
                continue;
            };
            let offset = utf16_byte(&rest, start);
            let m = readers()[spec].find(&rest[offset..])?;
            let value = m.as_str().to_string();
            let n = value.parse::<f64>().unwrap_or(f64::NAN);
            match token {
                "A" | "a" => f.afternoon = Some(value == if token == "A" { "PM" } else { "pm" }),
                "Q" => f.month = Some((n - 1.) * 3. + 1.),
                "S" => f.ms = n * 100.,
                "SS" => f.ms = n * 10.,
                "SSS" => f.ms = n,
                "s" | "ss" => f.second = n,
                "m" | "mm" => f.minute = n,
                "H" | "h" | "HH" | "hh" => f.hour = Some(n),
                "D" | "DD" => f.day = Some(n),
                "Do" => {
                    f.day = Some(readers()[8].find(&value)?.as_str().parse().ok()?);
                    f.day_given = true;
                    for i in 1..=31 {
                        if ordinal(i) == value {
                            f.day = Some(i as f64)
                        }
                    }
                }
                "w" | "ww" => f.week = Some(n),
                "M" | "MM" => f.month = Some(n),
                "MMM" | "MMMM" => {
                    let at = MONTHS.iter().position(|s| {
                        if token == "MMM" {
                            &s[..3] == value
                        } else {
                            **s == value
                        }
                    })?;
                    f.month = Some((at + 1) as f64)
                }
                "Y" | "YYYY" => f.year = Some(n),
                "YY" => f.year = Some(n + if n > 68. { 1900. } else { 2000. }),
                "Z" | "ZZ" => {
                    if value == "Z" {
                        f.offset = Some(0.)
                    } else {
                        let digits = value[1..].replace(':', "");
                        let hour = digits[..2].parse::<f64>().ok()?;
                        let minute = digits
                            .get(2..4)
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.);
                        f.offset = Some(
                            (hour * 60. + minute) * if value.starts_with('+') { -1. } else { 1. },
                        )
                    }
                }
                _ => {}
            }
            let at = rest.find(&value)?;
            rest.replace_range(at..at + value.len(), "");
        }
        let current = self.fields(now)?.civil;
        let mut hour = f.hour.unwrap_or(0.);
        if let Some(pm) = f.afternoon {
            if f.hour.is_some() {
                if pm {
                    if hour < 12. {
                        hour += 12.
                    }
                } else if hour == 12. {
                    hour = 0.
                }
            }
        }
        let day = if f.day_given || f.day.is_some_and(|n| n != 0.) {
            f.day.unwrap()
        } else if !f.year.is_some_and(|n| n != 0.) && !f.month.is_some_and(|n| n != 0.) {
            current.day as f64
        } else {
            1.
        };
        let year = f.year.filter(|n| *n != 0.).unwrap_or(current.year as f64);
        let month = if f.year.is_some_and(|n| n != 0.) && !f.month.is_some_and(|n| n != 0.) {
            0.
        } else {
            f.month
                .filter(|n| *n > 0.)
                .map_or(current.month as f64 - 1., |n| n - 1.)
        };
        if let Some(offset) = f.offset {
            let year = if (0. ..100.).contains(&year) {
                year + 1900.
            } else {
                year
            };
            let c = normalized(
                year,
                month,
                day,
                hour,
                f.minute,
                f.second,
                f.ms + offset * 60000.,
            )?;
            return Some(clip(utc_ms(c)));
        }
        if f.week.is_some_and(|n| n != 0.) {
            return None;
        }
        Some(self.make(year, month, day, hour, f.minute, f.second, f.ms, true))
    }
}

#[derive(Default)]
struct Fields {
    year: Option<f64>,
    month: Option<f64>,
    day: Option<f64>,
    day_given: bool,
    hour: Option<f64>,
    minute: f64,
    second: f64,
    ms: f64,
    week: Option<f64>,
    offset: Option<f64>,
    afternoon: Option<bool>,
}
fn utf16_byte(s: &str, n: usize) -> usize {
    let mut count = 0;
    for (i, c) in s.char_indices() {
        if count >= n {
            return i;
        }
        count += c.len_utf16()
    }
    s.len()
}
fn js_number(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        0.
    } else {
        s.parse().unwrap_or(f64::NAN)
    }
}
pub fn pad(n: i64, width: usize) -> String {
    let s = n.to_string();
    if s.len() >= width {
        s
    } else {
        format!("{}{}", "0".repeat(width - s.len()), s)
    }
}
fn ordinal(n: i64) -> String {
    let v = n % 100;
    let tail = match v {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{n}{tail}")
}
fn zone(offset: i32) -> String {
    let offset = ((offset as f64 / 15. + 0.5).floor() * 15.) as i32;
    format!(
        "{}{:02}:{:02}",
        if offset >= 0 { '+' } else { '-' },
        offset.abs() / 60,
        offset.abs() % 60
    )
}
fn adv_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        compiled_regex(r"\[([^\]]+)]|Q|wo|ww|w|WW|W|zzz|z|gggg|GGGG|Do|X|x|k{1,2}|S").unwrap()
    })
}
fn fmt_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        compiled_regex(
            r"\[([^\]]+)]|YYYY|YY|M{1,4}|D{1,2}|d{1,4}|H{1,2}|h{1,2}|a|A|m{1,2}|s{1,2}|Z{1,2}|SSS",
        )
        .unwrap()
    })
}
fn tokens_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(||compiled_regex(r"(\[[^\[]*\])|([-_:/.,()\s]+)|(A|a|Q|YYYY|YY?|ww?|MM?M?M?|Do|DD?|hh?|HH?|mm?|ss?|S{1,3}|z|ZZ?)").unwrap())
}
fn local_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| compiled_regex(r"\[[^\]]+]").unwrap())
}
fn readers() -> &'static Vec<Regex> {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    R.get_or_init(|| {
        [
            r"[0-9]*[^-_:/,()\s[0-9]]+",
            r"[0-9]",
            r"[0-9][0-9]",
            r"[0-9]{3}",
            r"[0-9]{4}",
            r"[+-]?[0-9]+",
            r"[+-][0-9][0-9]:?([0-9][0-9])?|Z",
            r"[0-9][0-9]?",
            r"[0-9]+",
        ]
        .iter()
        .map(|s| compiled_regex(s).unwrap())
        .collect()
    })
}
/// V8 Date.parse with a UTC calendar. Hosts resolve Local values through their calendar.
pub fn parse_date_utc(input: &str) -> Option<f64> {
    match crate::compat::calendar::date_parser::parse(input)? {
        crate::compat::calendar::date_parser::ParsedDate::Instant(ms) => Some(ms),
        crate::compat::calendar::date_parser::ParsedDate::Local(civil) => {
            crate::compat::calendar::utc_instant(civil)
        }
    }
}

fn compiled_regex(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&pattern.replace(r"\s", crate::compat::text::SPACE))
}
