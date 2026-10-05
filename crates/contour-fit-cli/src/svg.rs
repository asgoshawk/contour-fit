use contour_fit_core::{
    Curve, FitError, FitOptions, QualityReport, VectorPath, Vectorization, validate_paths,
};
use std::fmt::Write;

pub(crate) struct Exported {
    pub svg: String,
    pub report: QualityReport,
    pub precision: u32,
}
pub(crate) fn export(result: &Vectorization, options: &FitOptions) -> Result<Exported, FitError> {
    for precision in 3..=9 {
        let paths: Vec<_> = result
            .paths
            .iter()
            .map(|path| VectorPath {
                segments: path.segments.iter().map(|c| c.rounded(precision)).collect(),
                parent: path.parent,
                depth: path.depth,
            })
            .collect();
        match validate_paths(&result.reference_contours, &paths, options) {
            Ok(report) => {
                return Ok(Exported {
                    svg: write(result.width, result.height, &paths, precision),
                    report,
                    precision,
                });
            }
            Err(FitError::Quality(_)) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(FitError::Quality(
        "serialized SVG cannot meet quality constraints",
    ))
}
fn write(width: usize, height: usize, paths: &[VectorPath], precision: u32) -> String {
    let mut d = String::new();
    let precision = precision as usize;
    for path in paths {
        let first = path.segments[0].start();
        write!(d, "M {:.precision$} {:.precision$}", first.x, first.y).unwrap();
        for &curve in &path.segments {
            match curve {
            Curve::Line{end,..}=>write!(d," L {:.precision$} {:.precision$}",end.x,end.y).unwrap(),
            Curve::Cubic{control1,control2,end,..}=>write!(d," C {:.precision$} {:.precision$} {:.precision$} {:.precision$} {:.precision$} {:.precision$}",control1.x,control1.y,control2.x,control2.y,end.x,end.y).unwrap(),
        }
        }
        d.push_str(" Z ");
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\">\n  <path fill=\"currentColor\" fill-rule=\"evenodd\" d=\"{}\"/>\n</svg>\n",
        d.trim_end()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use contour_fit_core::{Point, RasterField, vectorize};
    #[test]
    fn parsed_svg_uses_the_certified_coordinates() {
        let field = RasterField::new(
            16,
            16,
            (0..256)
                .map(|i| {
                    if (3..13).contains(&(i % 16)) && (3..13).contains(&(i / 16)) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect(),
        )
        .unwrap();
        let result = vectorize(&field, &FitOptions::default()).unwrap();
        let exported = export(&result, &FitOptions::default()).unwrap();
        let tree =
            resvg::usvg::Tree::from_str(&exported.svg, &resvg::usvg::Options::default()).unwrap();
        assert_eq!(tree.size().width(), 16.0);
        assert!(exported.report.hausdorff_upper_bound_px <= 1.0);
        assert!(
            result.paths[0].segments[0]
                .start()
                .distance(Point::new(3.5, 3.0))
                < 1.0
        );
    }
}
