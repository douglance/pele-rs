//! Pele treemap/format.ts. IEEE-754 decimal rounding follows ECMAScript's
//! toFixed/toPrecision/toExponential, including half-up ties (not Rust's ties-even).
use crate::compat::svg::js_num;
use regex::Regex;
use std::sync::OnceLock;
pub(super) struct Formatter {
    plus: bool,
    dollar: bool,
    comma: bool,
    precision: Option<usize>,
    trim: bool,
    kind: String,
}
impl Formatter {
    pub fn new(spec: &str) -> Self {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re=RE.get_or_init(||Regex::new(r"^(?:[^\n\r\u{2028}\u{2029}\u{10000}-\u{10ffff}]?[<>=^])?([-+( ])?(\$)?0?[0-9]*(,)?(?:\.([0-9]+))?(~)?([a-zA-Z%])?$").unwrap());
        let m = re
            .captures(if spec == "$0,0" { "$," } else { spec })
            .unwrap_or_else(|| re.captures(",").unwrap());
        Self {
            plus: m.get(1).is_some_and(|m| m.as_str() == "+"),
            dollar: m.get(2).is_some(),
            comma: m.get(3).is_some(),
            precision: m
                .get(4)
                .map(|m| m.as_str().parse::<usize>().unwrap_or(20).min(20)),
            trim: m.get(5).is_some(),
            kind: m.get(6).map_or("", |m| m.as_str()).into(),
        }
    }
    pub fn format(&self, value: f64) -> String {
        let abs = value.abs();
        let (mut body, suffix) = match self.kind.as_str() {
            "f" => (fixed(abs, self.precision.unwrap_or(6)), ""),
            "%" => (fixed(abs * 100., self.precision.unwrap_or(6)), "%"),
            "d" => (
                fixed(
                    if abs >= 4503599627370496. {
                        abs
                    } else {
                        (abs + 0.5).floor()
                    },
                    0,
                ),
                "",
            ),
            "e" => (exponential(abs, self.precision.unwrap_or(6)), ""),
            _ => (precision(abs, self.precision.unwrap_or(12).max(1)), ""),
        };
        if self.trim && body.contains('.') && !body.contains('e') {
            while body.ends_with('0') {
                body.pop();
            }
            if body.ends_with('.') {
                body.pop();
            }
        }
        if self.comma && !body.contains('e') {
            let dot = body.find('.').unwrap_or(body.len());
            let digits = &body[..dot];
            let mut grouped = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i) % 3 == 0 {
                    grouped.push(',')
                }
                grouped.push(c)
            }
            grouped.push_str(&body[dot..]);
            body = grouped;
        }
        format!(
            "{}{}{body}{suffix}",
            if value < 0. {
                "-"
            } else if self.plus {
                "+"
            } else {
                ""
            },
            if self.dollar { "$" } else { "" }
        )
    }
}
// Exact finite positive binary number, as coefficient digits and decimal exponent.
// The tiny integer needs at most 1,074 multiplies and 120 base-1e9 words.
fn exact(v: f64) -> (String, i32) {
    if v == 0. {
        return ("0".into(), 0);
    }
    let bits = v.to_bits();
    let e = ((bits >> 52) & 2047) as i32;
    let mantissa = if e == 0 {
        bits & ((1_u64 << 52) - 1)
    } else {
        (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52)
    };
    let exponent = if e == 0 { -1074 } else { e - 1023 - 52 };
    let mut words = vec![
        (mantissa % 1_000_000_000) as u32,
        (mantissa / 1_000_000_000) as u32,
    ];
    while words.last() == Some(&0) {
        words.pop();
    }
    let factor = if exponent < 0 { 5 } else { 2 };
    for _ in 0..exponent.unsigned_abs() {
        let mut carry = 0_u64;
        for word in &mut words {
            let n = *word as u64 * factor + carry;
            *word = (n % 1_000_000_000) as u32;
            carry = n / 1_000_000_000;
        }
        if carry > 0 {
            words.push(carry as u32)
        }
    }
    let mut digits = words.pop().unwrap().to_string();
    for word in words.iter().rev() {
        digits.push_str(&format!("{word:09}"))
    }
    let point = digits.len() as i32 + exponent.min(0);
    (digits, point - 1)
}
fn rounded(digits: &str, places: usize) -> String {
    let mut out: Vec<u8> = digits.bytes().take(places).collect();
    out.resize(places, b'0');
    if digits.as_bytes().get(places).is_some_and(|b| *b >= b'5') {
        let mut i = out.len();
        while i > 0 && out[i - 1] == b'9' {
            out[i - 1] = b'0';
            i -= 1;
        }
        if i > 0 {
            out[i - 1] += 1
        } else {
            out.insert(0, b'1')
        }
    }
    String::from_utf8(out).unwrap()
}
fn fixed(v: f64, n: usize) -> String {
    if !v.is_finite() || v >= 1e21 {
        return js_num(v);
    }
    let (digits, exp) = exact(v);
    let point = exp + 1;
    let keep = point + n as i32;
    let mut out = if keep < 0 {
        "0".into()
    } else {
        rounded(&digits, keep as usize)
    };
    if out.is_empty() {
        out.push('0')
    }
    if n > 0 {
        if out.len() <= n {
            out = format!("{}{}", "0".repeat(n + 1 - out.len()), out)
        }
        out.insert(out.len() - n, '.');
    }
    out
}
fn significant(v: f64, count: usize) -> (String, i32) {
    let (digits, mut exp) = exact(v);
    let mut out = rounded(&digits, count);
    if out.len() > count {
        out.pop();
        exp += 1;
    }
    (out, exp)
}
fn exponential(v: f64, n: usize) -> String {
    if !v.is_finite() {
        return js_num(v);
    }
    let (digits, exp) = significant(v, n + 1);
    let mantissa = if n == 0 {
        digits
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    format!("{mantissa}e{}{exp}", if exp >= 0 { "+" } else { "" })
}
fn precision(v: f64, n: usize) -> String {
    if !v.is_finite() {
        return js_num(v);
    }
    let (digits, exp) = significant(v, n);
    let s = format!("{digits}e{}", exp - (digits.len() as i32 - 1));
    js_num(s.parse().unwrap_or(v))
}
