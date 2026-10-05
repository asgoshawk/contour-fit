use crate::{error::Error, image::encode_png};
use contour_fit_core::{Point, RasterField, Vectorization};

#[derive(serde::Serialize)]
pub(crate) struct RasterMetrics {
    pub iou: f64,
    pub pixel_disagreement: usize,
    pub reference_threshold: f64,
    pub rendered_alpha_threshold: f64,
    pub resolution: String,
}
pub(crate) struct Diagnostics {
    pub preview: Vec<u8>,
    pub overlay: Vec<u8>,
    pub metrics: RasterMetrics,
}

pub(crate) fn render(
    svg: &str,
    field: &RasterField,
    result: &Vectorization,
    threshold: f64,
) -> Result<Diagnostics, Error> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|e| Error::Input(format!("cannot render generated SVG: {e}")))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(field.width() as u32, field.height() as u32)
        .ok_or_else(|| Error::Resource("cannot allocate diagnostic pixmap".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    // Generated paths use opaque black, so the premultiplied RGB representation
    // is also valid straight-alpha RGBA for PNG serialization.
    let preview = encode_png(field.width(), field.height(), pixmap.data())?;
    let mut intersection = 0;
    let mut union = 0;
    let mut disagreement = 0;
    let mut overlay = Vec::with_capacity(field.width() * field.height() * 4);
    for (&value, pixel) in field.values().iter().zip(pixmap.data().chunks_exact(4)) {
        let source = f64::from(value) >= threshold;
        let target = pixel[3] >= 128;
        intersection += usize::from(source && target);
        union += usize::from(source || target);
        disagreement += usize::from(source != target);
        let grey = if source { 100 } else { 245 };
        overlay.extend_from_slice(&[grey, grey, grey, 255]);
    }
    for ring in &result.reference_contours {
        for (a, b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            draw_line(
                &mut overlay,
                field.width(),
                field.height(),
                *a,
                *b,
                [220, 40, 40, 255],
            );
        }
    }
    for path in &result.paths {
        for curve in &path.segments {
            let length = curve
                .controls()
                .windows(2)
                .map(|p| p[0].distance(p[1]))
                .sum::<f64>();
            let samples = (length * 2.0).ceil().max(2.0) as usize;
            for i in 0..samples {
                draw_line(
                    &mut overlay,
                    field.width(),
                    field.height(),
                    curve.point(i as f64 / samples as f64),
                    curve.point((i + 1) as f64 / samples as f64),
                    [30, 100, 230, 255],
                );
            }
        }
    }
    Ok(Diagnostics {
        preview,
        overlay: encode_png(field.width(), field.height(), &overlay)?,
        metrics: RasterMetrics {
            iou: if union == 0 {
                1.0
            } else {
                intersection as f64 / union as f64
            },
            pixel_disagreement: disagreement,
            reference_threshold: threshold,
            rendered_alpha_threshold: 128.0 / 255.0,
            resolution: format!("{}x{}", field.width(), field.height()),
        },
    })
}
fn draw_line(data: &mut [u8], width: usize, height: usize, a: Point, b: Point, color: [u8; 4]) {
    let count = (a.distance(b) * 2.0).ceil().max(1.0) as usize;
    for i in 0..=count {
        let p = a + (b - a) * (i as f64 / count as f64);
        let x = p.x.floor() as isize;
        let y = p.y.floor() as isize;
        if x >= 0 && y >= 0 && x < width as isize && y < height as isize {
            let offset = (y as usize * width + x as usize) * 4;
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}
