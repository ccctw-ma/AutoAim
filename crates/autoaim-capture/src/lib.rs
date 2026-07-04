use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::{
    error::Error,
    fmt,
    sync::mpsc::{Receiver, RecvTimeoutError, Sender},
    thread::JoinHandle,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    CaptureTimedOut,
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
            CaptureError::CaptureTimedOut => formatter.write_str("screen capture timed out"),
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
    let cursor = cursor_position().unwrap_or_else(|_| screen_center(screen_origin, screen_size));
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

fn screen_center(origin: [i32; 2], size: [u32; 2]) -> Point {
    [
        origin[0] as f32 + size[0] as f32 / 2.0,
        origin[1] as f32 + size[1] as f32 / 2.0,
    ]
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

struct CaptureRequest {
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
}

type CaptureResponse = Result<(Vec<u8>, CaptureBackend), CaptureError>;

/// Reusable screen capturer that keeps its native capture session alive across
/// frames on a dedicated thread. Rebuilding the DXGI desktop-duplication
/// session on every frame leaks GPU capture resources and can stall for many
/// seconds; owning one long-lived session and talking to it over a channel lets
/// callers apply a hard per-frame timeout instead of blocking forever.
pub struct ScreenCapturer {
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
    request_tx: Sender<CaptureRequest>,
    response_rx: Receiver<CaptureResponse>,
    worker: Option<JoinHandle<()>>,
}

impl ScreenCapturer {
    pub fn new(
        screen_origin: [i32; 2],
        screen_size: [u32; 2],
        max_frame_size: [u32; 2],
    ) -> Result<Self, CaptureError> {
        let frame_size = scaled_frame_size(screen_size, max_frame_size)?;
        let (request_tx, request_rx) = std::sync::mpsc::channel::<CaptureRequest>();
        let (response_tx, response_rx) = std::sync::mpsc::channel::<CaptureResponse>();

        let worker = std::thread::Builder::new()
            .name("autoaim-screen-capture".to_string())
            .spawn(move || capture_worker_loop(request_rx, response_tx))
            .map_err(|_| CaptureError::BackendUnavailable("failed to start capture thread"))?;

        Ok(Self {
            screen_origin,
            screen_size,
            frame_size,
            request_tx,
            response_rx,
            worker: Some(worker),
        })
    }

    pub fn matches(&self, screen_origin: [i32; 2], screen_size: [u32; 2]) -> bool {
        self.screen_origin == screen_origin && self.screen_size == screen_size
    }

    /// Capture a single frame, giving up after `timeout` so a stalled native
    /// capture call can never freeze the caller. On timeout the worker thread is
    /// abandoned (it is dropped without being joined) so the next attempt starts
    /// a fresh session.
    pub fn capture(&mut self, timeout: Duration) -> Result<CapturedFrame, CaptureError> {
        let cursor = cursor_position()
            .unwrap_or_else(|_| screen_center(self.screen_origin, self.screen_size));
        let cursor_on_screen = point_in_screen(cursor, self.screen_origin, self.screen_size);

        self.request_tx
            .send(CaptureRequest {
                screen_origin: self.screen_origin,
                screen_size: self.screen_size,
                frame_size: self.frame_size,
            })
            .map_err(|_| CaptureError::BackendUnavailable("capture thread is not running"))?;

        match self.response_rx.recv_timeout(timeout) {
            Ok(Ok((rgba, capture_backend))) => Ok(CapturedFrame {
                screen_origin: self.screen_origin,
                screen_size: self.screen_size,
                frame_size: self.frame_size,
                capture_backend,
                rgba,
                cursor,
                cursor_on_screen,
                timestamp_millis: unix_timestamp_millis(),
            }),
            Ok(Err(error)) => Err(error),
            Err(RecvTimeoutError::Timeout) => {
                // Abandon the stalled worker so the next capture starts fresh.
                self.worker.take();
                Err(CaptureError::CaptureTimedOut)
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.worker.take();
                Err(CaptureError::BackendUnavailable("capture thread stopped"))
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.worker.is_some()
    }
}

fn capture_worker_loop(request_rx: Receiver<CaptureRequest>, response_tx: Sender<CaptureResponse>) {
    #[cfg(target_os = "windows")]
    let mut desktop_duplication = DesktopDuplicationCaptureState::default();

    while let Ok(request) = request_rx.recv() {
        #[cfg(target_os = "windows")]
        let result = capture_screen_region_rgba_cached(
            request.screen_origin,
            request.screen_size,
            request.frame_size,
            &mut desktop_duplication,
        );
        #[cfg(not(target_os = "windows"))]
        let result = capture_screen_region_rgba(
            request.screen_origin,
            request.screen_size,
            request.frame_size,
        );
        if response_tx.send(result).is_err() {
            break;
        }
    }
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
fn capture_screen_region_rgba_cached(
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
    desktop_duplication: &mut DesktopDuplicationCaptureState,
) -> Result<(Vec<u8>, CaptureBackend), CaptureError> {
    if let Ok(rgba) = desktop_duplication.capture(screen_size, frame_size) {
        return Ok((rgba, CaptureBackend::DesktopDuplication));
    }

    capture_screen_region_rgba_gdi(screen_origin, screen_size, frame_size)
        .map(|rgba| (rgba, CaptureBackend::Gdi))
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct DesktopDuplicationCaptureState {
    screen_size: Option<[u32; 2]>,
    frame_size: Option<[u32; 2]>,
    src_width: usize,
    src_height: usize,
    capturer: Option<scrap::Capturer>,
    last_rgba: Option<Vec<u8>>,
}

#[cfg(target_os = "windows")]
impl DesktopDuplicationCaptureState {
    fn capture(
        &mut self,
        screen_size: [u32; 2],
        frame_size: [u32; 2],
    ) -> Result<Vec<u8>, CaptureError> {
        use std::{io::ErrorKind, thread, time::Duration};

        self.ensure_capturer(screen_size)?;
        if self.frame_size != Some(frame_size) {
            self.frame_size = Some(frame_size);
            self.last_rgba = None;
        }

        let wait_attempts = if self.last_rgba.is_some() { 1 } else { 8 };
        for _ in 0..wait_attempts {
            let capturer = self
                .capturer
                .as_mut()
                .ok_or(CaptureError::BackendUnavailable(
                    "desktop duplication capturer is not initialized",
                ))?;
            match capturer.frame() {
                Ok(frame) => {
                    let pitch = frame.len() / self.src_height.max(1);
                    let rgba = scale_bgra_to_rgba(
                        &frame,
                        self.src_width,
                        self.src_height,
                        pitch,
                        frame_size[0] as usize,
                        frame_size[1] as usize,
                    );
                    self.last_rgba = Some(rgba.clone());
                    return Ok(rgba);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if let Some(rgba) = self.last_rgba.as_ref() {
                        return Ok(rgba.clone());
                    }
                    thread::sleep(Duration::from_millis(4));
                }
                Err(_) => {
                    self.reset();
                    return Err(CaptureError::BackendUnavailable(
                        "desktop duplication failed to capture frame",
                    ));
                }
            }
        }

        self.last_rgba
            .clone()
            .ok_or(CaptureError::BackendUnavailable(
                "desktop duplication timed out while waiting for a frame",
            ))
    }

    fn ensure_capturer(&mut self, screen_size: [u32; 2]) -> Result<(), CaptureError> {
        if self.capturer.is_some() && self.screen_size == Some(screen_size) {
            return Ok(());
        }

        self.reset();
        let display = desktop_duplication_display(screen_size)?;
        self.src_width = display.width();
        self.src_height = display.height();
        self.capturer =
            Some(scrap::Capturer::new(display).map_err(|_| {
                CaptureError::BackendUnavailable("desktop duplication init failed")
            })?);
        self.screen_size = Some(screen_size);
        Ok(())
    }

    fn reset(&mut self) {
        self.screen_size = None;
        self.frame_size = None;
        self.src_width = 0;
        self.src_height = 0;
        self.capturer = None;
        self.last_rgba = None;
    }
}

#[cfg(target_os = "windows")]
fn desktop_duplication_display(screen_size: [u32; 2]) -> Result<scrap::Display, CaptureError> {
    let matching = scrap::Display::all()
        .map_err(|_| {
            CaptureError::BackendUnavailable("desktop duplication display enumeration failed")
        })?
        .into_iter()
        .filter(|display| {
            display.width() as u32 == screen_size[0] && display.height() as u32 == screen_size[1]
        })
        .collect::<Vec<_>>();

    match matching.len() {
        1 => Ok(matching.into_iter().next().expect("display should exist")),
        0 => Err(CaptureError::BackendUnavailable(
            "desktop duplication display not found",
        )),
        _ => Err(CaptureError::BackendUnavailable(
            "desktop duplication display match is ambiguous",
        )),
    }
}

#[cfg(target_os = "windows")]
fn capture_screen_region_rgba_desktop_duplication(
    screen_size: [u32; 2],
    frame_size: [u32; 2],
) -> Result<Vec<u8>, CaptureError> {
    let mut state = DesktopDuplicationCaptureState::default();
    state.capture(screen_size, frame_size)
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
    let src_x_offsets = (0..dst_width)
        .map(|dst_x| {
            (((dst_x * 2 + 1) * src_width) / (dst_width.max(1) * 2))
                .min(src_width.saturating_sub(1))
                * 4
        })
        .collect::<Vec<_>>();
    let src_y_rows = (0..dst_height)
        .map(|dst_y| {
            (((dst_y * 2 + 1) * src_height) / (dst_height.max(1) * 2))
                .min(src_height.saturating_sub(1))
                * src_pitch
        })
        .collect::<Vec<_>>();

    for (dst_y, src_row) in src_y_rows.iter().copied().enumerate() {
        for (dst_x, src_x) in src_x_offsets.iter().copied().enumerate() {
            let src_index = src_row + src_x;
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
