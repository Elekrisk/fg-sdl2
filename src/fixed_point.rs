use std::{
    error::Error,
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

type FixedPointType = i32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub struct FixedPoint(FixedPointType);

impl Display for FixedPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{}", f64::from(*self))
        f64::from(*self).fmt(f)
    }
}

fn change_scaling_factor(value: FixedPointType, from: i32, to: i32) -> FixedPointType {
    let change = to - from;
    if change >= 0 {
        value << change
    } else {
        println!("{change}");
        value >> -change
    }
}

impl FixedPoint {
    pub const ZERO: Self = Self(0);

    const DECIMAL_PLACES: usize = 8;

    pub fn significant_digits(mut self) -> usize {
        if self.0 < 0 {
            self.0 = -self.0;
        }
        (FixedPointType::BITS - self.0.leading_zeros() + self.0.trailing_zeros()) as _
    }

    fn whole(self) -> FixedPointType {
        self.0 >> Self::DECIMAL_PLACES
    }

    fn frac(self) -> FixedPointType {
        self.0 & (2i32.pow(Self::DECIMAL_PLACES as _) - 1)
    }

    pub fn trunc(self) -> Self {
        Self(self.whole() << Self::DECIMAL_PLACES)
    }

    pub fn round(self) -> Self {
        let frac = self.frac();
        if frac & (1 << (Self::DECIMAL_PLACES - 1)) != 1 {
            Self((self.whole() + 1) << Self::DECIMAL_PLACES)
        } else {
            self.trunc()
        }
    }

    pub fn abs(self) -> Self {
        if self < FixedPoint::ZERO {
            -self
        } else {
            self
        }
    }
}

impl Neg for FixedPoint {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for FixedPoint {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for FixedPoint {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedPoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for FixedPoint {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for FixedPoint {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self((self.0 * rhs.0) >> Self::DECIMAL_PLACES)
    }
}

impl MulAssign for FixedPoint {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for FixedPoint {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut numerator = self.0;
        let mut denominator = rhs.0;
        let mut numerator_scaling_factor = Self::DECIMAL_PLACES as i32;
        let mut denominator_scaling_factor = Self::DECIMAL_PLACES as i32;

        let mut sign = 1;

        if numerator < 0 {
            numerator *= -1;
            sign *= -1;
        }

        if denominator < 0 {
            denominator *= -1;
            sign *= -1;
        }

        let numerator_adjust = numerator.leading_zeros() as i32 - 1;
        let denominator_adjust = denominator.trailing_zeros() as i32;

        numerator <<= numerator_adjust;
        numerator_scaling_factor += numerator_adjust;
        denominator >>= denominator_adjust;
        denominator_scaling_factor -= denominator_adjust;

        let res = numerator / denominator;
        let scaling_factor = numerator_scaling_factor - denominator_scaling_factor;

        let res = change_scaling_factor(res, scaling_factor, Self::DECIMAL_PLACES as _);

        Self(res * sign)
    }
}

impl From<f32> for FixedPoint {
    fn from(value: f32) -> Self {
        let whole = value.trunc();

        let fract = value.fract();

        let adjusted_fract = fract * 2.0_f32.powi(Self::DECIMAL_PLACES as _);

        let int_whole = (whole as i32) << Self::DECIMAL_PLACES;
        let int_fract = adjusted_fract as i32;

        Self(int_whole | int_fract)
    }
}

impl From<usize> for FixedPoint {
    fn from(value: usize) -> Self {
        Self((value as i32) << Self::DECIMAL_PLACES)
    }
}

impl From<FixedPoint> for i32 {
    fn from(value: FixedPoint) -> Self {
        value.0 >> FixedPoint::DECIMAL_PLACES
    }
}

#[derive(Debug)]
pub struct TryFromError;

impl Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("value too thicc")
    }
}

impl Error for TryFromError {}

impl TryFrom<FixedPoint> for f32 {
    type Error = ();

    fn try_from(value: FixedPoint) -> Result<Self, Self::Error> {
        // if value.significant_digits() > f32::MANTISSA_DIGITS as _ {
        //     return Err(());
        // }

        Ok((value.0 as f32) / 2.0_f32.powi(FixedPoint::DECIMAL_PLACES as _))
    }
}

impl From<FixedPoint> for f64 {
    fn from(value: FixedPoint) -> Self {
        value.0 as f64 / 2.0_f64.powi(FixedPoint::DECIMAL_PLACES as _)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Vec2 {
    pub x: FixedPoint,
    pub y: FixedPoint,
}

impl Vec2 {
    pub fn new(x: FixedPoint, y: FixedPoint) -> Self {
        Self { x, y }
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<FixedPoint> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: FixedPoint) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<FixedPoint> for Vec2 {
    fn mul_assign(&mut self, rhs: FixedPoint) {
        *self = *self * rhs;
    }
}

impl Div<FixedPoint> for Vec2 {
    type Output = Self;

    fn div(self, rhs: FixedPoint) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl DivAssign<FixedPoint> for Vec2 {
    fn div_assign(&mut self, rhs: FixedPoint) {
        *self = *self / rhs;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn left(&self) -> FixedPoint {
        self.min.x
    }

    pub fn right(&self) -> FixedPoint {
        self.max.x
    }

    pub fn top(&self) -> FixedPoint {
        self.min.y
    }

    pub fn bottom(&self) -> FixedPoint {
        self.max.y
    }

    pub fn overlaps(self, other: Rect) -> bool {
        self.right() > other.left()
            && self.left() < other.right()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    pub fn offset(self, offset: Vec2) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset
        }
    }
}
