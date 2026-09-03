//! Q16.16 fixed-point arithmetic for the neural scheduler hot path.
//!
//! Keeps learning math integer-only so the MLP can run without relying on a
//! soft-float library (and can later drop the FPU entirely).

#![allow(dead_code)]

/// Signed Q16.16 fixed-point value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed(pub i32);

pub const ZERO: Fixed = Fixed(0);
pub const ONE: Fixed = Fixed(1 << 16);
pub const HALF: Fixed = Fixed(1 << 15);

impl Fixed {
    pub const fn from_i32(v: i32) -> Self {
        Fixed(v << 16)
    }

    /// Convert from f32 for host tests / display bridges.
    pub fn from_f32(v: f32) -> Self {
        Fixed((v * 65536.0) as i32)
    }

    pub fn to_f32(self) -> f32 {
        self.0 as f32 / 65536.0
    }

    pub const fn from_bits(raw: i32) -> Self {
        Fixed(raw)
    }

    pub const fn to_bits(self) -> i32 {
        self.0
    }

    #[inline]
    pub fn saturating_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }

    #[inline]
    pub fn saturating_sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(rhs.0))
    }

    #[inline]
    pub fn wrapping_mul(self, rhs: Fixed) -> Fixed {
        let prod = (self.0 as i64) * (rhs.0 as i64);
        Fixed((prod >> 16) as i32)
    }

    #[inline]
    pub fn abs(self) -> Fixed {
        Fixed(self.0.saturating_abs())
    }

    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[inline]
    pub fn max(self, other: Fixed) -> Fixed {
        if self.0 >= other.0 { self } else { other }
    }

    #[inline]
    pub fn min(self, other: Fixed) -> Fixed {
        if self.0 <= other.0 { self } else { other }
    }

    #[inline]
    pub fn clamp(self, lo: Fixed, hi: Fixed) -> Fixed {
        self.max(lo).min(hi)
    }
}

impl core::ops::Add for Fixed {
    type Output = Fixed;
    fn add(self, rhs: Fixed) -> Fixed {
        self.saturating_add(rhs)
    }
}

impl core::ops::Sub for Fixed {
    type Output = Fixed;
    fn sub(self, rhs: Fixed) -> Fixed {
        self.saturating_sub(rhs)
    }
}

impl core::ops::Mul for Fixed {
    type Output = Fixed;
    fn mul(self, rhs: Fixed) -> Fixed {
        self.wrapping_mul(rhs)
    }
}

impl core::ops::Neg for Fixed {
    type Output = Fixed;
    fn neg(self) -> Fixed {
        Fixed(self.0.saturating_neg())
    }
}

impl core::ops::AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Fixed) {
        *self = *self + rhs;
    }
}

/// Fast sigmoid approximation: 0.5 + 0.5 * x / (1 + |x|).
#[inline]
pub fn sigmoid(x: Fixed) -> Fixed {
    let denom = ONE + x.abs();
    // reciprocal via integer division in Q16.16: (1<<16) / denom.raw * scale
    // Compute x / (1+|x|) as (x << 16) / denom
    if denom.0 == 0 {
        return HALF;
    }
    let ratio = Fixed((((x.0 as i64) << 16) / (denom.0 as i64)) as i32);
    HALF + HALF * ratio
}

/// Derivative of the approximation above, evaluated at the *output* y = sigmoid(x).
/// For y = 0.5 + 0.5 * t with t = x/(1+|x|), dy/dx ≈ 0.5 / (1+|x|)^2.
/// In terms of y alone this is messy; use y*(1-y) as a stable surrogate (same
/// shape as logistic sigmoid) which works well enough for online SGD.
#[inline]
pub fn sigmoid_derivative_from_output(y: Fixed) -> Fixed {
    y * (ONE - y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_times_one() {
        assert_eq!((ONE * ONE).to_bits(), ONE.to_bits());
    }

    #[test]
    fn from_to_f32_roundtrip() {
        let v = Fixed::from_f32(0.5);
        assert!((v.to_f32() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn sigmoid_bounds() {
        let s0 = sigmoid(ZERO).to_f32();
        assert!((s0 - 0.5).abs() < 0.02);
        assert!(sigmoid(Fixed::from_i32(10)).to_f32() > 0.9);
        assert!(sigmoid(Fixed::from_i32(-10)).to_f32() < 0.1);
    }
}
