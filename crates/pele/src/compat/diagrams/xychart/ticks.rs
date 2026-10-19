//! Pele xychart/ticks.ts bounded decimal tick construction, 458322a5 (MIT, kepano).
use crate::compat::number_math::{round, NumberMath};
// Integer decimal powers are exact conversions in the pinned JS engine. The
// WASM libc pow implementation can differ by one ulp, changing tick text and
// therefore margins. Parsing the decimal power preserves the same binary value
// on every target; tick exponents are always integers (or non-finite).
fn decimal_power(power: f64) -> f64 {
    if power.is_nan() {
        return f64::NAN;
    }
    if power > 308. {
        return f64::INFINITY;
    }
    if power < -324. {
        return 0.;
    }
    format!("1e{}", power as i32).parse().unwrap()
}
pub(super) fn tick_step(span: f64, count: f64) -> f64 {
    let raw = span / count.js_max(1.);
    let power = raw.log10().floor();
    let unit = raw / decimal_power(power);
    let factor = if unit >= 7.07 {
        10.
    } else if unit >= 3.16 {
        5.
    } else if unit >= 1.41 {
        2.
    } else {
        1.
    };
    if power < 0. {
        -decimal_power(-power) / factor
    } else {
        factor * decimal_power(power)
    }
}
pub(super) fn tick_index(step: f64, value: f64) -> f64 {
    if step > 0. {
        value / step
    } else {
        value * -step
    }
}
pub(super) fn tick_value(step: f64, index: f64) -> f64 {
    if step > 0. {
        index * step
    } else {
        index / -step
    }
}
pub(super) fn ticks_between(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let mut first = round(tick_index(step, lo));
    if tick_value(step, first) < lo {
        first += 1.
    }
    let mut last = round(tick_index(step, hi));
    if tick_value(step, last) > hi {
        last -= 1.
    }
    let mut out = vec![];
    let mut i = first;
    while i <= last && out.len() < 100 {
        out.push(tick_value(step, i));
        i += 1.;
    }
    out
}
