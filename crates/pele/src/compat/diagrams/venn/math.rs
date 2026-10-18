//! V8 14.1 two-argument Math.hypot, from Node v25.9.0
//! deps/v8/src/builtins/math.tq FastMathHypot (BSD-3-Clause; third_party/v8/LICENSE).
//! Preserve operation order: native hypot rounds differently and the iterative solver amplifies it.
pub(super) fn hypot(a: f64, b: f64) -> f64 {
    let a = a.abs();
    let b = b.abs();
    if a == f64::INFINITY || b == f64::INFINITY {
        return f64::INFINITY;
    }
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    let max = a.max(b);
    if max == 0. {
        return 0.;
    }
    ((a / max) * (a / max) + (b / max) * (b / max)).sqrt() * max
}

// The following is adapted from V8's fdlibm (Node v25.9.0 deps/v8/src/base/ieee754.cc).
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================
// Modified significantly by Google Inc. Copyright 2016 the V8 project authors.
// BSD-3-Clause; see third_party/v8/LICENSE.
// Source SHA256 1bf999809d4c4c1d31ad5ff3578d32614796f159e08b9e7782560d017721a0c2.
// Compiler contractions in the arm64 oracle are explicit mul_add calls here,
// making their rounding independent of the Rust host architecture.
// Only the medium argument-reduction branch is needed: ring/search angles are
// below 2*pi; coincident-center fallback angles a+b are below 3464 because
// relaxation does zero rounds once n*n > 3,000,000. No large-angle fallback.
fn high(x: f64) -> u32 {
    (x.to_bits() >> 32) as u32
}
fn low(x: f64) -> u32 {
    x.to_bits() as u32
}
fn truncate_low(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000)
}
fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = high(x) & 0x7fff_ffff;
    if ix < 0x3e400000 {
        return 1.;
    }
    let z = x * x;
    let r = z * z.mul_add(
        z.mul_add(
            z.mul_add(
                z.mul_add(
                    z.mul_add(-1.13596475577881948265e-11, 2.08757232129817482790e-9),
                    -2.75573143513906633035e-7,
                ),
                2.48015872894767294178e-5,
            ),
            -1.38888888888741095749e-3,
        ),
        4.16666666666666019037e-2,
    );
    if ix < 0x3fd33333 {
        1. - z.mul_add(0.5, -z.mul_add(r, -(x * y)))
    } else {
        let qx = if ix > 0x3fe90000 {
            0.28125
        } else {
            f64::from_bits(((ix - 0x00200000) as u64) << 32)
        };
        let iz = z.mul_add(0.5, -qx);
        let a = 1. - qx;
        a - (iz - z.mul_add(r, -(x * y)))
    }
}
fn kernel_sin(x: f64, y: f64, tail: bool) -> f64 {
    if high(x) & 0x7fff_ffff < 0x3e400000 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = z.mul_add(
        z.mul_add(
            z.mul_add(
                z.mul_add(1.58969099521155010221e-10, -2.50507602534068634195e-8),
                2.75573137070700676789e-6,
            ),
            -1.98412698298579493134e-4,
        ),
        8.33333333332248946124e-3,
    );
    let s1 = -1.66666666666666324348e-1;
    if !tail {
        v.mul_add(z.mul_add(r, s1), x)
    } else {
        x - (-v).mul_add(s1, z.mul_add(y.mul_add(0.5, -(v * r)), -y))
    }
}
fn reduce(x: f64) -> (i32, f64, f64) {
    const P1: f64 = 1.57079632673412561417;
    const P1T: f64 = 6.07710050650619224932e-11;
    const P2: f64 = 6.07710050630396597660e-11;
    const P2T: f64 = 2.02226624879595063154e-21;
    const P3: f64 = 2.02226624871116645580e-21;
    const P3T: f64 = 8.47842766036889956997e-32;
    const HW: [u32; 32] = [
        0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB,
        0x402921FB, 0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB,
        0x40378FDB, 0x403921FB, 0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C,
        0x4041475C, 0x4042106C, 0x4042D97C, 0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB,
        0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
    ];
    let hx = high(x) as i32;
    let ix = (hx as u32) & 0x7fff_ffff;
    if ix <= 0x3fe921fb {
        return (0, x, 0.);
    }
    if ix < 0x4002d97c {
        if hx > 0 {
            let mut z = x - P1;
            let (y0, y1) = if ix != 0x3ff921fb {
                let y = z - P1T;
                (y, (z - y) - P1T)
            } else {
                z -= P2;
                let y = z - P2T;
                (y, (z - y) - P2T)
            };
            return (1, y0, y1);
        } else {
            let mut z = x + P1;
            let (y0, y1) = if ix != 0x3ff921fb {
                let y = z + P1T;
                (y, (z - y) + P1T)
            } else {
                z += P2;
                let y = z + P2T;
                (y, (z - y) + P2T)
            };
            return (-1, y0, y1);
        }
    }
    assert!(
        ix <= 0x413921fb,
        "Venn angles stay in fdlibm's medium reduction range"
    );
    let mut t = x.abs();
    let n = t.mul_add(6.36619772367581382433e-1, 0.5) as i32;
    let f = n as f64;
    let mut r = (-f).mul_add(P1, t);
    let mut w = f * P1T;
    let mut y0 = r - w;
    if !(n < 32 && ix != HW[n as usize - 1]) {
        let j = (ix >> 20) as i32;
        let mut i = j - ((high(y0) >> 20) & 0x7ff) as i32;
        if i > 16 {
            t = r;
            w = f * P2;
            r = t - w;
            w = f.mul_add(P2T, -((t - r) - w));
            y0 = r - w;
            i = j - ((high(y0) >> 20) & 0x7ff) as i32;
            if i > 49 {
                t = r;
                w = f * P3;
                r = t - w;
                w = f.mul_add(P3T, -((t - r) - w));
                y0 = r - w;
            }
        }
    }
    let y1 = (r - y0) - w;
    if hx < 0 {
        (-n, -y0, -y1)
    } else {
        (n, y0, y1)
    }
}
pub(super) fn cos(x: f64) -> f64 {
    let ix = high(x) & 0x7fff_ffff;
    if ix <= 0x3fe921fb {
        return kernel_cos(x, 0.);
    }
    if ix >= 0x7ff00000 {
        return x - x;
    }
    let (n, y0, y1) = reduce(x);
    match n & 3 {
        0 => kernel_cos(y0, y1),
        1 => -kernel_sin(y0, y1, true),
        2 => -kernel_cos(y0, y1),
        _ => kernel_sin(y0, y1, true),
    }
}
pub(super) fn sin(x: f64) -> f64 {
    let ix = high(x) & 0x7fff_ffff;
    if ix <= 0x3fe921fb {
        return kernel_sin(x, 0., false);
    }
    if ix >= 0x7ff00000 {
        return x - x;
    }
    let (n, y0, y1) = reduce(x);
    match n & 3 {
        0 => kernel_sin(y0, y1, true),
        1 => kernel_cos(y0, y1),
        2 => -kernel_sin(y0, y1, true),
        _ => -kernel_cos(y0, y1),
    }
}
// Node's arm64 build contracts multiply/add expressions in the C++ acos routine.
// Explicit mul_add preserves that rounding in native and WASM Rust alike.
fn rational(z: f64) -> f64 {
    let p = z * z.mul_add(
        z.mul_add(
            z.mul_add(
                z.mul_add(
                    z.mul_add(3.47933107596021167570e-5, 7.91534994289814532176e-4),
                    -4.00555345006794114027e-2,
                ),
                2.01212532134862925881e-1,
            ),
            -3.25565818622400915405e-1,
        ),
        1.66666666666666657415e-1,
    );
    let q = z.mul_add(
        z.mul_add(
            z.mul_add(
                z.mul_add(7.70381505559019352791e-2, -6.88283971605453293030e-1),
                2.02094576023350569471,
            ),
            -2.40339491173441421878,
        ),
        1.,
    );
    p / q
}

pub(super) fn acos(x: f64) -> f64 {
    let hx = high(x) as i32;
    let ix = hx as u32 & 0x7fff_ffff;
    let pi = std::f64::consts::PI;
    let hi = std::f64::consts::FRAC_PI_2;
    let lo = 6.12323399573676603587e-17;
    if ix >= 0x3ff00000 {
        if ((ix - 0x3ff00000) | low(x)) == 0 {
            return if hx > 0 { 0. } else { pi + 2. * lo };
        }
        return f64::NAN;
    }
    if ix < 0x3fe00000 {
        if ix <= 0x3c600000 {
            return hi + lo;
        }
        let z = x * x;
        let r = rational(z);
        hi - (x - x.mul_add(-r, lo))
    } else if hx < 0 {
        let z = (1. + x) * 0.5;
        let s = z.sqrt();
        let r = rational(z);
        let w = r.mul_add(s, -lo);
        (s + w).mul_add(-2., pi)
    } else {
        let z = (1. - x) * 0.5;
        let s = z.sqrt();
        let df = truncate_low(s);
        let c = df.mul_add(-df, z) / (s + df);
        let r = rational(z);
        let w = r.mul_add(s, c);
        2. * (df + w)
    }
}
fn atan(mut x: f64) -> f64 {
    const HI: [f64; 4] = [
        4.63647609000806093515e-1,
        7.85398163397448278999e-1,
        9.82793723247329054082e-1,
        1.57079632679489655800,
    ];
    const LO: [f64; 4] = [
        2.26987774529616870924e-17,
        3.06161699786838301793e-17,
        1.39033110312309984516e-17,
        6.12323399573676603587e-17,
    ];
    const A: [f64; 11] = [
        3.33333333333329318027e-1,
        -1.99999999998764832476e-1,
        1.42857142725034663711e-1,
        -1.11111104054623557880e-1,
        9.09088713343650656196e-2,
        -7.69187620504482999495e-2,
        6.66107313738753120669e-2,
        -5.83357013379057348645e-2,
        4.97687799461593236017e-2,
        -3.65315727442169155270e-2,
        1.62858201153657823623e-2,
    ];
    let hx = high(x) as i32;
    let ix = hx as u32 & 0x7fff_ffff;
    if ix >= 0x44100000 {
        if x.is_nan() {
            return x + x;
        }
        return if hx > 0 {
            HI[3] + LO[3]
        } else {
            -HI[3] - LO[3]
        };
    }
    let id = if ix < 0x3fdc0000 {
        if ix < 0x3e400000 {
            return x;
        }
        -1
    } else {
        x = x.abs();
        if ix < 0x3ff30000 {
            if ix < 0x3fe60000 {
                x = x.mul_add(2., -1.) / (2. + x);
                0
            } else {
                x = (x - 1.) / (x + 1.);
                1
            }
        } else if ix < 0x40038000 {
            x = (x - 1.5) / x.mul_add(1.5, 1.);
            2
        } else {
            x = -1. / x;
            3
        }
    };
    let z = x * x;
    let w = z * z;
    let s1 = z * w.mul_add(
        w.mul_add(
            w.mul_add(w.mul_add(w.mul_add(A[10], A[8]), A[6]), A[4]),
            A[2],
        ),
        A[0],
    );
    let s2 = w * w.mul_add(
        w.mul_add(w.mul_add(w.mul_add(A[9], A[7]), A[5]), A[3]),
        A[1],
    );
    if id < 0 {
        (-x).mul_add(s1 + s2, x)
    } else {
        let i = id as usize;
        let z = HI[i] - (x.mul_add(s1 + s2, -LO[i]) - x);
        if hx < 0 {
            -z
        } else {
            z
        }
    }
}
pub(super) fn atan2(y: f64, x: f64) -> f64 {
    let hx = high(x) as i32;
    let hy = high(y) as i32;
    let ix = hx as u32 & 0x7fff_ffff;
    let iy = hy as u32 & 0x7fff_ffff;
    let pi = std::f64::consts::PI;
    let half = std::f64::consts::FRAC_PI_2;
    let quarter = std::f64::consts::FRAC_PI_4;
    let lo = 1.2246467991473531772e-16;
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    if x == 1. {
        return atan(y);
    }
    let mut m = ((hy >> 31) & 1) | ((hx >> 30) & 2);
    if y == 0. {
        return match m {
            0 | 1 => y,
            2 => pi,
            _ => -pi,
        };
    }
    if x == 0. {
        return if hy < 0 { -half } else { half };
    }
    if x.is_infinite() {
        if y.is_infinite() {
            return match m {
                0 => quarter,
                1 => -quarter,
                2 => 3. * quarter,
                _ => -3. * quarter,
            };
        }
        return match m {
            0 => 0.,
            1 => -0.,
            2 => pi,
            _ => -pi,
        };
    }
    if y.is_infinite() {
        return if hy < 0 { -half } else { half };
    }
    let k = (iy as i32 - ix as i32) >> 20;
    let z = if k > 60 {
        m &= 1;
        half + 0.5 * lo
    } else if hx < 0 && k < -60 {
        0.
    } else {
        atan((y / x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => pi - (z - lo),
        _ => (z - lo) - pi,
    }
}

// JavaScript Math.min/max propagate NaN and distinguish the sign of zero.
pub(super) fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0. && b == 0. {
        f64::from_bits(a.to_bits() | b.to_bits())
    } else {
        a.min(b)
    }
}
pub(super) fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0. && b == 0. {
        f64::from_bits(a.to_bits() & b.to_bits())
    } else {
        a.max(b)
    }
}
