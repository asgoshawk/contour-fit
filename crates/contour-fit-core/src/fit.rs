use crate::geometry::distance_to_segment;
use crate::{Curve, FitError, FitOptions, Point};

#[derive(Clone, Debug)]
pub(crate) struct Piece {
    pub curve: Curve,
    pub start: usize,
    pub end: usize,
}
#[derive(Debug)]
pub(crate) struct Prepared {
    pub points: Vec<Point>,
    pub corners: Vec<bool>,
    pub pieces: Vec<Piece>,
}

pub(crate) fn fit_ring(
    mut points: Vec<Point>,
    corners: Vec<bool>,
    options: &FitOptions,
    work: &mut usize,
    canvas: Point,
) -> Result<Prepared, FitError> {
    let n = points.len();
    let mut anchors: Vec<_> = (0..n).filter(|&i| corners[i]).collect();
    anchors.push(0);
    if anchors.len() < 3 {
        anchors.extend([n / 4, n / 2, 3 * n / 4]);
    }
    anchors.sort_unstable();
    anchors.dedup();
    anchors.push(n);
    points.push(points[0]);
    let mut pieces = Vec::new();
    for range in anchors.windows(2) {
        let (start, end) = (range[0], range[1]);
        let start_tangent = tangent(&points, start % n, corners[start % n], true);
        let end_tangent = tangent(&points, end % n, corners[end % n], false);
        let mut stack = vec![(start, end, start_tangent, end_tangent)];
        while let Some((a, b, ta, tb)) = stack.pop() {
            *work += 1;
            if *work > options.limits.max_fit_attempts {
                return Err(FitError::ResourceLimit("fit attempts"));
            }
            let (curve, error, split) = fit_one(&points[a..=b], ta, tb, canvas);
            if error <= options.tolerance * 0.35 || b - a == 1 {
                pieces.push(Piece {
                    curve,
                    start: a,
                    end: b,
                });
                if pieces.len() > options.limits.max_segments {
                    return Err(FitError::ResourceLimit("curve segments"));
                }
            } else {
                let k = a + split.clamp(1, b - a - 1);
                let mid = (points[k + 1] - points[k - 1]).normalized();
                stack.push((k, b, mid, tb));
                stack.push((a, k, ta, -mid));
            }
        }
    }
    Ok(Prepared {
        points,
        corners,
        pieces,
    })
}

fn tangent(points: &[Point], i: usize, corner: bool, start: bool) -> Point {
    let n = points.len() - 1;
    if corner {
        if start {
            (points[(i + 1) % n] - points[i]).normalized()
        } else {
            (points[(i + n - 1) % n] - points[i]).normalized()
        }
    } else {
        let forward = (points[(i + 1) % n] - points[(i + n - 1) % n]).normalized();
        if start { forward } else { -forward }
    }
}

pub(crate) fn detect_corners(points: &[Point], degrees: f64) -> Vec<bool> {
    let n = points.len();
    let mut turns = vec![0.0; n];
    for i in 0..n {
        // Probe two physical scales, retaining a corner only when both agree.
        let angle_at = |radius: f64| {
            let mut before = i;
            let mut after = i;
            let mut s = 0.0;
            while s < radius {
                let next = (before + n - 1) % n;
                s += points[before].distance(points[next]);
                before = next;
                if before == i {
                    break;
                }
            }
            s = 0.0;
            while s < radius {
                let next = (after + 1) % n;
                s += points[after].distance(points[next]);
                after = next;
                if after == i {
                    break;
                }
            }
            let a = (points[i] - points[before]).normalized();
            let b = (points[after] - points[i]).normalized();
            a.dot(b).clamp(-1.0, 1.0).acos().to_degrees()
        };
        turns[i] = angle_at(1.0).min(angle_at(2.0));
    }
    let mut candidates: Vec<_> = (0..n).filter(|&i| turns[i] >= degrees).collect();
    candidates.sort_by(|&a, &b| turns[b].total_cmp(&turns[a]).then(a.cmp(&b)));
    let mut selected = vec![false; n];
    let mut selected_indices: Vec<usize> = Vec::new();
    for i in candidates {
        if !selected_indices
            .iter()
            .any(|&j| points[i].distance(points[j]) < 1.0)
        {
            selected[i] = true;
            selected_indices.push(i);
        }
    }
    selected
}

pub(crate) fn fit_one(
    points: &[Point],
    ta: Point,
    tb: Point,
    canvas: Point,
) -> (Curve, f64, usize) {
    let start = points[0];
    let end = *points.last().unwrap();
    let direction = (end - start).normalized();
    if ta.dot(direction) > 1.0 - 1e-10 && (-tb).dot(direction) > 1.0 - 1e-10 {
        let (split, error) = points
            .iter()
            .enumerate()
            .map(|(i, &p)| (i, distance_to_segment(p, start, end)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .unwrap();
        if error < 1e-9 {
            return (Curve::Line { start, end }, error, split);
        }
    }
    let mut parameters = vec![0.0; points.len()];
    for i in 1..points.len() {
        parameters[i] = parameters[i - 1] + points[i].distance(points[i - 1]);
    }
    let length = *parameters.last().unwrap();
    if length > 1e-12 {
        for t in &mut parameters {
            *t /= length;
        }
    } else {
        return (Curve::Line { start, end }, 0.0, points.len() / 2);
    }
    let mut best = (Curve::Line { start, end }, f64::INFINITY, points.len() / 2);
    for _ in 0..5 {
        let curve = least_squares(points, &parameters, ta, tb, canvas);
        let (split, error) = points
            .iter()
            .zip(&parameters)
            .enumerate()
            .map(|(i, (&p, &t))| (i, p.distance(curve.point(t))))
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .unwrap();
        if !error.is_finite() || error >= best.1 {
            break;
        }
        best = (curve, error, split);
        let mut updated = parameters.clone();
        for i in 1..points.len() - 1 {
            let t = parameters[i];
            let delta = curve.point(t) - points[i];
            let (first, second) = curve.derivatives(t);
            let denominator = first.dot(first) + delta.dot(second);
            if denominator.abs() > 1e-12 {
                updated[i] = (t - delta.dot(first) / denominator).clamp(0.0, 1.0);
            }
        }
        if updated.windows(2).any(|t| t[0] >= t[1]) {
            break;
        }
        parameters = updated;
    }
    best
}

fn least_squares(
    points: &[Point],
    parameters: &[f64],
    ta: Point,
    tb: Point,
    canvas: Point,
) -> Curve {
    let start = points[0];
    let end = *points.last().unwrap();
    let mut c00 = 0.0;
    let mut c01 = 0.0;
    let mut c11 = 0.0;
    let mut x0 = 0.0;
    let mut x1 = 0.0;
    for (&p, &t) in points.iter().zip(parameters) {
        let s = 1.0 - t;
        let b0 = s * s * s;
        let b1 = 3.0 * s * s * t;
        let b2 = 3.0 * s * t * t;
        let b3 = t * t * t;
        let a = ta * b1;
        let b = tb * b2;
        let residual = p - (start * (b0 + b1) + end * (b2 + b3));
        c00 += a.dot(a);
        c01 += a.dot(b);
        c11 += b.dot(b);
        x0 += a.dot(residual);
        x1 += b.dot(residual);
    }
    let chord = start.distance(end);
    let determinant = c00 * c11 - c01 * c01;
    let (mut alpha, mut beta) = if determinant.abs() > 1e-12 * (c00 * c11).max(1.0) {
        (
            (x0 * c11 - x1 * c01) / determinant,
            (c00 * x1 - c01 * x0) / determinant,
        )
    } else {
        (chord / 3.0, chord / 3.0)
    };
    if !alpha.is_finite()
        || !beta.is_finite()
        || alpha <= chord * 1e-6
        || beta <= chord * 1e-6
        || alpha > chord * 10.0
        || beta > chord * 10.0
    {
        alpha = chord / 3.0;
        beta = chord / 3.0;
    }
    Curve::Cubic {
        start,
        control1: bounded_handle(start, ta * alpha, canvas),
        control2: bounded_handle(end, tb * beta, canvas),
        end,
    }
}

// Clip handle length along its tangent, preserving direction and therefore G1.
fn bounded_handle(anchor: Point, handle: Point, canvas: Point) -> Point {
    let mut scale = 1.0_f64;
    for (position, delta, maximum) in [
        (anchor.x, handle.x, canvas.x),
        (anchor.y, handle.y, canvas.y),
    ] {
        if delta > 0.0 {
            scale = scale.min((maximum - position) / delta);
        } else if delta < 0.0 {
            scale = scale.min(-position / delta);
        }
    }
    anchor + handle * scale.clamp(0.0, 1.0)
}

pub(crate) fn protected_corners(points: &[Point], degrees: f64, canvas: Point) -> Vec<bool> {
    let mut corners = detect_corners(points, degrees);
    for i in 0..points.len() {
        let before = points[(i + points.len() - 1) % points.len()];
        let after = points[(i + 1) % points.len()];
        let p = points[i];
        for (value, a, b, edge) in [
            (p.x, before.x, after.x, 0.0),
            (p.x, before.x, after.x, canvas.x),
            (p.y, before.y, after.y, 0.0),
            (p.y, before.y, after.y, canvas.y),
        ] {
            if (value - edge).abs() < 1e-9 && ((a - edge).abs() >= 1e-9 || (b - edge).abs() >= 1e-9)
            {
                corners[i] = true;
            }
        }
    }
    corners
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn straight_samples_fit_one_line() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(10.0, 0.0),
        ];
        let (curve, error, _) = fit_one(
            &points,
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            Point::new(100.0, 100.0),
        );
        assert!(matches!(curve, Curve::Line { .. }));
        assert!(error < 1e-10);
    }
    #[test]
    fn repeated_points_do_not_produce_nan() {
        let points = [Point::new(1.0, 1.0); 4];
        let (curve, error, _) = fit_one(
            &points,
            Point::default(),
            Point::default(),
            Point::new(100.0, 100.0),
        );
        assert!(error.is_finite());
        assert!(curve.controls().iter().all(|p| p.is_finite()));
    }
}
