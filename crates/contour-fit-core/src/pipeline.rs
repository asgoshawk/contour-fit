use crate::{FitError, FitOptions, Point, RasterField, VectorPath, Vectorization};
use crate::{contour, fit, metrics, optimize};

pub fn vectorize(field: &RasterField, options: &FitOptions) -> Result<Vectorization, FitError> {
    options.check(field)?;
    let canvas = Point::new(field.width() as f64, field.height() as f64);
    let contours = contour::extract(field, options)?;
    let reference: Vec<_> = contours.iter().map(|c| c.points.clone()).collect();
    let mut work = 0;
    let mut prepared = Vec::new();
    let mut paths = Vec::new();
    let mut total_points = 0;
    for contour in &contours {
        let (points, corners) = contour::resample_anchored(
            &contour.points,
            (options.tolerance * 0.5).min(1.0),
            options
                .limits
                .max_contour_points
                .saturating_sub(total_points),
            &fit::protected_corners(&contour.points, options.corner_angle, canvas),
        )?;
        total_points += points.len();
        let fitted = fit::fit_ring(points, corners, options, &mut work, canvas)?;
        paths.push(VectorPath {
            segments: fitted.pieces.iter().map(|p| p.curve).collect(),
            parent: contour.parent,
            depth: contour.depth,
        });
        prepared.push(fitted);
    }
    // A failed initial validation retries with a denser sample grid and stricter
    // fit target, never silently relaxes the user's tolerance.
    if let Err(error) = metrics::validate_paths(&reference, &paths, options) {
        if !matches!(error, FitError::Quality(_)) {
            return Err(error);
        }
        prepared.clear();
        paths.clear();
        total_points = 0;
        let stricter = FitOptions {
            tolerance: options.tolerance * 0.25,
            merge: false,
            ..options.clone()
        };
        for contour in &contours {
            let (points, corners) = contour::resample_anchored(
                &contour.points,
                (options.tolerance * 0.125).min(0.5),
                options
                    .limits
                    .max_contour_points
                    .saturating_sub(total_points),
                &fit::protected_corners(&contour.points, options.corner_angle, canvas),
            )?;
            total_points += points.len();
            let fitted = fit::fit_ring(points, corners, &stricter, &mut work, canvas)?;
            paths.push(VectorPath {
                segments: fitted.pieces.iter().map(|p| p.curve).collect(),
                parent: contour.parent,
                depth: contour.depth,
            });
            prepared.push(fitted);
        }
        metrics::validate_paths(&reference, &paths, options)?;
    }
    optimize::merge(&mut prepared, &mut paths, &reference, options, canvas);
    let report = metrics::validate_paths(&reference, &paths, options)?;
    Ok(Vectorization {
        width: field.width(),
        height: field.height(),
        paths,
        reference_contours: reference,
        report,
    })
}
