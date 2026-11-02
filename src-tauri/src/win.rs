//! Thin wrappers over the Win32 calls the dock needs.

use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;
use std::sync::Mutex;

use base64::Engine;
use serde::Serialize;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HWND, LPARAM, POINT, RECT, SIZE,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_CLOAKED, DWMWA_TEXT_COLOR,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE, DwmGetWindowAttribute, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetMonitorInfoW, GetObjectW, HBITMAP, HGDIOBJ,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW,
};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    SendInput, VIRTUAL_KEY, VK_A, VK_LWIN, VK_N,
};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, QUNS_RUNNING_D3D_FULL_SCREEN, SHCreateItemFromParsingName,
    SHQueryUserNotificationState, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, FindWindowExW, FindWindowW, GW_OWNER, GWL_EXSTYLE,
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed,
    MONITORINFOF_PRIMARY, SPI_GETWORKAREA, SW_HIDE,
    SW_MINIMIZE, SW_RESTORE, SW_SHOWNA, SW_SHOWNORMAL, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, ShowWindowAsync, SystemParametersInfoW,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};
use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR, w};

const ICON_SIZE: i32 = 128;
const MAX_IMAGE_BYTES: u64 = 3 * 1024 * 1024;
const SHELL_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];
/// Shell surfaces that can span the screen, such as the desktop and Alt+Tab.
const NOT_FULLSCREEN_APPS: [&str; 6] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "XamlExplorerHostIslandWindow",
    "MultitaskingViewFrame",
    "Windows.UI.Core.CoreWindow",
];
const AUTOSTART_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const AUTOSTART_VALUE: PCWSTR = w!("Dock JM");

static ICON_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, PartialEq, Serialize)]
pub struct AppWindow {
    pub hwnd: isize,
    pub title: String,
    pub exe: String,
    pub active: bool,
}

fn to_hwnd(hwnd: isize) -> HWND {
    HWND(hwnd as *mut c_void)
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some((point.x, point.y))
}

/// Usable area of the primary monitor (the screen minus a visible taskbar).
pub fn work_area() -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .ok()?;
    Some(rect)
}

/// Keeps the dock out of Alt+Tab and toggles whether clicks pass through it.
/// Tauri's own toggle rewrites the whole extended style, which would undo the
/// tool window flag, so the style is managed here instead.
/// Turns "#rrggbb" into the 0x00BBGGRR layout Win32 expects.
fn colorref(color: &str) -> Option<u32> {
    let rgb = u32::from_str_radix(color.strip_prefix('#')?, 16).ok()?;
    Some((rgb & 0xff) << 16 | (rgb & 0xff00) | (rgb >> 16) & 0xff)
}

/// Windows 10 ignores the caption colors and only applies the dark mode.
pub fn set_title_bar_colors(hwnd: isize, caption: &str, text: &str, dark: bool) {
    let window = to_hwnd(hwnd);
    let set = |attribute: DWMWINDOWATTRIBUTE, value: u32| unsafe {
        let _ = DwmSetWindowAttribute(
            window,
            attribute,
            &value as *const u32 as *const c_void,
            size_of::<u32>() as u32,
        );
    };
    set(DWMWA_USE_IMMERSIVE_DARK_MODE, dark as u32);
    if let Some(color) = colorref(caption) {
        set(DWMWA_CAPTION_COLOR, color);
        set(DWMWA_BORDER_COLOR, color);
    }
    if let Some(color) = colorref(text) {
        set(DWMWA_TEXT_COLOR, color);
    }
}

pub fn set_dock_style(hwnd: isize, click_through: bool) {
    let hwnd = to_hwnd(hwnd);
    let pass_through = WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0;
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let base = (current & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
        let style = if click_through {
            base | pass_through
        } else {
            base & !pass_through
        };
        if style != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize);
        }
    }
}

/// Adds or removes the program from the apps Windows opens at sign-in.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let result = unsafe {
        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let command: Vec<u16> = format!("\"{}\"", exe.display())
                .encode_utf16()
                .chain(Some(0))
                .collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                AUTOSTART_KEY,
                AUTOSTART_VALUE,
                REG_SZ.0,
                Some(command.as_ptr() as *const c_void),
                (command.len() * size_of::<u16>()) as u32,
            )
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, AUTOSTART_KEY, AUTOSTART_VALUE)
        }
    };
    // Removing an entry that is already gone is not a failure.
    if result == ERROR_SUCCESS || (!enabled && result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(format!("Não foi possível alterar o início automático ({})", result.0))
    }
}

/// True when the foreground app takes over the whole primary monitor, where
/// the dock lives. A maximized window also covers it when the taskbar is
/// hidden, but that is a normal window the dock should stay over.
pub fn is_fullscreen_app_active(own_pid: u32) -> bool {
    unsafe {
        if SHQueryUserNotificationState().is_ok_and(|state| state == QUNS_RUNNING_D3D_FULL_SCREEN) {
            return true;
        }

        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid()
            || window_pid(hwnd) == own_pid
            || IsZoomed(hwnd).as_bool()
            || NOT_FULLSCREEN_APPS.contains(&window_class(hwnd).as_str())
        {
            return false;
        }

        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mut rect = RECT::default();
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(monitor, &mut info).as_bool()
            || info.dwFlags & MONITORINFOF_PRIMARY == 0
            || GetWindowRect(hwnd, &mut rect).is_err()
        {
            return false;
        }
        let screen = info.rcMonitor;
        rect.left <= screen.left
            && rect.top <= screen.top
            && rect.right >= screen.right
            && rect.bottom >= screen.bottom
    }
}

/// Foreground window, or `None` when it belongs to the dock itself.
pub fn foreground_window(own_pid: u32) -> Option<isize> {
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.is_invalid() && window_pid(hwnd) != own_pid).then_some(hwnd.0 as isize)
}

/// Top-level windows a user would switch to, topmost first.
pub fn list_windows(own_pid: u32, active: Option<isize>) -> Vec<AppWindow> {
    let mut handles: Vec<HWND> = Vec::new();
    let _ = unsafe {
        EnumWindows(
            Some(collect_hwnd),
            LPARAM(&mut handles as *mut Vec<HWND> as isize),
        )
    };

    handles
        .into_iter()
        .filter(|&hwnd| window_pid(hwnd) != own_pid && is_app_window(hwnd))
        .filter_map(|hwnd| {
            Some(AppWindow {
                hwnd: hwnd.0 as isize,
                title: window_title(hwnd),
                exe: window_exe(hwnd)?,
                active: active == Some(hwnd.0 as isize),
            })
        })
        .collect()
}

unsafe extern "system" fn collect_hwnd(hwnd: HWND, list: LPARAM) -> BOOL {
    unsafe { (*(list.0 as *mut Vec<HWND>)).push(hwnd) };
    true.into()
}

fn is_app_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || GetWindow(hwnd, GW_OWNER).is_ok() {
            return false;
        }

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let hidden = ex_style & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) != 0;
        if hidden && ex_style & WS_EX_APPWINDOW.0 == 0 {
            return false;
        }

        // Suspended store apps stay "visible" but are cloaked by the compositor.
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return false;
        }

        !SHELL_CLASSES.contains(&window_class(hwnd).as_str()) && !window_title(hwnd).is_empty()
    }
}

fn window_class(hwnd: HWND) -> String {
    let mut class = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
    String::from_utf16_lossy(&class[..len])
}

fn window_title(hwnd: HWND) -> String {
    let mut title = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) } as usize;
    String::from_utf16_lossy(&title[..len])
}

fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn window_exe(hwnd: HWND) -> Option<String> {
    let pid = window_pid(hwnd);
    let exe = process_path(pid)?;
    if !exe.to_lowercase().ends_with("applicationframehost.exe") {
        return Some(exe);
    }

    // Store apps are hosted by a frame process; the real app is a child window.
    let mut host = (pid, None::<u32>);
    unsafe {
        let _ = EnumChildWindows(
            Some(hwnd),
            Some(find_hosted_pid),
            LPARAM(&mut host as *mut (u32, Option<u32>) as isize),
        );
    }
    host.1.and_then(process_path).or(Some(exe))
}

unsafe extern "system" fn find_hosted_pid(child: HWND, host: LPARAM) -> BOOL {
    let host = unsafe { &mut *(host.0 as *mut (u32, Option<u32>)) };
    let pid = window_pid(child);
    if pid != host.0 {
        host.1 = Some(pid);
        return false.into();
    }
    true.into()
}

fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

pub fn focus_window(hwnd: isize) -> Result<(), String> {
    let hwnd = to_hwnd(hwnd);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("A janela não existe mais".into());
        }
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            // Windows refuses focus changes from background processes. Empty
            // input makes this process the last one to have received input.
            let input = INPUT {
                r#type: INPUT_MOUSE,
                ..Default::default()
            };
            SendInput(&[input], size_of::<INPUT>() as i32);
            if !SetForegroundWindow(hwnd).as_bool() {
                return Err("O Windows não permitiu trocar de janela".into());
            }
        }
    }
    Ok(())
}

/// Shows or hides the Windows taskbar on every monitor. It only touches the
/// windows, so nothing outlives Explorer restarting.
pub fn set_taskbar_visible(visible: bool) {
    let command = if visible { SW_SHOWNA } else { SW_HIDE };
    let apply = |hwnd: HWND| unsafe {
        if IsWindowVisible(hwnd).as_bool() != visible {
            let _ = ShowWindow(hwnd, command);
        }
    };

    unsafe {
        if let Ok(primary) = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) {
            apply(primary);
        }
        let mut previous = None;
        while let Ok(secondary) =
            FindWindowExW(None, previous, w!("Shell_SecondaryTrayWnd"), PCWSTR::null())
        {
            apply(secondary);
            previous = Some(secondary);
        }
    }
}

/// Presses the keys in order and releases them in reverse, like a shortcut.
fn press_keys(keys: &[VIRTUAL_KEY]) {
    let event = |key, flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let down = keys.iter().map(|&key| event(key, KEYBD_EVENT_FLAGS(0)));
    let up = keys.iter().rev().map(|&key| event(key, KEYEVENTF_KEYUP));
    let inputs: Vec<INPUT> = down.chain(up).collect();
    unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

/// The Windows key opens the Start menu, or closes it if open.
pub fn open_start_menu() {
    press_keys(&[VK_LWIN]);
}

/// Win+A: the Wi-Fi, volume and battery panel.
pub fn open_quick_settings() {
    press_keys(&[VK_LWIN, VK_A]);
}

/// Win+N: notifications and the calendar.
pub fn open_calendar() {
    press_keys(&[VK_LWIN, VK_N]);
}

pub fn minimize_window(hwnd: isize) {
    unsafe {
        let _ = ShowWindowAsync(to_hwnd(hwnd), SW_MINIMIZE);
    }
}

pub fn launch(path: &str) -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(path),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute reports success with any value above 32.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Não foi possível abrir {path}"))
    }
}

/// Reads a picture chosen by the user as a data URL. The size is capped because
/// it travels inside the config on every change.
pub fn read_image(path: &str) -> Result<String, String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase);
    let mime = match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => return Err("Escolha uma imagem PNG, JPG, WEBP ou GIF".into()),
    };
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_IMAGE_BYTES {
        return Err("A imagem precisa ter no máximo 3 MB".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

/// Icon of a file as a PNG data URL.
pub fn file_icon(path: &str) -> Option<String> {
    if !Path::new(path).exists() {
        return None;
    }

    // Parallel calls fail while the shell is still loading at startup.
    let _guard = ICON_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let icon = render_icon(path);
        if com.is_ok() {
            CoUninitialize();
        }
        icon
    }
}

unsafe fn render_icon(path: &str) -> Option<String> {
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
        let bitmap = factory
            .GetImage(
                SIZE {
                    cx: ICON_SIZE,
                    cy: ICON_SIZE,
                },
                SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
            )
            .ok()?;
        let pixels = read_bitmap(bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let (width, height, pixels) = pixels?;
        encode_png(width, height, &pixels)
    }
}

/// Copies a bitmap out as straight-alpha RGBA, top row first.
unsafe fn read_bitmap(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut info = BITMAP::default();
        let read = GetObjectW(
            HGDIOBJ(bitmap.0),
            size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut c_void),
        );
        if read == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (info.bmWidth, info.bmHeight);

        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        if lines == 0 {
            return None;
        }

        // Icons without an alpha channel come back fully transparent.
        let opaque = pixels.chunks_exact(4).all(|pixel| pixel[3] == 0);
        for pixel in pixels.chunks_exact_mut(4) {
            let alpha = if opaque { 255 } else { pixel[3] as u32 };
            let unpremultiply = |value: u8| match alpha {
                0 | 255 => value,
                _ => (value as u32 * 255 / alpha).min(255) as u8,
            };
            let (blue, green, red) = (pixel[0], pixel[1], pixel[2]);
            pixel[0] = unpremultiply(red);
            pixel[1] = unpremultiply(green);
            pixel[2] = unpremultiply(blue);
            pixel[3] = alpha as u8;
        }
        Some((width as u32, height as u32, pixels))
    }
}

fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Option<String> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(pixels).ok()?;
    writer.finish().ok()?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}
