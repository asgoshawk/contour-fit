use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "contour-fit-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn png(&self, name: &str, rgba: bool) -> PathBuf {
        let path = self.0.join(name);
        let file = fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(file, 32, 32);
        encoder.set_color(if rgba {
            png::ColorType::Rgba
        } else {
            png::ColorType::Grayscale
        });
        encoder.set_depth(png::BitDepth::Eight);
        let mut data = Vec::new();
        for y in 0..32 {
            for x in 0..32 {
                let filled = (5..27).contains(&x)
                    && (5..27).contains(&y)
                    && !((12..20).contains(&x) && (12..20).contains(&y));
                if rgba {
                    data.extend_from_slice(&[0, 0, 0, if filled { 255 } else { 0 }]);
                } else {
                    data.push(if filled { 0 } else { 255 });
                }
            }
        }
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&data)
            .unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_contour-fit"))
}

#[test]
fn converts_png_to_svg_and_report_with_holes() {
    for rgba in [false, true] {
        let dir = Scratch::new();
        let input = dir.png("input.png", rgba);
        let output = dir.0.join("output.svg");
        let report = dir.0.join("metrics.json");
        let run = command()
            .arg(input)
            .arg("-o")
            .arg(&output)
            .arg("--report")
            .arg(&report)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let svg = fs::read_to_string(output).unwrap();
        assert!(svg.contains("fill-rule=\"evenodd\""));
        assert!(svg.contains("currentColor"));
        assert_eq!(svg.matches("<path ").count(), 1);
        assert_eq!(svg.matches(" Z").count(), 2);
        let json: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["quality"]["contours"], 2);
        assert!(
            json["quality"]["hausdorff_upper_bound_px"]
                .as_f64()
                .unwrap()
                <= 1.0
        );
    }
}

#[test]
fn rejects_overwrites_and_colliding_destinations() {
    let dir = Scratch::new();
    let input = dir.png("input.png", false);
    let out = dir.0.join("out.svg");
    fs::write(&out, "keep me").unwrap();
    let run = command().arg(&input).arg("-o").arg(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(4));
    assert_eq!(fs::read_to_string(&out).unwrap(), "keep me");
    let collision = dir.0.join("collision.svg");
    let run = command()
        .arg(&input)
        .arg("-o")
        .arg(&collision)
        .arg("--report")
        .arg(&collision)
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(2));
    assert!(!collision.exists());
}

#[test]
fn diagnostics_are_real_pngs_and_iou_is_recorded() {
    let dir = Scratch::new();
    let input = dir.png("input.png", true);
    let debug = dir.0.join("debug");
    let report = dir.0.join("metrics.json");
    let run = command()
        .arg(input)
        .arg("-o")
        .arg(dir.0.join("out.svg"))
        .arg("--debug-dir")
        .arg(&debug)
        .arg("--report")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    for name in ["preview.png", "overlay.png"] {
        assert_eq!(
            &fs::read(debug.join(name)).unwrap()[..8],
            b"\x89PNG\r\n\x1a\n"
        );
    }
    let json: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert!(json["raster"]["iou"].as_f64().unwrap() > 0.98);
}

#[test]
fn corrupted_input_invalid_options_and_pixel_limits_have_distinct_codes() {
    let dir = Scratch::new();
    let input = dir.png("input.png", false);
    for args in [
        ["--tolerance", "0"],
        ["--threshold", "nan"],
        ["--channel", "nope"],
    ] {
        let run = command()
            .arg(&input)
            .arg("-o")
            .arg(dir.0.join("out.svg"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(run.status.code(), Some(2));
    }
    let run = command()
        .arg(&input)
        .arg("-o")
        .arg(dir.0.join("out.svg"))
        .args(["--max-pixels", "8"])
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(4));
    let bad = dir.0.join("bad.png");
    fs::write(&bad, b"not a png").unwrap();
    let run = command()
        .arg(&bad)
        .arg("-o")
        .arg(dir.0.join("out.svg"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(2));
}

#[test]
fn accepts_palette_low_bit_depth_and_sixteen_bit_samples() {
    let dir = Scratch::new();
    for (index, color, depth, components) in [
        (0, png::ColorType::Grayscale, png::BitDepth::One, 1),
        (1, png::ColorType::Indexed, png::BitDepth::Eight, 1),
        (2, png::ColorType::Grayscale, png::BitDepth::Sixteen, 1),
        (3, png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen, 2),
        (4, png::ColorType::Rgb, png::BitDepth::Eight, 3),
    ] {
        let path = dir.0.join(format!("input-{index}.png"));
        let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 16, 16);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if color == png::ColorType::Indexed {
            encoder.set_palette(vec![255, 255, 255, 0, 0, 0]);
        }
        let mut data = Vec::new();
        for y in 0..16 {
            if depth == png::BitDepth::One {
                data.extend_from_slice(if (4..12).contains(&y) {
                    &[0xf0, 0x0f]
                } else {
                    &[255, 255]
                });
                continue;
            }
            for x in 0..16 {
                let filled = (4..12).contains(&x) && (4..12).contains(&y);
                if color == png::ColorType::Indexed {
                    data.push(u8::from(filled));
                    continue;
                }
                if components == 2 {
                    data.extend_from_slice(&0_u16.to_be_bytes());
                    data.extend_from_slice(&(if filled { 65535_u16 } else { 0 }).to_be_bytes());
                } else {
                    for _ in 0..components {
                        if depth == png::BitDepth::Sixteen {
                            data.extend_from_slice(
                                &(if filled { 0_u16 } else { 65535 }).to_be_bytes(),
                            );
                        } else {
                            data.push(if filled { 0 } else { 255 });
                        }
                    }
                }
            }
        }
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&data)
            .unwrap();
        let run = command()
            .arg(&path)
            .arg("-o")
            .arg(dir.0.join(format!("output-{index}.svg")))
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "format {index}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}

#[test]
fn rejects_animated_png_and_missing_explicit_alpha() {
    let dir = Scratch::new();
    let input = dir.png("opaque.png", false);
    let run = command()
        .arg(input)
        .arg("-o")
        .arg(dir.0.join("opaque.svg"))
        .args(["--channel", "alpha"])
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(2));
    let animated = dir.0.join("animated.png");
    let mut encoder = png::Encoder::new(fs::File::create(&animated).unwrap(), 16, 16);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(2, 0).unwrap();
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[0; 256]).unwrap();
    writer.write_image_data(&[255; 256]).unwrap();
    writer.finish().unwrap();
    let run = command()
        .arg(animated)
        .arg("-o")
        .arg(dir.0.join("animated.svg"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(2));
    assert!(!dir.0.join("animated.svg").exists());
}

#[test]
fn quality_failure_leaves_no_output_or_debug_directory() {
    let dir = Scratch::new();
    let input = dir.0.join("near-threshold.png");
    let mut encoder = png::Encoder::new(fs::File::create(&input).unwrap(), 8, 8);
    encoder.set_color(png::ColorType::GrayscaleAlpha);
    encoder.set_depth(png::BitDepth::Sixteen);
    let mut data = Vec::new();
    for i in 0..64 {
        data.extend_from_slice(&0_u16.to_be_bytes());
        data.extend_from_slice(&(if i == 27 { 32768_u16 } else { 0 }).to_be_bytes());
    }
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&data)
        .unwrap();
    let output = dir.0.join("out.svg");
    let debug = dir.0.join("debug");
    let run = command()
        .arg(input)
        .arg("-o")
        .arg(&output)
        .arg("--debug-dir")
        .arg(&debug)
        .args(["--threshold", "0.50000762"])
        .output()
        .unwrap();
    assert_eq!(
        run.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!output.exists());
    assert!(!debug.exists());
}

#[test]
fn failed_destination_reservation_rolls_back_only_new_files() {
    let dir = Scratch::new();
    let input = dir.png("input.png", false);
    let output = dir.0.join("out.svg");
    let debug = dir.0.join("debug");
    let report = dir.0.join("missing/metrics.json");
    let original = fs::read(&input).unwrap();
    let run = command()
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("--report")
        .arg(report)
        .arg("--debug-dir")
        .arg(&debug)
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(4));
    assert!(!output.exists());
    assert!(!debug.exists());
    assert_eq!(fs::read(input).unwrap(), original);
}
