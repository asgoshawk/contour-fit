use crate::geometry::{area, bounds_contain, inside, polygon_bounds};
use crate::{FitError, FitOptions, Point, RasterField};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct Contour {
    pub points: Vec<Point>,
    pub parent: Option<usize>,
    pub depth: usize,
}

// Grid edge identity keeps stitching independent of floating point equality.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Edge {
    x: usize,
    y: usize,
    vertical: bool,
}

pub(crate) fn extract(field: &RasterField, options: &FitOptions) -> Result<Vec<Contour>, FitError> {
    let mut ids = HashMap::new();
    let mut points = Vec::new();
    let mut neighbors: Vec<Vec<usize>> = Vec::new();
    for y in 0..=field.height() {
        for x in 0..=field.width() {
            let values = [
                field.padded(x, y),
                field.padded(x + 1, y),
                field.padded(x + 1, y + 1),
                field.padded(x, y + 1),
            ];
            let mask = values.iter().enumerate().fold(0_u8, |m, (i, v)| {
                m | (((*v >= options.threshold) as u8) << i)
            });
            let pairs: &[(usize, usize)] = match mask {
                0 | 15 => &[],
                1 | 14 => &[(3, 0)],
                2 | 13 => &[(0, 1)],
                3 | 12 => &[(3, 1)],
                4 | 11 => &[(1, 2)],
                5 => &[(3, 0), (1, 2)],
                6 | 9 => &[(0, 2)],
                7 | 8 => &[(3, 2)],
                10 => &[(0, 1), (2, 3)],
                _ => unreachable!(),
            };
            for &(a, b) in pairs {
                let mut vertices = [0; 2];
                for (slot, edge) in vertices.iter_mut().zip([a, b]) {
                    let (key, p0, p1, v0, v1) = match edge {
                        0 => (
                            Edge {
                                x,
                                y,
                                vertical: false,
                            },
                            Point::new(x as f64 - 0.5, y as f64 - 0.5),
                            Point::new(x as f64 + 0.5, y as f64 - 0.5),
                            values[0],
                            values[1],
                        ),
                        1 => (
                            Edge {
                                x: x + 1,
                                y,
                                vertical: true,
                            },
                            Point::new(x as f64 + 0.5, y as f64 - 0.5),
                            Point::new(x as f64 + 0.5, y as f64 + 0.5),
                            values[1],
                            values[2],
                        ),
                        2 => (
                            Edge {
                                x,
                                y: y + 1,
                                vertical: false,
                            },
                            Point::new(x as f64 - 0.5, y as f64 + 0.5),
                            Point::new(x as f64 + 0.5, y as f64 + 0.5),
                            values[3],
                            values[2],
                        ),
                        _ => (
                            Edge {
                                x,
                                y,
                                vertical: true,
                            },
                            Point::new(x as f64 - 0.5, y as f64 - 0.5),
                            Point::new(x as f64 - 0.5, y as f64 + 0.5),
                            values[0],
                            values[3],
                        ),
                    };
                    *slot = *ids.entry(key).or_insert_with(|| {
                        let t = ((options.threshold - v0) / (v1 - v0)).clamp(1e-9, 1.0 - 1e-9);
                        let id = points.len();
                        points.push(p0 + (p1 - p0) * t);
                        neighbors.push(Vec::with_capacity(2));
                        id
                    });
                }
                if points.len() > options.limits.max_contour_points {
                    return Err(FitError::ResourceLimit("contour points"));
                }
                neighbors[vertices[0]].push(vertices[1]);
                neighbors[vertices[1]].push(vertices[0]);
            }
        }
    }
    if points.is_empty() {
        return Err(FitError::NoForeground);
    }
    if neighbors.iter().any(|v| v.len() != 2) {
        return Err(FitError::Quality("contour graph is not closed"));
    }
    let mut seen = vec![false; points.len()];
    let mut rings = Vec::new();
    for start in 0..points.len() {
        if seen[start] {
            continue;
        }
        let mut ring = Vec::new();
        let mut previous = usize::MAX;
        let mut current = start;
        while !seen[current] {
            seen[current] = true;
            ring.push(points[current]);
            let next = if neighbors[current][0] != previous {
                neighbors[current][0]
            } else {
                neighbors[current][1]
            };
            previous = current;
            current = next;
        }
        if current != start || ring.len() < 3 {
            return Err(FitError::Quality("degenerate contour"));
        }
        rings.push(Contour {
            points: ring,
            parent: None,
            depth: 0,
        });
    }
    // Stable area ordering puts containers before their children.
    rings.sort_by(|a, b| {
        area(&b.points)
            .abs()
            .total_cmp(&area(&a.points).abs())
            .then_with(|| a.points[0].y.total_cmp(&b.points[0].y))
            .then_with(|| a.points[0].x.total_cmp(&b.points[0].x))
    });
    let bounds: Vec<_> = rings
        .iter()
        .map(|ring| polygon_bounds(&ring.points))
        .collect();
    let mut containment_work = 0_usize;
    for i in 0..rings.len() {
        let p = rings[i].points[0];
        for j in (0..i).rev() {
            containment_work = containment_work.saturating_add(1);
            let candidate = bounds_contain(bounds[j], p);
            if candidate {
                containment_work = containment_work.saturating_add(rings[j].points.len());
            }
            if containment_work > options.limits.max_topology_comparisons {
                return Err(FitError::ResourceLimit("contour containment comparisons"));
            }
            if candidate && inside(p, &rings[j].points) {
                rings[i].parent = Some(j);
                break;
            }
        }
        rings[i].depth = rings[i].parent.map_or(0, |j| rings[j].depth + 1);
        let positive = area(&rings[i].points) > 0.0;
        if positive != (rings[i].depth.is_multiple_of(2)) {
            rings[i].points.reverse();
        }
        let first = (0..rings[i].points.len())
            .min_by(|&a, &b| {
                rings[i].points[a]
                    .y
                    .total_cmp(&rings[i].points[b].y)
                    .then_with(|| rings[i].points[a].x.total_cmp(&rings[i].points[b].x))
            })
            .unwrap();
        rings[i].points.rotate_left(first);
    }
    Ok(rings)
}

pub(crate) fn resample(
    points: &[Point],
    spacing: f64,
    limit: usize,
) -> Result<Vec<Point>, FitError> {
    let mut distances = vec![0.0];
    for i in 0..points.len() {
        distances.push(distances[i] + points[i].distance(points[(i + 1) % points.len()]));
    }
    let length = *distances.last().unwrap();
    let count = (length / spacing).ceil().max(8.0) as usize;
    if count > limit {
        return Err(FitError::ResourceLimit("resampled contour points"));
    }
    let mut edge = 0;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let s = length * i as f64 / count as f64;
        while edge + 1 < points.len() && distances[edge + 1] < s {
            edge += 1;
        }
        let len = distances[edge + 1] - distances[edge];
        let t = if len > 0.0 {
            (s - distances[edge]) / len
        } else {
            0.0
        };
        out.push(points[edge] + (points[(edge + 1) % points.len()] - points[edge]) * t);
    }
    Ok(out)
}

/// Keep exact source corner coordinates alongside uniformly spaced fit samples.
pub(crate) fn resample_anchored(
    points: &[Point],
    spacing: f64,
    limit: usize,
    protected: &[bool],
) -> Result<(Vec<Point>, Vec<bool>), FitError> {
    let mut distances = vec![0.0];
    for i in 0..points.len() {
        distances.push(distances[i] + points[i].distance(points[(i + 1) % points.len()]));
    }
    let length = *distances.last().unwrap();
    let count = (length / spacing).ceil().max(8.0) as usize;
    let anchors = protected.iter().filter(|&&v| v).count();
    if count.saturating_add(anchors) > limit {
        return Err(FitError::ResourceLimit("anchored contour points"));
    }
    let uniform = resample(points, spacing, limit)?;
    let mut records: Vec<_> = uniform
        .into_iter()
        .enumerate()
        .map(|(i, p)| (length * i as f64 / count as f64, p, false))
        .collect();
    for (i, &corner) in protected.iter().enumerate() {
        if corner {
            records.push((distances[i], points[i], true));
        }
    }
    records.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut result = Vec::new();
    let mut corners = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for (s, p, corner) in records {
        if s - last < 1e-9 {
            if corner {
                *result.last_mut().unwrap() = p;
                *corners.last_mut().unwrap() = true;
            }
        } else {
            result.push(p);
            corners.push(corner);
            last = s;
        }
    }
    Ok((result, corners))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_cell_configuration_stitches() {
        for mask in 1_u8..16 {
            let values = (0..4)
                .map(|i| {
                    if mask & (1 << [0, 1, 3, 2][i]) != 0 {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let field = RasterField::new(2, 2, values).unwrap();
            let contours = extract(&field, &FitOptions::default()).unwrap();
            assert!(!contours.is_empty());
        }
    }
}
