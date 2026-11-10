use crate::{model::Metadata, platform};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::{Component, Path, PathBuf, Prefix};
use windows::{
    Foundation::IPropertyValue,
    Graphics::Imaging::*,
    Storage::{
        FileProperties::{ThumbnailMode, ThumbnailOptions, ThumbnailType},
        StorageFile,
        Streams::IRandomAccessStream,
    },
    Win32::System::WinRT::CreateRandomAccessStreamOnFile,
    core::{HSTRING, Interface},
};

#[derive(serde::Deserialize)]
struct Request {
    operation: String,
    path: String,
    output: Option<PathBuf>,
    #[serde(rename = "maxSize")]
    max_size: Option<u32>,
}

// StorageFile expects a DOS path; the Win32 decoder stream retains the verbatim path.
pub(super) fn storage_path(path: &Path) -> PathBuf {
    if let Some(Component::Prefix(prefix)) = path.components().next()
        && let Prefix::VerbatimDisk(letter) = prefix.kind()
    {
        let mut result = PathBuf::from(format!("{}:\\", letter as char));
        result.extend(path.components().skip(2));
        return result;
    }
    path.to_owned()
}

fn exif_text(decoder: &BitmapDecoder, name: &str) -> Option<String> {
    let keys: windows_collections::IIterable<HSTRING> = vec![HSTRING::from(name)].into();
    let properties = decoder
        .BitmapProperties()
        .ok()?
        .GetPropertiesAsync(&keys)
        .ok()?
        .join()
        .ok()?;
    let value = properties
        .Lookup(&HSTRING::from(name))
        .ok()?
        .Value()
        .ok()?
        .cast::<IPropertyValue>()
        .ok()?;
    Some(value.GetString().ok()?.to_string_lossy())
}

fn write_jpeg(decoder: &BitmapDecoder, output: &Path, max_size: u32) -> Result<()> {
    let (width, height) = (decoder.PixelWidth()?, decoder.PixelHeight()?);
    if width == 0
        || height == 0
        || width > 100_000
        || height > 100_000
        || u64::from(width) * u64::from(height) > 200_000_000
    {
        bail!("image_dimensions_limit");
    }
    let scale = (max_size as f64 / width.max(height) as f64).min(1.0);
    let (scaled_width, scaled_height) = (
        ((width as f64 * scale) as u32).max(1),
        ((height as f64 * scale) as u32).max(1),
    );
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(scaled_width)?;
    transform.SetScaledHeight(scaled_height)?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
    let pixels = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transform,
            ExifOrientationMode::RespectExifOrientation,
            ColorManagementMode::ColorManageToSRgb,
        )?
        .join()?
        .DetachPixelData()?;
    let (width, height) = if decoder.OrientedPixelWidth()? != width {
        (scaled_height, scaled_width)
    } else {
        (scaled_width, scaled_height)
    };
    // Start with a fresh file and encoder: source EXIF/GPS is never copied into derivatives.
    std::fs::write(output, [])?;
    let stream: IRandomAccessStream =
        unsafe { CreateRandomAccessStreamOnFile(&HSTRING::from(output), 1)? };
    let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId()?, &stream)?.join()?;
    encoder.SetIsThumbnailGenerated(false)?;
    // Premultiplied RGB with ignored alpha composites transparency on black.
    encoder.SetPixelData(
        BitmapPixelFormat::Bgra8,
        BitmapAlphaMode::Ignore,
        width,
        height,
        96.0,
        96.0,
        &pixels,
    )?;
    encoder.FlushAsync()?.join()?;
    stream.FlushAsync()?.join()?;
    stream.Close()?;
    Ok(())
}

pub fn probe(bytes: &[u8]) -> Result<Metadata> {
    let req: Request = serde_json::from_slice(bytes)?;
    if req.operation != "probe" {
        bail!("Invalid media operation");
    }
    let bytes = STANDARD.decode(&req.path)?;
    if bytes.len() % 2 != 0 || bytes.as_chunks::<2>().0.contains(&[0, 0]) {
        bail!("Invalid path");
    }
    let path = platform::from_bytes(bytes);
    let metadata = std::fs::symlink_metadata(&path)?;
    if platform::is_link(&metadata) || platform::is_placeholder(&metadata) || !metadata.is_file() {
        bail!("file_unavailable");
    }
    let max_size = req.max_size.unwrap_or(512).clamp(64, 2560);
    let file =
        StorageFile::GetFileFromPathAsync(&HSTRING::from(storage_path(&path).as_path()))?.join()?;
    let video = path.extension().is_some_and(|e| {
        ["mp4", "mov", "m4v", "3gp", "avi", "mkv"]
            .iter()
            .any(|v| e.eq_ignore_ascii_case(v))
    });
    let mut result = Metadata {
        metadata_state: "ready".into(),
        ..Default::default()
    };
    if video {
        let properties = file.Properties()?.GetVideoPropertiesAsync()?.join()?;
        let (width, height) = (properties.Width()?, properties.Height()?);
        if width == 0 || height == 0 {
            bail!("unsupported_or_corrupt_video");
        }
        result.width = Some(width as i64);
        result.height = Some(height as i64);
        result.duration = Some(properties.Duration()?.Duration as f64 / 10_000_000.0);
        if let (Ok(lat), Ok(lon)) = (
            properties.Latitude().and_then(|v| v.Value()),
            properties.Longitude().and_then(|v| v.Value()),
        ) && lat.abs() <= 90.0
            && lon.abs() <= 180.0
        {
            result.latitude = Some(lat);
            result.longitude = Some(lon);
        }
        if let Some(output) = &req.output {
            let thumbnail = (|| -> Result<()> {
                let thumbnail = file
                    .GetThumbnailAsync(
                        ThumbnailMode::VideosView,
                        max_size,
                        ThumbnailOptions::UseCurrentScale,
                    )?
                    .join()?;
                if thumbnail.Type()? != ThumbnailType::Image {
                    bail!("No video thumbnail");
                }
                let decoder = BitmapDecoder::CreateAsync(&thumbnail)?.join()?;
                write_jpeg(&decoder, output, max_size)
            })();
            result.thumbnail_state = if thumbnail.is_ok() { "ready" } else { "failed" }.into();
            if thumbnail.is_err() {
                result.error = Some("video_thumbnail_failed_or_codec_unavailable".into());
            }
        }
    } else {
        let stream: IRandomAccessStream =
            unsafe { CreateRandomAccessStreamOnFile(&HSTRING::from(path.as_path()), 0)? };
        let decoder = BitmapDecoder::CreateAsync(&stream)?
            .join()
            .context("unsupported_or_corrupt_image")?;
        let (width, height) = (
            decoder.OrientedPixelWidth()?,
            decoder.OrientedPixelHeight()?,
        );
        if width == 0
            || height == 0
            || width > 100_000
            || height > 100_000
            || u64::from(width) * u64::from(height) > 200_000_000
        {
            bail!("image_dimensions_limit");
        }
        result.width = Some(width as i64);
        result.height = Some(height as i64);
        result.captured = exif_text(&decoder, "/app1/ifd/exif/{ushort=36867}")
            .or_else(|| exif_text(&decoder, "/ifd/exif/{ushort=36867}"));
        if let Ok(properties) = file.Properties()?.GetImagePropertiesAsync()?.join() {
            result.camera = properties
                .CameraModel()
                .ok()
                .map(|v| v.to_string_lossy())
                .filter(|v| !v.is_empty())
                .map(|v| v.chars().take(256).collect());
            if let (Ok(lat), Ok(lon)) = (
                properties.Latitude().and_then(|v| v.Value()),
                properties.Longitude().and_then(|v| v.Value()),
            ) && lat.abs() <= 90.0
                && lon.abs() <= 180.0
            {
                result.latitude = Some(lat);
                result.longitude = Some(lon);
            }
        }
        if let Some(output) = &req.output {
            let thumbnail = write_jpeg(&decoder, output, max_size);
            result.thumbnail_state = if thumbnail.is_ok() { "ready" } else { "failed" }.into();
            if thumbnail.is_err() {
                result.error = Some("image_thumbnail_failed".into());
            }
        }
    }
    Ok(result)
}
