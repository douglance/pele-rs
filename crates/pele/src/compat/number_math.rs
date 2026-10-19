//! ECMAScript numeric operations used by public render options.
//! Unlike Rust min/max, Math.min/max propagate NaN and specify zero ties.
pub(crate) trait NumberMath {
    fn js_min(self, other: Self) -> Self;
    fn js_max(self, other: Self) -> Self;
}
impl NumberMath for f64 {
    fn js_min(self, other: Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return f64::NAN;
        }
        if self == 0. && other == 0. {
            return if self.is_sign_negative() || other.is_sign_negative() {
                -0.
            } else {
                0.
            };
        }
        if self < other {
            self
        } else {
            other
        }
    }
    fn js_max(self, other: Self) -> Self {
        if self.is_nan() || other.is_nan() {
            return f64::NAN;
        }
        if self == 0. && other == 0. {
            return if self.is_sign_negative() && other.is_sign_negative() {
                -0.
            } else {
                0.
            };
        }
        if self > other {
            self
        } else {
            other
        }
    }
}
/// Math.round ties toward positive infinity and retains negative zero. Avoid
/// adding .5 to the input: it changes integers and fractions near f64 precision.
pub(crate) fn round(value: f64) -> f64 {
    if !value.is_finite() || value == 0. || value.fract() == 0. {
        return value;
    }
    let lower = value.floor();
    let rounded = if value - lower < 0.5 {
        lower
    } else {
        lower + 1.
    };
    if rounded == 0. && value.is_sign_negative() {
        -0.
    } else {
        rounded
    }
}
