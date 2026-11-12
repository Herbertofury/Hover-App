use crate::{
    DecodeError, DecodeLimits, DecodedImage, ImageFormat, ensure_pixel_limit,
    validate_local_image_source,
};
use hover_core::CancellationToken;
use std::path::Path;

#[cfg(windows)]
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmapSource, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
#[cfg(windows)]
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
#[cfg(windows)]
use windows::core::Interface;

pub fn decode_local_file(
    path: &Path,
    target_width: u32,
    target_height: u32,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<DecodedImage, DecodeError> {
    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }
    let format = validate_local_image_source(path, limits)?;
    decode_local_file_inner(
        path,
        target_width,
        target_height,
        limits,
        format,
        cancellation,
    )
}

#[cfg(windows)]
fn decode_local_file_inner(
    path: &Path,
    target_width: u32,
    target_height: u32,
    limits: DecodeLimits,
    format: ImageFormat,
    cancellation: &CancellationToken,
) -> Result<DecodedImage, DecodeError> {
    let bytes =
        crate::local_io::read_local_snapshot(path, limits.max_local_file_size_bytes, cancellation)?;
    // The guard is declared before all COM interfaces so they drop first.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _com = ComGuard;

    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }

    let factory: IWICImagingFactory = unsafe {
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };
    let stream = unsafe { factory.CreateStream() }
        .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
    unsafe { stream.InitializeFromMemory(&bytes) }
        .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
    let decoder = unsafe {
        factory
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };
    let frame = unsafe {
        decoder
            .GetFrame(0)
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };
    let mut width = 0;
    let mut height = 0;
    unsafe {
        frame
            .GetSize(&mut width, &mut height)
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };
    ensure_pixel_limit(width, height, limits)?;

    let (output_width, output_height) = fit_decode_size(
        width,
        height,
        target_width,
        target_height,
        limits.max_output_dimension,
    );
    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }

    let source: IWICBitmapSource = if output_width != width || output_height != height {
        let scaler = unsafe {
            factory
                .CreateBitmapScaler()
                .map_err(|error| DecodeError::Wic(error.message().to_string()))?
        };
        unsafe {
            scaler
                .Initialize(
                    &frame,
                    output_width,
                    output_height,
                    WICBitmapInterpolationModeFant,
                )
                .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
        }
        scaler
            .cast()
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    } else {
        frame
            .cast()
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };

    let converter = unsafe {
        factory
            .CreateFormatConverter()
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?
    };
    unsafe {
        converter
            .Initialize(
                &source,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
    }

    let stride = output_width.saturating_mul(4);
    let mut pixels = vec![0u8; stride as usize * output_height as usize];
    unsafe {
        converter
            .CopyPixels(std::ptr::null(), stride, &mut pixels)
            .map_err(|error| DecodeError::Wic(error.message().to_string()))?;
    }

    Ok(DecodedImage {
        width: output_width,
        height: output_height,
        stride,
        pixels_bgra: pixels,
        format,
    })
}

#[cfg(not(windows))]
fn decode_local_file_inner(
    _path: &Path,
    _target_width: u32,
    _target_height: u32,
    _limits: DecodeLimits,
    _format: ImageFormat,
    _cancellation: &CancellationToken,
) -> Result<DecodedImage, DecodeError> {
    Err(DecodeError::UnsupportedFormat)
}

fn fit_decode_size(
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
    max_dimension: u32,
) -> (u32, u32) {
    let target_width = target_width.max(1).min(max_dimension);
    let target_height = target_height.max(1).min(max_dimension);
    if width <= target_width && height <= target_height {
        return (width, height);
    }
    let scale = (target_width as f32 / width as f32).min(target_height as f32 / height as f32);
    let out_width = ((width as f32 * scale).round() as u32)
        .max(1)
        .min(max_dimension);
    let out_height = ((height as f32 * scale).round() as u32)
        .max(1)
        .min(max_dimension);
    (out_width, out_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_decode_size_preserves_aspect() {
        assert_eq!(fit_decode_size(4000, 2000, 1000, 1000, 32768), (1000, 500));
        assert_eq!(fit_decode_size(400, 200, 1000, 1000, 32768), (400, 200));
    }
}
