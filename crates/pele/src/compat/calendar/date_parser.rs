//! ECMAScript ISO/legacy Date.parse, translated from V8's dateparser.h,
//! dateparser-inl.h, dateparser.cc in Node.js v25.9.0 (the development oracle).
//! Copyright 2011 the V8 project authors. All rights reserved.
//! BSD-3-Clause; see third_party/v8/LICENSE. No JavaScript runtime is embedded.
//! https://github.com/nodejs/node/tree/v25.9.0/deps/v8/src/date
use super::CivilDateTime;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParsedDate {
    Instant(f64),
    Local(CivilDateTime),
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Invalid,
    Month,
    Zone,
    Separator,
    AmPm,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Invalid,
    Unknown,
    Space,
    Number(i32, usize),
    Symbol(char),
    End,
    Keyword(Key, i32, usize),
}
impl Token {
    fn fixed(self, n: usize) -> bool {
        matches!(self,Self::Number(_,k)if k==n)
    }
    fn n(self) -> i32 {
        if let Self::Number(n, _) = self {
            n
        } else {
            0
        }
    }
    fn sign(self) -> bool {
        matches!(self, Self::Symbol('+' | '-'))
    }
    fn sign_value(self) -> i32 {
        if self == Self::Symbol('-') {
            -1
        } else {
            1
        }
    }
    fn z(self) -> bool {
        self == Self::Keyword(Key::Zone, 0, 1)
    }
}
fn white(c: char) -> bool {
    crate::compat::source::space(c) && !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
fn keyword(prefix: &[char], len: usize) -> Token {
    let names = [
        ("jan", Key::Month, 1),
        ("feb", Key::Month, 2),
        ("mar", Key::Month, 3),
        ("apr", Key::Month, 4),
        ("may", Key::Month, 5),
        ("jun", Key::Month, 6),
        ("jul", Key::Month, 7),
        ("aug", Key::Month, 8),
        ("sep", Key::Month, 9),
        ("oct", Key::Month, 10),
        ("nov", Key::Month, 11),
        ("dec", Key::Month, 12),
        ("am", Key::AmPm, 0),
        ("pm", Key::AmPm, 12),
        ("ut", Key::Zone, 0),
        ("utc", Key::Zone, 0),
        ("z", Key::Zone, 0),
        ("gmt", Key::Zone, 0),
        ("cdt", Key::Zone, -5),
        ("cst", Key::Zone, -6),
        ("edt", Key::Zone, -4),
        ("est", Key::Zone, -5),
        ("mdt", Key::Zone, -6),
        ("mst", Key::Zone, -7),
        ("pdt", Key::Zone, -7),
        ("pst", Key::Zone, -8),
        ("t", Key::Separator, 0),
    ];
    for (name, kind, value) in names {
        let key: Vec<char> = name.chars().collect();
        if prefix == key.as_slice() && (len <= 3 || kind == Key::Month) {
            return Token::Keyword(kind, value, len);
        }
    }
    Token::Keyword(Key::Invalid, 0, len)
}
struct Scanner {
    tokens: Vec<Token>,
    i: usize,
}
impl Scanner {
    fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let mut p = 0;
        let mut tokens = vec![];
        while p < chars.len() && chars[p] != '\0' {
            let c = chars[p];
            if c.is_ascii_digit() {
                let start = p;
                while p < chars.len() && chars[p] == '0' {
                    p += 1
                }
                let mut n = 0;
                let mut significant = 0;
                while p < chars.len() && chars[p].is_ascii_digit() {
                    if significant < 9 {
                        n = n * 10 + chars[p] as i32 - '0' as i32
                    }
                    significant += 1;
                    p += 1
                }
                tokens.push(Token::Number(n, p - start));
            } else if matches!(c, ':' | '-' | '+' | '.' | ')') {
                tokens.push(Token::Symbol(c));
                p += 1
            } else if c >= 'A' && !white(c) {
                let mut prefix = vec![];
                let mut len = 0;
                while p < chars.len() && chars[p] >= 'A' && !white(chars[p]) {
                    if len < 3 {
                        prefix.push(chars[p].to_ascii_lowercase())
                    }
                    len += chars[p].len_utf16();
                    p += 1
                }
                tokens.push(keyword(&prefix, len))
            } else if crate::compat::source::space(c) {
                tokens.push(Token::Space);
                p += 1
            } else if c == '(' {
                let mut balance = 0;
                loop {
                    if p >= chars.len() || chars[p] == '\0' {
                        break;
                    }
                    if chars[p] == ')' {
                        balance -= 1
                    } else if chars[p] == '(' {
                        balance += 1
                    }
                    p += 1;
                    if balance <= 0 {
                        break;
                    }
                }
                tokens.push(Token::Unknown)
            } else {
                tokens.push(Token::Unknown);
                p += 1
            }
        }
        tokens.push(Token::End);
        Self { tokens, i: 0 }
    }
    fn peek(&self) -> Token {
        self.tokens.get(self.i).copied().unwrap_or(Token::End)
    }
    fn next(&mut self) -> Token {
        let t = self.peek();
        if t != Token::End {
            self.i += 1
        }
        t
    }
    fn skip(&mut self, c: char) -> bool {
        if self.peek() == Token::Symbol(c) {
            self.next();
            true
        } else {
            false
        }
    }
}
#[derive(Default)]
struct Day {
    values: Vec<i32>,
    named: Option<i32>,
    iso: bool,
}
impl Day {
    fn add(&mut self, n: i32) -> bool {
        if self.values.len() == 3 {
            return false;
        }
        self.values.push(n);
        true
    }
    fn finish(&self) -> Option<(i32, i32, i32)> {
        if self.values.is_empty() {
            return None;
        }
        let mut values = self.values.clone();
        values.resize(3, 1);
        let (mut year, month, day);
        if let Some(named) = self.named {
            month = named;
            if !(1..=31).contains(&values[0]) {
                year = values[0];
                day = values[1]
            } else {
                day = values[0];
                year = values[1]
            }
        } else if self.iso || !(1..=31).contains(&values[0]) {
            year = values[0];
            month = values[1];
            day = values[2]
        } else {
            month = values[0];
            day = values[1];
            year = values[2]
        }
        if !self.iso {
            if (0..=49).contains(&year) {
                year += 2000
            } else if (50..=99).contains(&year) {
                year += 1900
            }
        }
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            None
        } else {
            Some((year, month, day))
        }
    }
}
#[derive(Default)]
struct Time {
    values: Vec<i32>,
    hour_offset: Option<i32>,
}
impl Time {
    fn add(&mut self, n: i32) -> bool {
        if self.values.len() == 4 {
            return false;
        }
        self.values.push(n);
        true
    }
    fn final_add(&mut self, n: i32) -> bool {
        if !self.add(n) {
            return false;
        }
        self.values.resize(4, 0);
        true
    }
    fn expecting(&self, n: i32) -> bool {
        match self.values.len() {
            1 | 2 => (0..=59).contains(&n),
            3 => (0..=999).contains(&n),
            _ => false,
        }
    }
    fn finish(&self) -> Option<[i32; 4]> {
        let mut v = self.values.clone();
        v.resize(4, 0);
        if let Some(offset) = self.hour_offset {
            if !(0..=12).contains(&v[0]) {
                return None;
            }
            v[0] = v[0] % 12 + offset
        }
        if !((0..=23).contains(&v[0])
            && (0..=59).contains(&v[1])
            && (0..=59).contains(&v[2])
            && (0..=999).contains(&v[3]))
            && !(v[0] == 24 && v[1..].iter().all(|n| *n == 0))
        {
            None
        } else {
            Some([v[0], v[1], v[2], v[3]])
        }
    }
}
#[derive(Default)]
struct Zone {
    sign: Option<i32>,
    hour: Option<i32>,
    minute: Option<i32>,
}
impl Zone {
    fn set(&mut self, h: i32) {
        self.sign = Some(if h < 0 { -1 } else { 1 });
        self.hour = Some(h.abs());
        self.minute = Some(0)
    }
    fn utc(&self) -> bool {
        self.hour == Some(0) && self.minute == Some(0)
    }
    fn expecting(&self, n: i32) -> bool {
        self.hour.is_some() && self.minute.is_none() && (0..=59).contains(&n)
    }
    fn finish(&self) -> Option<Option<i64>> {
        let Some(sign) = self.sign else {
            return Some(None);
        };
        let seconds = (self.hour.unwrap_or(0) as u32)
            .wrapping_mul(3600)
            .wrapping_add((self.minute.unwrap_or(0) as u32).wrapping_mul(60));
        if seconds > i32::MAX as u32 {
            None
        } else {
            Some(Some(seconds as i64 * sign as i64))
        }
    }
}
fn milliseconds(t: Token) -> i32 {
    let Token::Number(mut n, mut len) = t else {
        return -1;
    };
    if len < 3 {
        n *= if len == 1 { 100 } else { 10 }
    } else if len > 3 {
        len = len.min(9);
        n /= 10_i32.pow((len - 3) as u32)
    }
    n
}
fn iso(s: &mut Scanner, day: &mut Day, time: &mut Time, zone: &mut Zone) -> Token {
    if s.peek().sign() {
        let sign = s.next();
        if !s.peek().fixed(6) {
            return sign;
        }
        let year = s.next().n();
        if sign.sign_value() < 0 && year == 0 {
            return sign;
        }
        day.add(sign.sign_value() * year);
    } else if s.peek().fixed(4) {
        day.add(s.next().n());
    } else {
        return s.next();
    }
    if s.skip('-') {
        if !s.peek().fixed(2) || !(1..=12).contains(&s.peek().n()) {
            return s.next();
        }
        day.add(s.next().n());
        if s.skip('-') {
            if !s.peek().fixed(2) || !(1..=31).contains(&s.peek().n()) {
                return s.next();
            }
            day.add(s.next().n());
        }
    }
    if !matches!(s.peek(), Token::Keyword(Key::Separator, _, _)) {
        if s.peek() != Token::End {
            return s.next();
        }
    } else {
        s.next();
        if !s.peek().fixed(2) || !(0..=24).contains(&s.peek().n()) {
            return Token::Invalid;
        }
        let h24 = s.peek().n() == 24;
        time.add(s.next().n());
        if !s.skip(':') {
            return Token::Invalid;
        }
        if !s.peek().fixed(2) || !(0..=59).contains(&s.peek().n()) || h24 && s.peek().n() > 0 {
            return Token::Invalid;
        }
        time.add(s.next().n());
        if s.skip(':') {
            if !s.peek().fixed(2) || !(0..=59).contains(&s.peek().n()) || h24 && s.peek().n() > 0 {
                return Token::Invalid;
            }
            time.add(s.next().n());
            if s.skip('.') {
                if !matches!(s.peek(), Token::Number(..)) || h24 && s.peek().n() > 0 {
                    return Token::Invalid;
                }
                time.add(milliseconds(s.next()));
            }
        }
        if s.peek().z() {
            s.next();
            zone.set(0)
        } else if s.peek().sign() {
            zone.sign = Some(s.next().sign_value());
            if s.peek().fixed(4) {
                let hm = s.next().n();
                if !(0..=23).contains(&(hm / 100)) || !(0..=59).contains(&(hm % 100)) {
                    return Token::Invalid;
                }
                zone.hour = Some(hm / 100);
                zone.minute = Some(hm % 100)
            } else {
                if !s.peek().fixed(2) || !(0..=23).contains(&s.peek().n()) {
                    return Token::Invalid;
                }
                zone.hour = Some(s.next().n());
                if !s.skip(':') {
                    return Token::Invalid;
                }
                if !s.peek().fixed(2) || !(0..=59).contains(&s.peek().n()) {
                    return Token::Invalid;
                }
                zone.minute = Some(s.next().n())
            }
        }
        if s.peek() != Token::End {
            return Token::Invalid;
        }
    }
    if zone.hour.is_none() && time.values.is_empty() {
        zone.set(0)
    }
    day.iso = true;
    Token::End
}
/// V8-compatible parsing; local civil values deliberately remain unresolved until
/// the host supplies its calendar, including DST fold/gap disambiguation.
pub fn parse(input: &str) -> Option<ParsedDate> {
    let mut scanner = Scanner::new(input);
    let mut day = Day::default();
    let mut time = Time::default();
    let mut zone = Zone::default();
    let mut token = iso(&mut scanner, &mut day, &mut time, &mut zone);
    if token == Token::Invalid {
        return None;
    }
    let mut number = !day.values.is_empty();
    while token != Token::End {
        match token {
            Token::Number(n, _) => {
                number = true;
                if scanner.skip(':') {
                    if scanner.skip(':') {
                        if !time.values.is_empty() {
                            return None;
                        }
                        time.add(n);
                        time.add(0);
                    } else {
                        if !time.add(n) {
                            return None;
                        }
                        if scanner.peek() == Token::Symbol('.') {
                            scanner.next();
                        }
                    }
                } else if scanner.skip('.') && time.expecting(n) {
                    time.add(n);
                    if !matches!(scanner.peek(), Token::Number(..)) {
                        return None;
                    }
                    let ms = milliseconds(scanner.next());
                    if ms < 0 {
                        return None;
                    }
                    time.final_add(ms);
                } else if zone.expecting(n) {
                    zone.minute = Some(n)
                } else if time.expecting(n) {
                    time.final_add(n);
                    let p = scanner.peek();
                    if p != Token::End && p != Token::Space && !p.z() && !p.sign() {
                        return None;
                    }
                } else {
                    if !day.add(n) {
                        return None;
                    }
                    scanner.skip('-');
                }
            }
            Token::Keyword(kind, value, _) => {
                if kind == Key::AmPm && !time.values.is_empty() {
                    time.hour_offset = Some(value)
                } else if kind == Key::Month {
                    day.named = Some(value);
                    scanner.skip('-');
                } else if kind == Key::Zone && number {
                    zone.set(value)
                } else if number || matches!(scanner.peek(), Token::Number(..)) {
                    return None;
                }
            }
            t if t.sign() && (zone.utc() || !time.values.is_empty()) => {
                zone.sign = Some(t.sign_value());
                let (n, len) = if let Token::Number(n, len) = scanner.peek() {
                    scanner.next();
                    (n, len)
                } else {
                    (0, 0)
                };
                number = true;
                if scanner.peek() == Token::Symbol(':') {
                    zone.hour = Some(n);
                    zone.minute = None
                } else if len == 1 || len == 2 {
                    zone.hour = Some(n);
                    zone.minute = Some(0)
                } else if len == 3 || len == 4 {
                    zone.hour = Some(n / 100);
                    zone.minute = Some(n % 100)
                } else {
                    return None;
                }
            }
            t if (t.sign() || t == Token::Symbol(')')) && number => return None,
            _ => {}
        }
        token = scanner.next()
    }
    let (year, month, day) = day.finish()?;
    let [hour, minute, second, ms] = time.finish()?;
    let zone = zone.finish()?;
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from(month) + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    let base = days as f64 * 86400000.;
    let raw = base
        + (day - 1) as f64 * 86400000.
        + hour as f64 * 3600000.
        + minute as f64 * 60000.
        + second as f64 * 1000.
        + ms as f64;
    if let Some(offset) = zone {
        let instant = raw - offset as f64 * 1000.;
        if instant.is_finite() && instant.abs() <= 8.64e15 {
            Some(ParsedDate::Instant(instant.trunc()))
        } else {
            None
        }
    } else {
        normalize_local(raw).map(ParsedDate::Local)
    }
}

// Local civil time may lie just outside TimeClip while its UTC instant is valid
// in an eastern/western timezone. Resolve the host zone before clipping.
fn normalize_local(ms: f64) -> Option<CivilDateTime> {
    if !ms.is_finite() || ms.abs() > 8.64e15 + 86400000. {
        return None;
    }
    let ms = ms.trunc() as i64;
    let days = ms.div_euclid(86400000);
    let rem = ms.rem_euclid(86400000);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Some(CivilDateTime {
        year: year as i32,
        month: month as u8,
        day: day as u8,
        hour: (rem / 3600000) as u8,
        minute: (rem / 60000 % 60) as u8,
        second: (rem / 1000 % 60) as u8,
        millisecond: (rem % 1000) as u16,
    })
}
