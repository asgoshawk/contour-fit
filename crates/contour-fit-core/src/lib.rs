#![forbid(unsafe_code)]
//! Deterministic silhouette fitting. Coordinates and tolerances use source pixels.
//!
//! ```
//! use contour_fit_core::{FitOptions, RasterField, vectorize};
//! let field = RasterField::new(2, 2, vec![1.0, 0.0, 0.0, 0.0])?;
//! let result = vectorize(&field, &FitOptions::default())?;
//! assert!(result.report.hausdorff_upper_bound_px <= 1.0);
//! # Ok::<(), contour_fit_core::FitError>(())
//! ```
mod contour;
mod fit;
mod geometry;
mod metrics;
mod optimize;
mod pipeline;

pub use geometry::{Curve, Point, VectorPath};
pub use metrics::{QualityReport, validate_paths};
pub use pipeline::vectorize;

#[derive(Debug, thiserror::Error)]
pub enum FitError {
    #[error("invalid raster: {0}")]
    InvalidRaster(&'static str),
    #[error("invalid options: {0}")]
    InvalidOptions(&'static str),
    #[error("the selected channel contains no foreground")]
    NoForeground,
    #[error("resource limit exceeded: {0}")]
    ResourceLimit(&'static str),
    #[error("quality validation failed: {0}")]
    Quality(&'static str),
}

#[derive(Clone, Debug)]
pub struct RasterField {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

impl RasterField {
    pub fn new(width: usize, height: usize, values: Vec<f32>) -> Result<Self, FitError> {
        if width == 0 || height == 0 || width.checked_mul(height) != Some(values.len()) {
            return Err(FitError::InvalidRaster("dimensions do not match samples"));
        }
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(FitError::InvalidRaster(
                "samples must be finite and in [0, 1]",
            ));
        }
        Ok(Self {
            width,
            height,
            values,
        })
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn values(&self) -> &[f32] {
        &self.values
    }
    pub(crate) fn padded(&self, x: usize, y: usize) -> f64 {
        if x == 0 || y == 0 || x > self.width || y > self.height {
            0.0
        } else {
            f64::from(self.values[(y - 1) * self.width + x - 1])
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceLimits {
    pub max_pixels: usize,
    pub max_contour_points: usize,
    pub max_segments: usize,
    pub max_validation_samples: usize,
    pub max_fit_attempts: usize,
    pub max_topology_comparisons: usize,
}
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_pixels: 16_777_216,
            max_contour_points: 250_000,
            max_segments: 20_000,
            max_validation_samples: 500_000,
            max_fit_attempts: 100_000,
            max_topology_comparisons: 100_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FitOptions {
    pub threshold: f64,
    pub tolerance: f64,
    /// Minimum turning angle in degrees for a protected corner.
    pub corner_angle: f64,
    pub merge: bool,
    pub limits: ResourceLimits,
}
impl Default for FitOptions {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            tolerance: 1.0,
            corner_angle: 60.0,
            merge: true,
            limits: ResourceLimits::default(),
        }
    }
}
impl FitOptions {
    pub(crate) fn check(&self, field: &RasterField) -> Result<(), FitError> {
        if !self.threshold.is_finite() || self.threshold <= 0.0 || self.threshold >= 1.0 {
            return Err(FitError::InvalidOptions("threshold must be in (0, 1)"));
        }
        if !self.tolerance.is_finite() || !(0.001..=64.0).contains(&self.tolerance) {
            return Err(FitError::InvalidOptions(
                "tolerance must be in [0.001, 64] pixels",
            ));
        }
        if !self.corner_angle.is_finite() || !(1.0..=179.0).contains(&self.corner_angle) {
            return Err(FitError::InvalidOptions(
                "corner angle must be in [1, 179] degrees",
            ));
        }
        if field.values.len() > self.limits.max_pixels {
            return Err(FitError::ResourceLimit("pixels"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Vectorization {
    pub width: usize,
    pub height: usize,
    pub paths: Vec<VectorPath>,
    /// Unmodified marching-squares contours; use these to revalidate serialization.
    pub reference_contours: Vec<Vec<Point>>,
    pub report: QualityReport,
}
