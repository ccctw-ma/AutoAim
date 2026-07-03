use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::{
    error::Error,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

pub type Point = [f32; 2];

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum CaptureBackend {
    #[serde(rename = "gdi")]
    Gdi,
    #[serde(rename = "desktop_duplication")]
    DesktopDuplication,
}

impl CaptureBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gdi => "gdi",
            Self::DesktopDuplication => "desktop_duplication",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapturedFrame {
    pub screen_origin: [i32; 2],
    pub screen_size: [u32; 2],
    pub frame_size: [u32; 2],
    pub capture_backend: CaptureBackend,
    pub rgba: Vec<u8>,
    pub cursor: Point,
    pub cursor_on_screen: bool,
    pub timestamp_millis: u128,
}

impl CapturedFrame {
    pub fn rgba_base64(&self) -> String {
        STANDARD.encode(&self.rgba)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CapturedFramePreview {
    pub screen_origin: [i32; 2],
    pub screen_size: [u32; 2],
    pub frame_size: [u32; 2],
    pub capture_backend: CaptureBackend,
    pub rgba_base64: String,
    pub cursor: Point,
    pub cursor_on_screen: bool,
    pub timestamp_millis: u128,
}

impl From<&CapturedFrame> for CapturedFramePreview {
    fn from(frame: &CapturedFrame) -> Self {
        Self {
            screen_origin: frame.screen_origin,
            screen_size: frame.screen_size,
            frame_size: frame.frame_size,
            capture_backend: frame.capture_backend,
            rgba_base64: frame.rgba_base64(),
            cursor: frame.cursor,
            cursor_on_screen: frame.cursor_on_screen,
            timestamp_millis: frame.timestamp_millis,
        }
    }
}

#[derive(Debug)]
pub enum CaptureError {
    InvalidScreenSize([u32; 2]),
    NativeCallFailed(&'static str),
    BackendUnavailable(&'static str),
    UnsupportedPlatform(&'static str),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::InvalidScreenSize(size) => {
                write!(formatter, "invalid screen size: {}x{}", size[0], size[1])
            }
            CaptureError::NativeCallFailed(call) => write!(formatter, "{call} failed"),
            CaptureError::BackendUnavailable(message) => formatter.write_str(message),
            CaptureError::UnsupportedPlatform(message) => formatter.write_str(message),
        }
    }
}

impl Error for CaptureError {}

pub fn capture_screen_frame(
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    max_frame_size: [u32; 2],
) -> Result<CapturedFrame, CaptureError> {
    let frame_size = scaled_frame_size(screen_size, max_frame_size)?;
    let cursor = cursor_position()?;
    let cursor_on_screen = point_in_screen(cursor, screen_origin, screen_size);
    let (rgba, capture_backend) =
        capture_screen_region_rgba(screen_origin, screen_size, frame_size)?;

    Ok(CapturedFrame {
        screen_origin,
        screen_size,
        frame_size,
        capture_backend,
        rgba,
        cursor,
        cursor_on_screen,
        timestamp_millis: unix_timestamp_millis(),
    })
}

pub fn cursor_position() -> Result<Point, CaptureError> {
    platform_cursor_position()
}

pub fn point_in_screen(point: Point, origin: [i32; 2], size: [u32; 2]) -> bool {
    let left = origin[0] as f32;
    let top = origin[1] as f32;
    let right = left + size[0] as f32;
    let bottom = top + size[1] as f32;

    point[0] >= left && point[0] < right && point[1] >= top && point[1] < bottom
}

pub fn scaled_frame_size(
    screen_size: [u32; 2],
    max_frame_size: [u32; 2],
) -> Result<[u32; 2], CaptureError> {
    let [screen_width, screen_height] = screen_size;
    if screen_width == 0 || screen_height == 0 {
        return Err(CaptureError::InvalidScreenSize(screen_size));
    }

    let max_width = max_frame_size[0].max(1);
    let max_height = max_frame_size[1].max(1);
    let width_scale = max_width as f32 / screen_width as f32;
    let height_scale = max_height as f32 / screen_height as f32;
    let scale = width_scale.min(height_scale).min(1.0);
    let frame_width = ((screen_width as f32 * scale).round() as u32).max(1);
    let frame_height = ((screen_height as f32 * scale).round() as u32).max(1);

    Ok([frame_width, frame_height])
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn platform_cursor_position() -> Result<Point, CaptureError> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        return Err(CaptureError::NativeCallFailed("GetCursorPos"));
    }

    Ok([point.x as f32, point.y as f32])
}

#[cfg(not(target_os = "windows"))]
fn platform_cursor_position() -> Result<Point, CaptureError> {
    Err(CaptureError::UnsupportedPlatform(
        "native cursor capture is available only on Windows",
    ))
}

#[cfg(target_os = "windows")]
fn capture_screen_region_rgba(
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
) -> Result<(Vec<u8>, CaptureBackend), CaptureError> {
    if let Ok(rgba) = capture_screen_region_rgba_desktop_duplication(screen_size, frame_size) {
        return Ok((rgba, CaptureBackend::DesktopDuplication));
    }

    capture_screen_region_rgba_gdi(screen_origin, screen_size, frame_size)
        .map(|rgba| (rgba, CaptureBackend::Gdi))
}

#[cfg(target_os = "windows")]
fn capture_screen_region_rgba_desktop_duplication(
    screen_size: [u32; 2],
    frame_size: [u32; 2],
) -> Result<Vec<u8>, CaptureError> {
    use scrap::{Capturer, Display};
    use std::{io::ErrorKind, thread, time::Duration};

    let matching = Display::all()
        .map_err(|_| {
            CaptureError::BackendUnavailable("desktop duplication display enumeration failed")
        })?
        .into_iter()
        .filter(|display| {
            display.width() as u32 == screen_size[0] && display.height() as u32 == screen_size[1]
        })
        .collect::<Vec<_>>();

    let display = match matching.len() {
        1 => matching.into_iter().next().expect("display should exist"),
        0 => {
            return Err(CaptureError::BackendUnavailable(
                "desktop duplication display not found",
            ));
        }
        _ => {
            return Err(CaptureError::BackendUnavailable(
                "desktop duplication display match is ambiguous",
            ));
        }
    };

    let src_width = display.width() as usize;
    let src_height = display.height() as usize;
    let mut capturer = Capturer::new(display)
        .map_err(|_| CaptureError::BackendUnavailable("desktop duplication init failed"))?;

    for _ in 0..8 {
        match capturer.frame() {
            Ok(frame) => {
                let pitch = frame.len() / src_height.max(1);
                return Ok(scale_bgra_to_rgba(
                    &frame,
                    src_width,
                    src_height,
                    pitch,
                    frame_size[0] as usize,
                    frame_size[1] as usize,
                ));
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(8));
            }
            Err(_) => {
                return Err(CaptureError::BackendUnavailable(
                    "desktop duplication failed to capture frame",
                ));
            }
        }
    }

    Err(CaptureError::BackendUnavailable(
        "desktop duplication timed out while waiting for a frame",
    ))
}

#[cfg(target_os = "windows")]
fn scale_bgra_to_rgba(
    bgra: &[u8],
    src_width: usize,
    src_height: usize,
    src_pitch: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    let mut rgba = vec![0_u8; dst_width * dst_height * 4];
    let scale_x = src_width as f32 / dst_width.max(1) as f32;
    let scale_y = src_height as f32 / dst_height.max(1) as f32;

    for dst_y in 0..dst_height {
        let src_y = ((dst_y as f32 + 0.5) * scale_y)
            .floor()
            .clamp(0.0, src_height.saturating_sub(1) as f32) as usize;
        let src_row = src_y * src_pitch;
        for dst_x in 0..dst_width {
            let src_x = ((dst_x as f32 + 0.5) * scale_x)
                .floor()
                .clamp(0.0, src_width.saturating_sub(1) as f32) as usize;
            let src_index = src_row + src_x * 4;
            let dst_index = (dst_y * dst_width + dst_x) * 4;
            rgba[dst_index] = bgra[src_index + 2];
            rgba[dst_index + 1] = bgra[src_index + 1];
            rgba[dst_index + 2] = bgra[src_index];
            rgba[dst_index + 3] = 255;
        }
    }

    rgba
}

#[cfg(target_os = "windows")]
fn capture_screen_region_rgba_gdi(
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
) -> Result<Vec<u8>, CaptureError> {
    use std::{mem::size_of, ptr::null_mut};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, COLORONCOLOR, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
    };

    let [screen_x, screen_y] = screen_origin;
    let [screen_width, screen_height] = screen_size;
    let [frame_width, frame_height] = frame_size;
    let screen_width_i32 =
        i32::try_from(screen_width).map_err(|_| CaptureError::InvalidScreenSize(screen_size))?;
    let screen_height_i32 =
        i32::try_from(screen_height).map_err(|_| CaptureError::InvalidScreenSize(screen_size))?;
    let frame_width_i32 =
        i32::try_from(frame_width).map_err(|_| CaptureError::InvalidScreenSize(frame_size))?;
    let frame_height_i32 =
        i32::try_from(frame_height).map_err(|_| CaptureError::InvalidScreenSize(frame_size))?;

    unsafe {
        let screen_dc = GetDC(null_mut());
        if screen_dc.is_null() {
            return Err(CaptureError::NativeCallFailed("GetDC"));
        }

        let memory_dc = CreateCompatibleDC(screen_dc);
        if memory_dc.is_null() {
            ReleaseDC(null_mut(), screen_dc);
            return Err(CaptureError::NativeCallFailed("CreateCompatibleDC"));
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, frame_width_i32, frame_height_i32);
        if bitmap.is_null() {
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
            return Err(CaptureError::NativeCallFailed("CreateCompatibleBitmap"));
        }

        let previous_object = SelectObject(memory_dc, bitmap);
        if previous_object.is_null() {
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
            return Err(CaptureError::NativeCallFailed("SelectObject"));
        }

        SetStretchBltMode(memory_dc, COLORONCOLOR);
        let blt_ok = if frame_size == screen_size {
            BitBlt(
                memory_dc,
                0,
                0,
                frame_width_i32,
                frame_height_i32,
                screen_dc,
                screen_x,
                screen_y,
                SRCCOPY | CAPTUREBLT,
            )
        } else {
            StretchBlt(
                memory_dc,
                0,
                0,
                frame_width_i32,
                frame_height_i32,
                screen_dc,
                screen_x,
                screen_y,
                screen_width_i32,
                screen_height_i32,
                SRCCOPY | CAPTUREBLT,
            )
        };

        if blt_ok == 0 {
            SelectObject(memory_dc, previous_object);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
            return Err(CaptureError::NativeCallFailed("BitBlt/StretchBlt"));
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: frame_width_i32,
                biHeight: -frame_height_i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: frame_width * frame_height * 4,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut pixels = vec![0_u8; (frame_width as usize) * (frame_height as usize) * 4];
        let scanlines = GetDIBits(
            memory_dc,
            bitmap,
            0,
            frame_height,
            pixels.as_mut_ptr().cast(),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        SelectObject(memory_dc, previous_object);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);

        if scanlines == 0 {
            return Err(CaptureError::NativeCallFailed("GetDIBits"));
        }

        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }

        Ok(pixels)
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_screen_region_rgba(
    _screen_origin: [i32; 2],
    _screen_size: [u32; 2],
    _frame_size: [u32; 2],
) -> Result<(Vec<u8>, CaptureBackend), CaptureError> {
    Err(CaptureError::UnsupportedPlatform(
        "native screen capture is available only on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_frame_size_preserves_aspect_ratio() {
        assert_eq!(
            scaled_frame_size([1920, 1080], [960, 540]).unwrap(),
            [960, 540]
        );
        assert_eq!(
            scaled_frame_size([1920, 1080], [640, 640]).unwrap(),
            [640, 360]
        );
    }

    #[test]
    fn point_in_screen_uses_screen_space_coordinates() {
        assert!(point_in_screen([150.0, 120.0], [100, 50], [800, 600]));
        assert!(!point_in_screen([99.0, 120.0], [100, 50], [800, 600]));
        assert!(!point_in_screen([900.0, 120.0], [100, 50], [800, 600]));
    }

    #[test]
    fn captured_frame_preview_keeps_capture_backend() {
        let frame = CapturedFrame {
            screen_origin: [0, 0],
            screen_size: [1920, 1080],
            frame_size: [960, 540],
            capture_backend: CaptureBackend::DesktopDuplication,
            rgba: vec![0; 960 * 540 * 4],
            cursor: [12.0, 24.0],
            cursor_on_screen: true,
            timestamp_millis: 123,
        };

        let preview = CapturedFramePreview::from(&frame);
        assert_eq!(preview.capture_backend, CaptureBackend::DesktopDuplication);
        assert_eq!(preview.frame_size, [960, 540]);
        assert_eq!(preview.cursor, [12.0, 24.0]);
    }
}
