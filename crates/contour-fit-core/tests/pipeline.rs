use contour_fit_core::{FitError, FitOptions, RasterField, vectorize};

fn field(width: usize, height: usize, filled: impl Fn(usize, usize) -> bool) -> RasterField {
    let values = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| if filled(x, y) { 1.0 } else { 0.0 })
        .collect();
    RasterField::new(width, height, values).unwrap()
}

#[test]
fn rectangle_is_closed_and_certified() {
    let input = field(32, 32, |x, y| (4..28).contains(&x) && (6..26).contains(&y));
    let result = vectorize(&input, &FitOptions::default()).unwrap();
    assert_eq!(result.paths.len(), 1);
    assert_eq!(result.paths[0].depth, 0);
    assert!(result.report.hausdorff_upper_bound_px <= 1.0);
    assert!(result.report.segments < 24);
    assert_eq!(
        result.paths[0].segments[0].start(),
        result.paths[0].segments.last().unwrap().end()
    );
}

#[test]
fn preserves_nested_holes_and_separate_components() {
    let input = field(64, 48, |x, y| {
        let outer = (2..40).contains(&x) && (2..44).contains(&y);
        let hole = (8..34).contains(&x) && (8..38).contains(&y);
        let island = (16..26).contains(&x) && (16..30).contains(&y);
        let separate = (48..60).contains(&x) && (12..36).contains(&y);
        (outer && !hole) || island || separate
    });
    let result = vectorize(&input, &FitOptions::default()).unwrap();
    assert_eq!(result.paths.len(), 4);
    let mut depths: Vec<_> = result.paths.iter().map(|p| p.depth).collect();
    depths.sort();
    assert_eq!(depths, [0, 0, 1, 2]);
}

#[test]
fn empty_input_is_an_explicit_error() {
    let input = field(8, 8, |_, _| false);
    assert!(matches!(
        vectorize(&input, &FitOptions::default()),
        Err(FitError::NoForeground)
    ));
}

#[test]
fn image_border_and_single_pixel_are_supported() {
    for input in [
        field(8, 8, |_, _| true),
        field(8, 8, |x, y| x == 0 && y == 0),
    ] {
        let result = vectorize(&input, &FitOptions::default()).unwrap();
        assert_eq!(result.paths.len(), 1);
        for segment in &result.paths[0].segments {
            assert!(segment.start().x >= -1e-7 && segment.start().y >= -1e-7);
        }
    }
}

#[test]
fn diagonal_foreground_pixels_remain_disconnected() {
    let input = field(8, 8, |x, y| (x == 2 && y == 2) || (x == 3 && y == 3));
    assert_eq!(
        vectorize(&input, &FitOptions::default())
            .unwrap()
            .paths
            .len(),
        2
    );
}

#[test]
fn invalid_inputs_and_limits_fail_without_panicking() {
    assert!(RasterField::new(0, 3, vec![]).is_err());
    assert!(RasterField::new(2, 2, vec![f32::NAN; 4]).is_err());
    let input = field(16, 16, |x, y| x > 3 && y > 3);
    let options = FitOptions {
        tolerance: 0.0,
        ..FitOptions::default()
    };
    assert!(matches!(
        vectorize(&input, &options),
        Err(FitError::InvalidOptions(_))
    ));
    let mut options = FitOptions::default();
    options.limits.max_pixels = 8;
    assert!(matches!(
        vectorize(&input, &options),
        Err(FitError::ResourceLimit(_))
    ));
}

#[test]
fn repeated_conversion_is_deterministic() {
    let input = field(48, 48, |x, y| {
        let dx = x as f64 - 24.0;
        let dy = y as f64 - 24.0;
        dx * dx + dy * dy < 300.0
    });
    let a = vectorize(&input, &FitOptions::default()).unwrap();
    let b = vectorize(&input, &FitOptions::default()).unwrap();
    assert_eq!(a.paths, b.paths);
    assert_eq!(a.report, b.report);
}

#[test]
fn containment_work_has_a_separate_budget() {
    let input = field(32, 32, |x, y| {
        (4..8).contains(&y)
            && ((2..6).contains(&x) || (12..16).contains(&x) || (22..26).contains(&x))
    });
    let mut options = FitOptions::default();
    options.limits.max_topology_comparisons = 1;
    assert!(matches!(
        vectorize(&input, &options),
        Err(FitError::ResourceLimit("contour containment comparisons"))
    ));
}

#[test]
fn cropped_curves_stay_inside_the_preserved_svg_canvas() {
    for center in [(-20.0, 32.0), (-28.0, 32.0), (32.0, -28.0), (72.0, 32.0)] {
        let input = RasterField::new(
            64,
            64,
            (0..64)
                .flat_map(|y| {
                    (0..64).map(move |x| {
                        if (x as f64 + 0.5 - center.0).hypot(y as f64 + 0.5 - center.1) < 36.0 {
                            1.0
                        } else {
                            0.0
                        }
                    })
                })
                .collect(),
        )
        .unwrap();
        let result = vectorize(&input, &FitOptions::default()).unwrap();
        for curve in &result.paths[0].segments {
            for i in 0..101 {
                let p = curve.point(i as f64 / 100.0);
                assert!(
                    p.x >= -1e-8 && p.y >= -1e-8 && p.x <= 64.0 + 1e-8 && p.y <= 64.0 + 1e-8,
                    "curve would be clipped by the canvas: {p:?}"
                );
            }
        }
    }
}
