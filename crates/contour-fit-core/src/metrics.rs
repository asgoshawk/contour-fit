use crate::geometry::{area, bounds_contain, distance_to_segment, inside, polygon_bounds};
use crate::{FitError, FitOptions, Point, VectorPath};

#[derive(Clone, Debug, PartialEq)]
pub struct QualityReport {
    pub contours: usize,
    pub segments: usize,
    /// Source-to-output distances estimated from uniform arc-length samples.
    pub rms_error_px: f64,
    pub p95_error_px: f64,
    pub sampled_hausdorff_px: f64,
    /// Conservative geometric bound, including sampling and flattening slack.
    /// Floating-point rounding is covered by a small numerical margin, not a formal proof.
    pub hausdorff_upper_bound_px: f64,
    pub flattening_error_px: f64,
    pub topology_resolution_px: f64,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    a: Point,
    b: Point,
    ring: usize,
    index: usize,
    count: usize,
}
#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: Point,
    max: Point,
}
impl Bounds {
    fn segment(s: Segment) -> Self {
        Self {
            min: Point::new(s.a.x.min(s.b.x), s.a.y.min(s.b.y)),
            max: Point::new(s.a.x.max(s.b.x), s.a.y.max(s.b.y)),
        }
    }
    fn union(self, b: Self) -> Self {
        Self {
            min: Point::new(self.min.x.min(b.min.x), self.min.y.min(b.min.y)),
            max: Point::new(self.max.x.max(b.max.x), self.max.y.max(b.max.y)),
        }
    }
    fn expanded(self, d: f64) -> Self {
        Self {
            min: self.min - Point::new(d, d),
            max: self.max + Point::new(d, d),
        }
    }
    fn overlaps(self, b: Self) -> bool {
        self.min.x <= b.max.x
            && b.min.x <= self.max.x
            && self.min.y <= b.max.y
            && b.min.y <= self.max.y
    }
    fn distance(self, p: Point) -> f64 {
        let dx = (self.min.x - p.x).max(0.0).max(p.x - self.max.x);
        let dy = (self.min.y - p.y).max(0.0).max(p.y - self.max.y);
        dx * dx + dy * dy
    }
}
#[derive(Debug)]
struct Node {
    bounds: Bounds,
    kind: NodeKind,
}
#[derive(Debug)]
enum NodeKind {
    Leaf(Vec<Segment>),
    Branch(Box<Node>, Box<Node>),
}
impl Node {
    fn build(mut segments: Vec<Segment>) -> Self {
        let bounds = segments
            .iter()
            .map(|&s| Bounds::segment(s))
            .reduce(Bounds::union)
            .unwrap();
        if segments.len() <= 8 {
            return Self {
                bounds,
                kind: NodeKind::Leaf(segments),
            };
        }
        let horizontal = bounds.max.x - bounds.min.x >= bounds.max.y - bounds.min.y;
        segments.sort_by(|a, b| {
            let value = |s: &Segment| {
                if horizontal {
                    s.a.x + s.b.x
                } else {
                    s.a.y + s.b.y
                }
            };
            value(a)
                .total_cmp(&value(b))
                .then(a.ring.cmp(&b.ring))
                .then(a.index.cmp(&b.index))
        });
        let right = segments.split_off(segments.len() / 2);
        Self {
            bounds,
            kind: NodeKind::Branch(
                Box::new(Self::build(segments)),
                Box::new(Self::build(right)),
            ),
        }
    }
    fn nearest(&self, p: Point, best: &mut f64) {
        if self.bounds.distance(p) >= *best {
            return;
        }
        match &self.kind {
            NodeKind::Leaf(segments) => {
                for s in segments {
                    *best = best.min(distance_to_segment(p, s.a, s.b).powi(2));
                }
            }
            NodeKind::Branch(a, b) => {
                let (first, second) = if a.bounds.distance(p) <= b.bounds.distance(p) {
                    (a, b)
                } else {
                    (b, a)
                };
                first.nearest(p, best);
                second.nearest(p, best);
            }
        }
    }
    fn distance(&self, p: Point) -> f64 {
        let mut best = f64::INFINITY;
        self.nearest(p, &mut best);
        best.sqrt()
    }
    fn topology(
        &self,
        segment: Segment,
        slack: f64,
        work: &mut usize,
        limit: usize,
    ) -> Result<(), FitError> {
        if !self
            .bounds
            .overlaps(Bounds::segment(segment).expanded(slack))
        {
            return Ok(());
        }
        match &self.kind {
            NodeKind::Branch(a, b) => {
                a.topology(segment, slack, work, limit)?;
                b.topology(segment, slack, work, limit)?;
            }
            NodeKind::Leaf(segments) => {
                for &other in segments {
                    if (other.ring, other.index) <= (segment.ring, segment.index) {
                        continue;
                    }
                    if other.ring == segment.ring {
                        let difference = other.index.abs_diff(segment.index);
                        if difference == 1 || difference == segment.count - 1 {
                            continue;
                        }
                    }
                    if !Bounds::segment(other).overlaps(Bounds::segment(segment).expanded(slack)) {
                        continue;
                    }
                    *work += 1;
                    if *work > limit {
                        return Err(FitError::ResourceLimit("topology comparisons"));
                    }
                    if intersects(segment, other) {
                        return Err(FitError::Quality("intersecting or touching contours"));
                    }
                    let separation = distance_to_segment(segment.a, other.a, other.b)
                        .min(distance_to_segment(segment.b, other.a, other.b))
                        .min(distance_to_segment(other.a, segment.a, segment.b))
                        .min(distance_to_segment(other.b, segment.a, segment.b));
                    if separation <= slack {
                        return Err(FitError::Quality(
                            "topology ambiguous at checking precision",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
fn intersects(a: Segment, b: Segment) -> bool {
    if !Bounds::segment(a).overlaps(Bounds::segment(b)) {
        return false;
    }
    let ab = a.b - a.a;
    let cd = b.b - b.a;
    let side1 = ab.cross(b.a - a.a);
    let side2 = ab.cross(b.b - a.a);
    let side3 = cd.cross(a.a - b.a);
    let side4 = cd.cross(a.b - b.a);
    side1 * side2 <= 0.0 && side3 * side4 <= 0.0
}
fn segments(rings: &[Vec<Point>]) -> Vec<Segment> {
    rings
        .iter()
        .enumerate()
        .flat_map(|(ring, points)| {
            points.iter().enumerate().map(move |(index, &a)| Segment {
                a,
                b: points[(index + 1) % points.len()],
                ring,
                index,
                count: points.len(),
            })
        })
        .collect()
}
fn flatten_paths(
    paths: &[VectorPath],
    epsilon: f64,
    limit: usize,
) -> Result<Vec<Vec<Point>>, FitError> {
    let mut used = 0;
    let mut rings = Vec::new();
    for path in paths {
        if path.segments.is_empty() {
            return Err(FitError::Quality("empty path"));
        }
        let mut ring = vec![path.segments[0].start()];
        for (i, &curve) in path.segments.iter().enumerate() {
            if !curve.controls().iter().all(|p| p.is_finite()) {
                return Err(FitError::Quality("nonfinite curve"));
            }
            if curve.start() != *ring.last().unwrap() {
                return Err(FitError::Quality("disconnected path"));
            }
            let points = curve.flatten(epsilon, limit.saturating_sub(used))?;
            used += points.len() - 1;
            if used > limit {
                return Err(FitError::ResourceLimit("flattened path points"));
            }
            ring.extend(points.into_iter().skip(1));
            if i + 1 == path.segments.len() && curve.end() != ring[0] {
                return Err(FitError::Quality("open path"));
            }
        }
        ring.pop();
        if ring.len() < 3 || area(&ring).abs() < 1e-12 {
            return Err(FitError::Quality("zero-area path"));
        }
        if ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
            .any(|(a, b)| a.distance(*b) < 1e-12)
        {
            return Err(FitError::Quality("zero-length path edge"));
        }
        rings.push(ring);
    }
    Ok(rings)
}
fn check_topology(
    paths: &[VectorPath],
    rings: &[Vec<Point>],
    epsilon: f64,
    limit: usize,
) -> Result<(), FitError> {
    let all = segments(rings);
    let tree = Node::build(all.clone());
    let mut work = 0;
    for s in all {
        tree.topology(s, 2.0 * epsilon, &mut work, limit)?;
    }
    let bounds: Vec<_> = rings.iter().map(|ring| polygon_bounds(ring)).collect();
    let areas: Vec<_> = rings.iter().map(|ring| area(ring).abs()).collect();
    for (i, ring) in rings.iter().enumerate() {
        let mut parent: Option<usize> = None;
        for j in 0..rings.len() {
            if j == i {
                continue;
            }
            work = work.saturating_add(1);
            let candidate = bounds_contain(bounds[j], ring[0]);
            if candidate {
                work = work.saturating_add(rings[j].len());
            }
            if work > limit {
                return Err(FitError::ResourceLimit("topology containment comparisons"));
            }
            if candidate
                && inside(ring[0], &rings[j])
                && parent.is_none_or(|current| areas[j] < areas[current])
            {
                parent = Some(j);
            }
        }
        if parent != paths[i].parent {
            return Err(FitError::Quality("contour containment changed"));
        }
        if (area(ring) > 0.0) != (paths[i].depth.is_multiple_of(2)) {
            return Err(FitError::Quality("contour winding changed"));
        }
    }
    Ok(())
}
fn samples(rings: &[Vec<Point>], spacing: f64, limit: usize) -> Result<Vec<Point>, FitError> {
    let mut result = Vec::new();
    for ring in rings {
        result.extend(crate::contour::resample(
            ring,
            spacing,
            limit.saturating_sub(result.len()),
        )?);
    }
    Ok(result)
}

/// Validate rounded/exported curves against the original reference polylines.
/// The bound covers continuous geometry via subdivision and sample coverage;
/// topology is checked conservatively at the reported numerical resolution.
pub fn validate_paths(
    reference: &[Vec<Point>],
    paths: &[VectorPath],
    options: &FitOptions,
) -> Result<QualityReport, FitError> {
    if reference.is_empty()
        || reference.len() != paths.len()
        || reference
            .iter()
            .any(|p| p.len() < 3 || p.iter().any(|p| !p.is_finite()))
    {
        return Err(FitError::Quality("invalid reference contours"));
    }
    if !options.tolerance.is_finite() || !(0.001..=64.0).contains(&options.tolerance) {
        return Err(FitError::InvalidOptions("invalid tolerance"));
    }
    let count: usize = paths.iter().map(|p| p.segments.len()).sum();
    if count > options.limits.max_segments {
        return Err(FitError::ResourceLimit("curve segments"));
    }
    let mut epsilon = options.tolerance / 32.0;
    let mut output = None;
    for _ in 0..7 {
        let rings = flatten_paths(paths, epsilon, options.limits.max_validation_samples)?;
        match check_topology(
            paths,
            &rings,
            epsilon,
            options.limits.max_topology_comparisons,
        ) {
            Ok(()) => {
                output = Some(rings);
                break;
            }
            Err(FitError::Quality("topology ambiguous at checking precision")) => epsilon /= 4.0,
            Err(error) => return Err(error),
        }
    }
    let output = output.ok_or(FitError::Quality(
        "topology could not be certified at checking precision",
    ))?;
    let spacing = options.tolerance / 8.0;
    let source_tree = Node::build(segments(reference));
    let output_tree = Node::build(segments(&output));
    let source_samples = samples(reference, spacing, options.limits.max_validation_samples)?;
    let output_samples = samples(&output, spacing, options.limits.max_validation_samples)?;
    let mut distances: Vec<_> = source_samples
        .iter()
        .map(|&p| output_tree.distance(p))
        .collect();
    let rms = (distances.iter().map(|d| d * d).sum::<f64>() / distances.len() as f64).sqrt();
    distances.sort_by(f64::total_cmp);
    let p95 = distances[((distances.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
    let maximum = output_samples
        .iter()
        .map(|&p| source_tree.distance(p))
        .fold(*distances.last().unwrap(), f64::max);
    let upper = maximum + spacing / 2.0 + epsilon + 1e-8;
    if upper > options.tolerance {
        return Err(FitError::Quality(
            "maximum geometric error exceeds tolerance",
        ));
    }
    Ok(QualityReport {
        contours: paths.len(),
        segments: count,
        rms_error_px: rms,
        p95_error_px: p95,
        sampled_hausdorff_px: maximum,
        hausdorff_upper_bound_px: upper,
        flattening_error_px: epsilon,
        topology_resolution_px: epsilon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bvh_matches_exhaustive_segment_distance() {
        let ring: Vec<_> = (0..32)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 32.0;
                let radius = if i % 2 == 0 { 8.0 } else { 5.0 };
                Point::new(7.0 + radius * angle.cos(), 5.0 + radius * angle.sin())
            })
            .collect();
        let segments = segments(&[ring]);
        let tree = Node::build(segments.clone());
        for x in -2..14 {
            for y in -2..12 {
                let p = Point::new(x as f64 + 0.31, y as f64 + 0.17);
                let expected = segments
                    .iter()
                    .map(|s| distance_to_segment(p, s.a, s.b))
                    .fold(f64::INFINITY, f64::min);
                assert!((tree.distance(p) - expected).abs() < 1e-10);
            }
        }
    }
}
