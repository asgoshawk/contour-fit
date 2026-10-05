use crate::diagnostics::RasterMetrics;
use contour_fit_core::{FitOptions, QualityReport};

#[derive(serde::Serialize)]
pub(crate) struct Report<'a> {
    pub schema_version: u32,
    pub version: &'static str,
    pub input: Input<'a>,
    pub options: Options,
    pub quality: Quality,
    pub raster: Option<RasterMetrics>,
    pub timings_ms: Timings,
}
#[derive(serde::Serialize)]
pub(crate) struct Input<'a> {
    pub width: usize,
    pub height: usize,
    pub channel: &'a str,
    pub inverted: bool,
}
#[derive(serde::Serialize)]
pub(crate) struct Options {
    pub threshold: f64,
    pub tolerance_px: f64,
    pub corner_angle_degrees: f64,
    pub merge: bool,
    pub svg_precision: u32,
}
impl Options {
    pub fn new(options: &FitOptions, precision: u32) -> Self {
        Self {
            threshold: options.threshold,
            tolerance_px: options.tolerance,
            corner_angle_degrees: options.corner_angle,
            merge: options.merge,
            svg_precision: precision,
        }
    }
}
#[derive(serde::Serialize)]
pub(crate) struct Quality {
    contours: usize,
    segments: usize,
    rms_error_px: f64,
    p95_error_px: f64,
    sampled_hausdorff_px: f64,
    hausdorff_upper_bound_px: f64,
    flattening_error_px: f64,
    topology_resolution_px: f64,
    reference: &'static str,
    distribution: &'static str,
    passed: bool,
}
impl From<QualityReport> for Quality {
    fn from(q: QualityReport) -> Self {
        Self {
            contours: q.contours,
            segments: q.segments,
            rms_error_px: q.rms_error_px,
            p95_error_px: q.p95_error_px,
            sampled_hausdorff_px: q.sampled_hausdorff_px,
            hausdorff_upper_bound_px: q.hausdorff_upper_bound_px,
            flattening_error_px: q.flattening_error_px,
            topology_resolution_px: q.topology_resolution_px,
            reference: "marching-squares level-set polylines",
            distribution: "uniform arc-length source-to-output samples",
            passed: true,
        }
    }
}
#[derive(serde::Serialize)]
pub(crate) struct Timings {
    pub decode: f64,
    pub vectorize: f64,
    pub export_validation: f64,
    pub diagnostics: f64,
}
