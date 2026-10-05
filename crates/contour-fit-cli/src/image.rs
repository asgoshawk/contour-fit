use crate::error::Error;
use contour_fit_core::RasterField;
use std::{fs::File, io::BufReader, path::Path};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum Channel {
    Auto,
    Alpha,
    Luminance,
}
impl Channel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Alpha => "alpha",
            Self::Luminance => "luminance",
        }
    }
}
pub(crate) struct Decoded {
    pub field: RasterField,
    pub channel: Channel,
}

pub(crate) fn load(
    path: &Path,
    requested: Channel,
    invert: bool,
    max_pixels: usize,
) -> Result<Decoded, Error> {
    let file = File::open(path)?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_limits(png::Limits {
        bytes: 128 * 1024 * 1024,
    });
    // Palette and grayscale samples expand without discarding 16-bit precision.
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::Input(format!("invalid PNG: {e}")))?;
    if reader.info().animation_control.is_some() {
        return Err(Error::Input("animated PNG is not supported".into()));
    }
    let info = reader.info();
    let count = (info.width as usize)
        .checked_mul(info.height as usize)
        .ok_or_else(|| Error::Resource("PNG dimensions overflow".into()))?;
    if count == 0 || count > max_pixels {
        return Err(Error::Resource("PNG exceeds --max-pixels".into()));
    }
    let capacity = reader
        .output_buffer_size()
        .ok_or_else(|| Error::Resource("PNG buffer size overflow".into()))?;
    if capacity > 128 * 1024 * 1024 {
        return Err(Error::Resource("PNG decoded buffer exceeds 128 MiB".into()));
    }
    let mut bytes = vec![0; capacity];
    let output = reader
        .next_frame(&mut bytes)
        .map_err(|e| Error::Input(format!("cannot decode PNG: {e}")))?;
    // next_frame may finish before trailing chunks. Validate IEND and checksums too.
    reader
        .finish()
        .map_err(|e| Error::Input(format!("invalid PNG trailer: {e}")))?;
    let (components, alpha) = match output.color_type {
        png::ColorType::Grayscale => (1, None),
        png::ColorType::GrayscaleAlpha => (2, Some(1)),
        png::ColorType::Rgb => (3, None),
        png::ColorType::Rgba => (4, Some(3)),
        png::ColorType::Indexed => return Err(Error::Input("palette was not expanded".into())),
    };
    let stride = match output.bit_depth {
        png::BitDepth::Eight => 1,
        png::BitDepth::Sixteen => 2,
        _ => return Err(Error::Input("unsupported decoded bit depth".into())),
    };
    let data = &bytes[..output.buffer_size()];
    let sample = |pixel: &[u8], component: usize| -> f32 {
        if stride == 1 {
            f32::from(pixel[component]) / 255.0
        } else {
            f32::from(u16::from_be_bytes([
                pixel[component * 2],
                pixel[component * 2 + 1],
            ])) / 65535.0
        }
    };
    let nonopaque = alpha.is_some_and(|a| {
        data.chunks_exact(components * stride)
            .any(|p| sample(p, a) < 1.0)
    });
    let channel = match requested {
        Channel::Auto => {
            if nonopaque {
                Channel::Alpha
            } else {
                Channel::Luminance
            }
        }
        other => other,
    };
    if matches!(channel, Channel::Alpha) && alpha.is_none() {
        return Err(Error::Input(
            "--channel alpha requires an alpha channel or PNG transparency".into(),
        ));
    }
    let values = data
        .chunks_exact(components * stride)
        .map(|p| {
            let value = match channel {
                Channel::Alpha => sample(p, alpha.unwrap()),
                Channel::Luminance | Channel::Auto => {
                    let luminance = if components <= 2 {
                        sample(p, 0)
                    } else {
                        0.2126 * sample(p, 0) + 0.7152 * sample(p, 1) + 0.0722 * sample(p, 2)
                    };
                    // Transparent pixels are background when luminance is requested.
                    (1.0 - luminance) * alpha.map_or(1.0, |a| sample(p, a))
                }
            };
            if invert { 1.0 - value } else { value }
        })
        .collect();
    let field = RasterField::new(output.width as usize, output.height as usize, values)?;
    Ok(Decoded { field, channel })
}

pub(crate) fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .map_err(|e| Error::Input(e.to_string()))?
        .write_image_data(rgba)
        .map_err(|e| Error::Input(e.to_string()))?;
    Ok(bytes)
}
