use contour_fit_core::{FitOptions, RasterField, vectorize};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn fixture(size: usize, complex: bool) -> RasterField {
    let mid = size as f64 / 2.0;
    let values = (0..size)
        .flat_map(|y| {
            (0..size).map(move |x| {
                let dx = x as f64 + 0.5 - mid;
                let dy = y as f64 + 0.5 - mid;
                let angle = dy.atan2(dx);
                let radius = if complex {
                    mid * (0.65 + 0.08 * (angle * 9.0).cos())
                } else {
                    mid * 0.7
                };
                // Analytic antialiasing reference; test data is generated, not downloaded.
                (radius - dx.hypot(dy) + 0.5).clamp(0.0, 1.0) as f32
            })
        })
        .collect();
    RasterField::new(size, size, values).unwrap()
}
fn pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectorize");
    group.sample_size(10);
    for size in [512, 1024, 2048] {
        for complex in [false, true] {
            let field = fixture(size, complex);
            group.bench_with_input(
                BenchmarkId::new(if complex { "scalloped" } else { "circle" }, size),
                &field,
                |b, field| {
                    b.iter(|| {
                        std::hint::black_box(
                            vectorize(std::hint::black_box(field), &FitOptions::default()).unwrap(),
                        )
                    });
                },
            );
        }
    }
    group.finish();
}
criterion_group!(benches, pipeline);
criterion_main!(benches);
