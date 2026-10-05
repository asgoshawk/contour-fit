use crate::FitError;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }
    pub fn normalized(self) -> Self {
        let n = self.length();
        if n > 1e-12 { self / n } else { Self::default() }
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
impl Add for Point {
    type Output = Self;
    fn add(self, p: Self) -> Self {
        Self::new(self.x + p.x, self.y + p.y)
    }
}
impl Sub for Point {
    type Output = Self;
    fn sub(self, p: Self) -> Self {
        Self::new(self.x - p.x, self.y - p.y)
    }
}
impl Mul<f64> for Point {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}
impl Div<f64> for Point {
    type Output = Self;
    fn div(self, s: f64) -> Self {
        Self::new(self.x / s, self.y / s)
    }
}
impl Neg for Point {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    Line {
        start: Point,
        end: Point,
    },
    Cubic {
        start: Point,
        control1: Point,
        control2: Point,
        end: Point,
    },
}
impl Curve {
    pub fn start(self) -> Point {
        match self {
            Self::Line { start, .. } | Self::Cubic { start, .. } => start,
        }
    }
    pub fn end(self) -> Point {
        match self {
            Self::Line { end, .. } | Self::Cubic { end, .. } => end,
        }
    }
    pub fn controls(self) -> [Point; 4] {
        match self {
            Self::Line { start, end } => [
                start,
                start + (end - start) / 3.0,
                start + (end - start) * (2.0 / 3.0),
                end,
            ],
            Self::Cubic {
                start,
                control1,
                control2,
                end,
            } => [start, control1, control2, end],
        }
    }
    pub fn point(self, t: f64) -> Point {
        let [a, b, c, d] = self.controls();
        let s = 1.0 - t;
        a * (s * s * s) + b * (3.0 * s * s * t) + c * (3.0 * s * t * t) + d * (t * t * t)
    }
    pub(crate) fn derivatives(self, t: f64) -> (Point, Point) {
        let [a, b, c, d] = self.controls();
        let s = 1.0 - t;
        (
            (b - a) * (3.0 * s * s) + (c - b) * (6.0 * s * t) + (d - c) * (3.0 * t * t),
            (c - b * 2.0 + a) * (6.0 * s) + (d - c * 2.0 + b) * (6.0 * t),
        )
    }
    pub(crate) fn split(self) -> (Self, Self) {
        let [a, b, c, d] = self.controls();
        let ab = (a + b) / 2.0;
        let bc = (b + c) / 2.0;
        let cd = (c + d) / 2.0;
        let abc = (ab + bc) / 2.0;
        let bcd = (bc + cd) / 2.0;
        let mid = (abc + bcd) / 2.0;
        (
            Self::Cubic {
                start: a,
                control1: ab,
                control2: abc,
                end: mid,
            },
            Self::Cubic {
                start: mid,
                control1: bcd,
                control2: cd,
                end: d,
            },
        )
    }
    pub(crate) fn flatten(self, epsilon: f64, limit: usize) -> Result<Vec<Point>, FitError> {
        let mut points = vec![self.start()];
        let mut stack = vec![(self, 0)];
        while let Some((curve, depth)) = stack.pop() {
            let [a, b, c, d] = curve.controls();
            if distance_to_segment(b, a, d).max(distance_to_segment(c, a, d)) <= epsilon {
                points.push(d);
                if points.len() > limit {
                    return Err(FitError::ResourceLimit("flattened curve points"));
                }
            } else {
                if depth >= 32 {
                    return Err(FitError::Quality("curve flattening did not converge"));
                }
                let (left, right) = curve.split();
                stack.push((right, depth + 1));
                stack.push((left, depth + 1));
            }
        }
        Ok(points)
    }
    pub fn rounded(self, decimals: u32) -> Self {
        let factor = 10_f64.powi(decimals.min(12) as i32);
        let round = |p: Point| {
            Point::new(
                (p.x * factor).round() / factor,
                (p.y * factor).round() / factor,
            )
        };
        match self {
            Self::Line { start, end } => Self::Line {
                start: round(start),
                end: round(end),
            },
            Self::Cubic {
                start,
                control1,
                control2,
                end,
            } => Self::Cubic {
                start: round(start),
                control1: round(control1),
                control2: round(control2),
                end: round(end),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VectorPath {
    pub segments: Vec<Curve>,
    pub parent: Option<usize>,
    pub depth: usize,
}

pub(crate) fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let line = kurbo::Line::new(kurbo::Point::new(a.x, a.y), kurbo::Point::new(b.x, b.y));
    use kurbo::ParamCurveNearest;
    line.nearest(kurbo::Point::new(p.x, p.y), 1e-9)
        .distance_sq
        .sqrt()
}
pub(crate) fn area(points: &[Point]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.cross(*b))
        .sum::<f64>()
        / 2.0
}
pub(crate) fn inside(p: Point, polygon: &[Point]) -> bool {
    let mut result = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            result = !result;
        }
    }
    result
}

pub(crate) fn polygon_bounds(points: &[Point]) -> (Point, Point) {
    points.iter().fold((points[0], points[0]), |(min, max), p| {
        (
            Point::new(min.x.min(p.x), min.y.min(p.y)),
            Point::new(max.x.max(p.x), max.y.max(p.y)),
        )
    })
}
pub(crate) fn bounds_contain((min, max): (Point, Point), p: Point) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}
