#![forbid(unsafe_code)]
mod diagnostics;
mod error;
mod image;
mod output;
mod report;
mod svg;

use clap::Parser;
use contour_fit_core::{FitOptions, ResourceLimits, vectorize};
use error::Error;
use image::Channel;
use std::{path::PathBuf, process::ExitCode, time::Instant};

#[derive(Parser)]
#[command(version,about="Fit a silhouette PNG to an error-controlled SVG",long_about=None)]
struct Args {
    /// Input silhouette PNG; photos and animated PNG are not supported.
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    /// Maximum bidirectional contour deviation in source pixels (0.001..=64).
    #[arg(long,default_value_t=1.0,value_parser=parse_tolerance)]
    tolerance: f64,
    /// Foreground level in the selected channel (strictly between 0 and 1).
    #[arg(long,default_value_t=0.5,value_parser=parse_threshold)]
    threshold: f64,
    #[arg(long,value_enum,default_value_t=Channel::Auto)]
    channel: Channel,
    #[arg(long)]
    invert: bool,
    #[arg(long,default_value_t=60.0,value_parser=parse_angle)]
    corner_angle: f64,
    /// Skip optional adjacent curve merging.
    #[arg(long)]
    no_merge: bool,
    #[arg(long)]
    report: Option<PathBuf>,
    /// Write preview.png and overlay.png; enables raster IoU reporting.
    #[arg(long)]
    debug_dir: Option<PathBuf>,
    #[arg(long,default_value_t=16_777_216,value_parser=clap::value_parser!(u32).range(1..=16_777_216))]
    max_pixels: u32,
}
fn finite_range(text: &str, min: f64, max: f64, exclusive: bool) -> Result<f64, String> {
    let value = text
        .parse::<f64>()
        .map_err(|_| "expected a number".to_string())?;
    if !value.is_finite()
        || value < min
        || value > max
        || (exclusive && (value == min || value == max))
    {
        return Err(format!(
            "value must be {} {min} and {max}",
            if exclusive {
                "strictly between"
            } else {
                "between"
            }
        ));
    }
    Ok(value)
}
fn parse_tolerance(text: &str) -> Result<f64, String> {
    finite_range(text, 0.001, 64.0, false)
}
fn parse_threshold(text: &str) -> Result<f64, String> {
    finite_range(text, 0.0, 1.0, true)
}
fn parse_angle(text: &str) -> Result<f64, String> {
    finite_range(text, 1.0, 179.0, false)
}
fn elapsed(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn run(args: Args) -> Result<(), Error> {
    let mut destinations = vec![args.output.clone()];
    if let Some(path) = &args.report {
        destinations.push(path.clone());
    }
    if let Some(dir) = &args.debug_dir {
        destinations.extend([dir.join("preview.png"), dir.join("overlay.png")]);
    }
    output::check(&args.input, &destinations)?;
    let start = Instant::now();
    let decoded = image::load(
        &args.input,
        args.channel,
        args.invert,
        args.max_pixels as usize,
    )?;
    let decode_ms = elapsed(start);
    let options = FitOptions {
        threshold: args.threshold,
        tolerance: args.tolerance,
        corner_angle: args.corner_angle,
        merge: !args.no_merge,
        limits: ResourceLimits {
            max_pixels: args.max_pixels as usize,
            ..ResourceLimits::default()
        },
    };
    let start = Instant::now();
    let result = vectorize(&decoded.field, &options)?;
    let fit_ms = elapsed(start);
    let start = Instant::now();
    let exported = svg::export(&result, &options)?;
    let export_ms = elapsed(start);
    let start = Instant::now();
    let diagnostics = if args.debug_dir.is_some() {
        Some(diagnostics::render(
            &exported.svg,
            &decoded.field,
            &result,
            options.threshold,
        )?)
    } else {
        None
    };
    let diagnostic_ms = elapsed(start);
    let mut files = vec![(args.output, exported.svg.into_bytes())];
    let raster = if let (Some(dir), Some(diagnostics)) = (args.debug_dir.as_ref(), diagnostics) {
        files.push((dir.join("preview.png"), diagnostics.preview));
        files.push((dir.join("overlay.png"), diagnostics.overlay));
        Some(diagnostics.metrics)
    } else {
        None
    };
    let summary = format!(
        "{} contours, {} segments; max error bound {:.4} px (limit {:.4} px)",
        exported.report.contours,
        exported.report.segments,
        exported.report.hausdorff_upper_bound_px,
        options.tolerance
    );
    if let Some(path) = args.report {
        let report = report::Report {
            schema_version: 1,
            version: env!("CARGO_PKG_VERSION"),
            input: report::Input {
                width: result.width,
                height: result.height,
                channel: decoded.channel.label(),
                inverted: args.invert,
            },
            options: report::Options::new(&options, exported.precision),
            quality: exported.report.into(),
            raster,
            timings_ms: report::Timings {
                decode: decode_ms,
                vectorize: fit_ms,
                export_validation: export_ms,
                diagnostics: diagnostic_ms,
            },
        };
        let mut bytes = serde_json::to_vec_pretty(&report)?;
        bytes.push(b'\n');
        files.push((path, bytes));
    }
    // Only create the optional diagnostic directory after all computation succeeds.
    let created = if let Some(dir) = args.debug_dir.as_ref() {
        if !dir.exists() {
            std::fs::create_dir(dir)?;
            true
        } else {
            false
        }
    } else {
        false
    };
    let write = output::write_all(files);
    if write.is_err()
        && created
        && let Some(dir) = args.debug_dir
    {
        let _ = std::fs::remove_dir(dir);
    }
    write?;
    eprintln!("{summary}");
    Ok(())
}
fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("contour-fit: {error}");
            ExitCode::from(error.code())
        }
    }
}
