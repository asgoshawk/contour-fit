use contour_fit_core::{
    Curve, FitOptions, Point, RasterField, VectorPath, validate_paths, vectorize,
};
use proptest::prelude::*;

fn ellipse(size: usize) -> RasterField {
    let mid = size as f64 / 2.0;
    RasterField::new(
        size,
        size,
        (0..size)
            .flat_map(|y| {
                (0..size).map(move |x| {
                    let dx = (x as f64 + 0.5 - mid) / 0.85;
                    let dy = y as f64 + 0.5 - mid;
                    (mid * 0.65 - dx.hypot(dy) + 0.5).clamp(0.0, 1.0) as f32
                })
            })
            .collect(),
    )
    .unwrap()
}
proptest! {
    #![proptest_config(ProptestConfig { cases: 32, rng_seed: proptest::test_runner::RngSeed::Fixed(0xc07f17), ..ProptestConfig::default() })]
    #[test]
    fn rectangles_translate_without_shape_drift(dx in 0_usize..12,dy in 0_usize..12,w in 5_usize..20,h in 5_usize..20) {
        let make=|ox:usize,oy:usize| RasterField::new(48,48,(0..48).flat_map(|y|(0..48).map(move |x| if (ox..ox+w).contains(&x)&&(oy..oy+h).contains(&y) {1.0} else {0.0})).collect()).unwrap();
        let a=vectorize(&make(2,2),&FitOptions::default()).unwrap();
        let b=vectorize(&make(2+dx,2+dy),&FitOptions::default()).unwrap();
        prop_assert_eq!(a.paths[0].segments.len(),b.paths[0].segments.len());
        for (a,b) in a.paths[0].segments.iter().zip(&b.paths[0].segments) {
            for (a,b) in a.controls().iter().zip(b.controls()) {
                prop_assert!((b.x-a.x-dx as f64).abs()<1e-7);
                prop_assert!((b.y-a.y-dy as f64).abs()<1e-7);
            }
        }
    }
}
#[test]
fn smooth_antialiased_ellipse_has_g1_joins_and_bounded_error() {
    let field = ellipse(128);
    let result = vectorize(&field, &FitOptions::default()).unwrap();
    assert!(result.report.hausdorff_upper_bound_px <= 1.0);
    assert!(result.report.segments < 24);
    let curves = &result.paths[0].segments;
    for (a, b) in curves
        .iter()
        .zip(curves.iter().cycle().skip(1))
        .take(curves.len())
    {
        let [_, _, c, d] = a.controls();
        let [e, f, _, _] = b.controls();
        assert_eq!(d, e);
        let outgoing = (d - c).normalized();
        let incoming = (f - e).normalized();
        assert!(outgoing.dot(incoming) > 1.0 - 1e-8);
    }
}
#[test]
fn stricter_tolerance_is_checked_independently() {
    let field = ellipse(64);
    for tolerance in [0.1, 0.25, 0.5, 1.0, 2.0] {
        let options = FitOptions {
            tolerance,
            ..FitOptions::default()
        };
        let result = vectorize(&field, &options).unwrap();
        assert!(
            validate_paths(&result.reference_contours, &result.paths, &options)
                .unwrap()
                .hausdorff_upper_bound_px
                <= tolerance
        );
    }
}
#[test]
fn independent_validator_rejects_crossings_and_changed_containment() {
    let reference = vec![vec![
        Point::new(0.0, 0.0),
        Point::new(8.0, 0.0),
        Point::new(8.0, 8.0),
        Point::new(0.0, 8.0),
    ]];
    let points = [
        Point::new(0.0, 0.0),
        Point::new(8.0, 8.0),
        Point::new(0.0, 8.0),
        Point::new(6.0, 0.0),
    ];
    let path = VectorPath {
        segments: (0..4)
            .map(|i| Curve::Line {
                start: points[i],
                end: points[(i + 1) % 4],
            })
            .collect(),
        parent: None,
        depth: 0,
    };
    assert!(matches!(
        validate_paths(&reference, &[path], &FitOptions::default()),
        Err(contour_fit_core::FitError::Quality(
            "intersecting or touching contours"
        ))
    ));
    let mut result = vectorize(&ellipse(64), &FitOptions::default()).unwrap();
    result.paths[0].parent = Some(0);
    assert!(
        validate_paths(
            &result.reference_contours,
            &result.paths,
            &FitOptions::default()
        )
        .is_err()
    );
}

#[test]
fn pointed_silhouette_keeps_the_reference_apex_as_an_anchor() {
    let field = RasterField::new(
        64,
        48,
        (0..48)
            .flat_map(|y| {
                (0..64).map(move |x| {
                    let half = (y as isize - 24).unsigned_abs();
                    if (8..48).contains(&x) && half < (48 - x) / 2 {
                        1.0
                    } else {
                        0.0
                    }
                })
            })
            .collect(),
    )
    .unwrap();
    let result = vectorize(&field, &FitOptions::default()).unwrap();
    let apex = *result.reference_contours[0]
        .iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    assert!(
        result.paths[0]
            .segments
            .iter()
            .any(|c| c.start().distance(apex) < 1e-9),
        "source apex {apex:?} must remain an exact protected anchor"
    );
}

#[test]
fn geometry_validation_scales_with_pixel_units() {
    let original = vectorize(&ellipse(64), &FitOptions::default()).unwrap();
    for scale in [0.5, 2.0, 4.0] {
        let reference: Vec<Vec<_>> = original
            .reference_contours
            .iter()
            .map(|ring| ring.iter().map(|&p| p * scale).collect())
            .collect();
        let paths: Vec<_> = original
            .paths
            .iter()
            .map(|path| VectorPath {
                parent: path.parent,
                depth: path.depth,
                segments: path
                    .segments
                    .iter()
                    .map(|&c| match c {
                        Curve::Line { start, end } => Curve::Line {
                            start: start * scale,
                            end: end * scale,
                        },
                        Curve::Cubic {
                            start,
                            control1,
                            control2,
                            end,
                        } => Curve::Cubic {
                            start: start * scale,
                            control1: control1 * scale,
                            control2: control2 * scale,
                            end: end * scale,
                        },
                    })
                    .collect(),
            })
            .collect();
        let options = FitOptions {
            tolerance: scale,
            ..FitOptions::default()
        };
        let report = validate_paths(&reference, &paths, &options).unwrap();
        assert!((report.rms_error_px - original.report.rms_error_px * scale).abs() < 1e-7);
        assert!(report.hausdorff_upper_bound_px <= scale);
    }
}

#[test]
fn validated_merging_spends_available_tolerance_to_reduce_curves() {
    let input = ellipse(128);
    let unmerged = vectorize(
        &input,
        &FitOptions {
            merge: false,
            ..FitOptions::default()
        },
    )
    .unwrap();
    let merged = vectorize(&input, &FitOptions::default()).unwrap();
    assert!(
        merged.report.segments < unmerged.report.segments,
        "validated merging should reduce this smooth outline without reusing the stricter initial-fit threshold"
    );
    assert!(merged.report.hausdorff_upper_bound_px <= 1.0);
}
