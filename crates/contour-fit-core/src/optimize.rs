use crate::Point;
use crate::fit::{Piece, Prepared, fit_one};
use crate::metrics::validate_paths;
use crate::{FitOptions, VectorPath};

/// Deterministic adjacent merge. Every accepted change passes the independent
/// global geometric and topology validator; corners and the seam stay protected.
pub(crate) fn merge(
    prepared: &mut [Prepared],
    paths: &mut [VectorPath],
    reference: &[Vec<Point>],
    options: &FitOptions,
    canvas: Point,
) {
    if !options.merge {
        return;
    }
    let mut attempts = 0;
    for ring in 0..prepared.len() {
        let mut i = 0;
        while i + 1 < prepared[ring].pieces.len() && attempts < 128 {
            let first = &prepared[ring].pieces[i];
            let second = &prepared[ring].pieces[i + 1];
            let joint = first.end % prepared[ring].corners.len();
            if prepared[ring].corners[joint] {
                i += 1;
                continue;
            }
            attempts += 1;
            let [a, b, _, _] = first.curve.controls();
            let [_, _, c, d] = second.curve.controls();
            let (curve, error, _) = fit_one(
                &prepared[ring].points[first.start..=second.end],
                (b - a).normalized(),
                (c - d).normalized(),
                canvas,
            );
            // Initial fitting reserves validation headroom. Merging can spend
            // the full budget because every candidate is independently checked.
            if error > options.tolerance {
                i += 1;
                continue;
            }
            let candidate = Piece {
                curve,
                start: first.start,
                end: second.end,
            };
            let previous = paths[ring].segments.clone();
            paths[ring].segments.splice(i..=i + 1, [curve]);
            if validate_paths(reference, paths, options).is_ok() {
                prepared[ring].pieces.splice(i..=i + 1, [candidate]);
                i = i.saturating_sub(1);
            } else {
                paths[ring].segments = previous;
                i += 1;
            }
        }
    }
}
