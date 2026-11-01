mod config;
mod widgets;
mod win;

use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, RunEvent, State, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, Wry,
};
use windows::Win32::Foundation::RECT;

use config::{DockConfig, PinnedApp, WidgetId};
use widgets::WidgetData;
use win::AppWindow;

const CURSOR_POLL: Duration = Duration::from_millis(40);
/// How many cursor polls pass between window list refreshes (about 500 ms).
const TICKS_PER_SNAPSHOT: u32 = 12;
const SETTINGS_WINDOW: &str = "settings";
const DOCK_WINDOW: &str = "main";
const DOCK_RETRY: Duration = Duration::from_secs(1);
/// Grace period for reaching an auto-hidden dock after touching the screen edge.
const DOCK_HIDE_DELAY: Duration = Duration::from_millis(500);
const WIDGET_POLL: Duration = Duration::from_millis(100);
const WIDGET_REFRESH: Duration = Duration::from_secs(1);

/// Dock geometry measured by the frontend, in CSS pixels. `bottom` is the gap
/// between the bar and the bottom of the window.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Layout {
    x: f64,
    width: f64,
    height: f64,
    bottom: f64,
    /// How far the magnified icons can reach outside the bar at rest.
    extra: f64,
    window_height: f64,
}

#[derive(Default)]
struct DockState {
    enabled: AtomicBool,
    hide_taskbar: AtomicBool,
    auto_hide: AtomicBool,
    layout: Mutex<Option<Layout>>,
    windows: Mutex<Vec<AppWindow>>,
    tray_toggle: Mutex<Option<CheckMenuItem<Wry>>>,
    dock_on_screen: AtomicBool,
    widgets: Mutex<Vec<WidgetId>>,
    widget_data: Mutex<WidgetData>,
    /// Asks the widget thread to read again without waiting for its next turn.
    widgets_stale: AtomicBool,
    dragging: AtomicBool,
    start_with_windows: AtomicBool,
}

#[tauri::command]
fn get_config(app: AppHandle) -> Result<DockConfig, String> {
    config::load(&app)
}

#[tauri::command]
fn save_config(config: DockConfig, app: AppHandle) -> Result<(), String> {
    let config = config::store(&app, config)?;
    let state = app.state::<DockState>();
    let autostart = config.start_with_windows;
    if state.start_with_windows.swap(autostart, Ordering::Relaxed) != autostart {
        win::set_autostart(autostart)?;
    }
    apply_config(&app, config);
    Ok(())
}

/// Stores an order set by dragging icons. It reads the file again so a change
/// made in the settings window at the same time is not lost.
#[tauri::command]
fn save_dock_order(
    apps: Vec<PinnedApp>,
    app_order: Vec<String>,
    app: AppHandle,
) -> Result<(), String> {
    let mut config = config::load(&app)?;
    config.apps = apps;
    config.app_order = app_order;
    config::save(&app, &config)?;
    let _ = app.emit("dock-order-changed", config);
    Ok(())
}

#[tauri::command]
fn set_dragging(dragging: bool, state: State<DockState>) {
    state.dragging.store(dragging, Ordering::Relaxed);
}

#[tauri::command]
fn get_windows(state: State<DockState>) -> Vec<AppWindow> {
    state.windows.lock().unwrap().clone()
}

// Runs off the main thread because extracting an icon can touch the disk.
#[tauri::command(async)]
fn get_icon(path: String) -> Option<String> {
    win::file_icon(&path)
}

#[tauri::command]
fn set_layout(layout: Layout, state: State<DockState>) {
    *state.layout.lock().unwrap() = Some(layout);
}

#[tauri::command]
fn launch_app(path: String) -> Result<(), String> {
    win::launch(&path)
}

#[tauri::command]
fn focus_window(hwnd: isize) -> Result<(), String> {
    win::focus_window(hwnd)
}

#[tauri::command]
fn minimize_window(hwnd: isize) {
    win::minimize_window(hwnd);
}

#[tauri::command]
fn open_start_menu() {
    win::open_start_menu();
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    request_settings(&app);
}

#[tauri::command]
fn open_quick_settings() {
    win::open_quick_settings();
}

#[tauri::command]
fn open_calendar() {
    win::open_calendar();
}

#[tauri::command]
fn open_task_manager() -> Result<(), String> {
    win::launch("taskmgr.exe")
}

#[tauri::command]
fn get_widget_data(state: State<DockState>) -> WidgetData {
    state.widgets_stale.store(true, Ordering::Relaxed);
    state.widget_data.lock().unwrap().clone()
}

#[tauri::command(async)]
fn set_volume(level: f32, state: State<DockState>) -> Result<(), String> {
    widgets::set_volume(level)?;
    state.widgets_stale.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command(async)]
fn media_action(action: String, state: State<DockState>) -> Result<(), String> {
    widgets::media_action(&action)?;
    state.widgets_stale.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_config_file(app: AppHandle) -> Result<(), String> {
    let path = config::path(&app)?;
    Command::new("notepad.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Pushes a stored config to the running dock, tray and settings window.
fn apply_config(app: &AppHandle, config: DockConfig) {
    let state = app.state::<DockState>();
    state.enabled.store(config.enabled, Ordering::Relaxed);
    state.hide_taskbar.store(config.hide_taskbar, Ordering::Relaxed);
    state.auto_hide.store(config.auto_hide, Ordering::Relaxed);
    *state.widgets.lock().unwrap() = config.enabled_widgets();
    state.widgets_stale.store(true, Ordering::Relaxed);
    if let Some(toggle) = state.tray_toggle.lock().unwrap().as_ref() {
        let _ = toggle.set_checked(config.enabled);
    }
    let _ = app.emit("config-changed", config);
}

fn toggle_enabled(app: &AppHandle) {
    let enabled = !app.state::<DockState>().enabled.load(Ordering::Relaxed);
    // A broken file still lets the dock be toggled, just not remembered.
    match config::load(app) {
        Ok(mut config) => {
            config.enabled = enabled;
            let _ = config::save(app, &config);
            apply_config(app, config);
        }
        Err(_) => app.state::<DockState>().enabled.store(enabled, Ordering::Relaxed),
    }
}

/// Creating a window from the main thread deadlocks on Windows, and menu events
/// run there, so the window is always opened from its own thread.
fn request_settings(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        let window = match app.get_webview_window(SETTINGS_WINDOW) {
            Some(window) => {
                let _ = window.unminimize();
                window
            }
            None => {
                let built = WebviewWindowBuilder::new(
                    &app,
                    SETTINGS_WINDOW,
                    WebviewUrl::App("settings.html".into()),
                )
                .title("Personalizar Dock JM")
                .inner_size(620.0, 780.0)
                .min_inner_size(520.0, 480.0)
                .center()
                .build();
                let Ok(window) = built else {
                    return;
                };
                window
            }
        };
        if let Ok(hwnd) = window.hwnd() {
            let _ = win::focus_window(hwnd.0 as isize);
        }
    });
}

/// Pins the window to the bottom of the work area, spanning its full width.
fn place_window(window: &WebviewWindow, area: RECT, height: f64) -> tauri::Result<()> {
    let height = height.ceil() as i32;
    window.set_size(PhysicalSize::new(area.right - area.left, height))?;
    window.set_position(PhysicalPosition::new(area.left, area.bottom - height))
}

fn is_over_dock(
    (x, y): (i32, i32),
    area: RECT,
    layout: Layout,
    scale: f64,
    hovering: bool,
    auto_hide: bool,
) -> bool {
    let (x, y) = (x as f64, y as f64);
    let reach = if hovering { layout.extra * scale } else { 0.0 };
    let left = area.left as f64 + layout.x * scale - reach;
    let right = area.left as f64 + (layout.x + layout.width) * scale + reach;
    let bar_bottom = area.bottom as f64 - layout.bottom * scale;
    let top = bar_bottom - layout.height * scale - reach;
    // The cursor arrives from the screen edge, so the gap below must not count as leaving.
    let bottom = if auto_hide { area.bottom as f64 } else { bar_bottom };
    x >= left && x <= right && y >= top && y <= bottom
}

fn is_at_bottom_edge((x, y): (i32, i32), area: RECT) -> bool {
    x >= area.left && x < area.right && y >= area.bottom - 1
}

/// Decides when an auto-hidden dock is on screen.
#[derive(Default)]
struct AutoHide {
    revealed: bool,
    left_at: Option<Instant>,
}

impl AutoHide {
    /// Returns true when the dock has to slide in or out.
    fn update(&mut self, at_edge: bool, over_dock: bool) -> bool {
        if !self.revealed {
            self.revealed = at_edge;
            self.left_at = None;
            return at_edge;
        }
        if at_edge || over_dock {
            self.left_at = None;
            return false;
        }
        let left_at = *self.left_at.get_or_insert_with(Instant::now);
        if left_at.elapsed() < DOCK_HIDE_DELAY {
            return false;
        }
        self.reset();
        true
    }

    fn reset(&mut self) {
        self.revealed = false;
        self.left_at = None;
    }
}

/// Reads widget data only while the dock is on screen, so an auto-hidden dock
/// costs nothing, and refreshes the moment it slides in.
fn spawn_widget_poller(app: AppHandle) {
    thread::spawn(move || {
        widgets::init_thread();
        let mut collector = widgets::Collector::default();
        let mut last_read: Option<Instant> = None;

        loop {
            thread::sleep(WIDGET_POLL);

            let state = app.state::<DockState>();
            if !state.dock_on_screen.load(Ordering::Relaxed) {
                last_read = None;
                continue;
            }
            let stale = state.widgets_stale.swap(false, Ordering::Relaxed);
            if !stale && last_read.is_some_and(|read| read.elapsed() < WIDGET_REFRESH) {
                continue;
            }

            let wanted = state.widgets.lock().unwrap().clone();
            let data = collector.collect(&wanted);
            last_read = Some(Instant::now());

            let mut current = state.widget_data.lock().unwrap();
            if *current != data {
                *current = data.clone();
                let _ = app.emit("widgets-changed", data);
            }
        }
    });
}

/// Background loop that does what the page cannot: a click-through window
/// receives no mouse events, so the cursor has to be tracked from here.
fn spawn_monitor(app: AppHandle) {
    thread::spawn(move || {
        let own_pid = std::process::id();
        let mut dock: Option<(WebviewWindow, isize)> = None;
        let mut shown = false;
        let mut hovering = false;
        let mut tick = 0u32;
        let mut active = None;
        let mut placed = None;
        let mut fullscreen = false;
        let mut auto_hide_state = AutoHide::default();
        // Starts true so a taskbar left hidden by a killed dock comes back.
        let mut taskbar_hidden = true;

        loop {
            thread::sleep(CURSOR_POLL);
            tick = tick.wrapping_add(1);

            let state = app.state::<DockState>();
            let enabled = state.enabled.load(Ordering::Relaxed);

            // Checked on every poll because opening Start brings the taskbar back.
            if enabled && state.hide_taskbar.load(Ordering::Relaxed) {
                win::set_taskbar_visible(false);
                taskbar_hidden = true;
            } else if taskbar_hidden {
                win::set_taskbar_visible(true);
                taskbar_hidden = false;
            }

            if !enabled {
                state.dock_on_screen.store(false, Ordering::Relaxed);
                // Closing the page, not just hiding it, is what frees the
                // memory held by WebView2. It is created again when enabled.
                if let Some((window, _)) = dock.take() {
                    let _ = window.destroy();
                    *state.layout.lock().unwrap() = None;
                    state.dragging.store(false, Ordering::Relaxed);
                    shown = false;
                    hovering = false;
                    placed = None;
                    fullscreen = false;
                    auto_hide_state.reset();
                }
                continue;
            }

            if dock.is_none() {
                // The previous window may still be on its way out.
                if app.get_webview_window(DOCK_WINDOW).is_some() {
                    continue;
                }
                let created = create_dock_window(&app).and_then(|window| {
                    let hwnd = window.hwnd()?.0 as isize;
                    Ok((window, hwnd))
                });
                let Ok((window, hwnd)) = created else {
                    thread::sleep(DOCK_RETRY);
                    continue;
                };
                win::set_dock_style(hwnd, true);
                dock = Some((window, hwnd));
            }
            let Some((window, hwnd)) = dock.clone() else {
                continue;
            };

            if tick % TICKS_PER_SNAPSHOT == 0 {
                fullscreen = win::is_fullscreen_app_active(own_pid);
            }
            if fullscreen {
                state.dock_on_screen.store(false, Ordering::Relaxed);
                if shown {
                    let _ = window.hide();
                    shown = false;
                    hovering = false;
                    win::set_dock_style(hwnd, true);
                    let _ = app.emit("dock-hover", false);
                    auto_hide_state.reset();
                    let _ = app.emit("dock-reveal", false);
                }
                continue;
            }

            let Some(layout) = *state.layout.lock().unwrap() else {
                continue;
            };
            let Some(area) = win::work_area() else {
                continue;
            };
            let scale = window.scale_factor().unwrap_or(1.0);

            let target = (area, (layout.window_height * scale).ceil() as i32);
            if placed != Some(target) && place_window(&window, area, target.1 as f64).is_ok() {
                placed = Some(target);
            }
            if !shown && placed.is_some() {
                let _ = window.show();
                win::set_dock_style(hwnd, true);
                shown = true;
            }

            if let Some(cursor) = win::cursor_pos() {
                let auto_hide = state.auto_hide.load(Ordering::Relaxed);
                if !auto_hide && auto_hide_state.revealed {
                    auto_hide_state.reset();
                    let _ = app.emit("dock-reveal", false);
                }

                // A dock that is slid away must let every click through.
                let on_screen = !auto_hide || auto_hide_state.revealed;
                state.dock_on_screen.store(on_screen, Ordering::Relaxed);
                // An icon being dragged may leave the bar and must keep the mouse.
                let over = state.dragging.load(Ordering::Relaxed)
                    || (on_screen
                        && is_over_dock(cursor, area, layout, scale, hovering, auto_hide));
                if over != hovering {
                    hovering = over;
                    win::set_dock_style(hwnd, !over);
                    let _ = app.emit("dock-hover", over);
                }

                if auto_hide && auto_hide_state.update(is_at_bottom_edge(cursor, area), over) {
                    let _ = app.emit("dock-reveal", auto_hide_state.revealed);
                }
            }

            if tick % TICKS_PER_SNAPSHOT == 0 {
                // Showing or moving the window can make Tauri restore its styles.
                win::set_dock_style(hwnd, !hovering);

                // Clicking the dock must not make the app the user was in
                // look inactive, so the dock's own focus is skipped here.
                active = win::foreground_window(own_pid).or(active);
                let windows = win::list_windows(own_pid, active);
                let mut current = state.windows.lock().unwrap();
                if *current != windows {
                    *current = windows.clone();
                    let _ = app.emit("windows-changed", windows);
                }
            }
        }
    });
}

fn create_dock_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .expect("the dock window is declared in tauri.conf.json");
    WebviewWindowBuilder::from_config(app, config)?.build()
}

fn build_tray(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    let toggle = CheckMenuItem::with_id(app, "toggle", "Dock ativa", true, enabled, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Personalizar...", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
    *app.state::<DockState>().tray_toggle.lock().unwrap() = Some(toggle);

    let mut tray = TrayIconBuilder::new()
        .tooltip("Dock JM")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_enabled(app),
            "settings" => request_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Opening the program again brings up the settings, the only way back
        // to the dock while it is disabled and the tray icon is out of reach.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            request_settings(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(DockState::default())
        .setup(|app| {
            let config = config::load(app.handle()).ok();
            let enabled = config.as_ref().is_none_or(|config| config.enabled);
            let state = app.state::<DockState>();
            state.enabled.store(enabled, Ordering::Relaxed);
            if let Some(config) = config {
                state.hide_taskbar.store(config.hide_taskbar, Ordering::Relaxed);
                state.auto_hide.store(config.auto_hide, Ordering::Relaxed);
                *state.widgets.lock().unwrap() = config.enabled_widgets();
                state
                    .start_with_windows
                    .store(config.start_with_windows, Ordering::Relaxed);
                // Written again so the entry follows the program if it was moved.
                if config.start_with_windows {
                    let _ = win::set_autostart(true);
                }
            }
            build_tray(app.handle(), enabled)?;
            spawn_monitor(app.handle().clone());
            spawn_widget_poller(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            save_dock_order,
            set_dragging,
            get_windows,
            get_icon,
            set_layout,
            launch_app,
            focus_window,
            minimize_window,
            open_start_menu,
            open_settings,
            open_config_file,
            quit_app,
            open_quick_settings,
            open_calendar,
            open_task_manager,
            get_widget_data,
            set_volume,
            media_action
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // A disabled dock has no windows left, which must not end the program.
            RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            RunEvent::Exit => {
                // Stops the monitor first, or it could hide the taskbar again.
                app.state::<DockState>()
                    .hide_taskbar
                    .store(false, Ordering::Relaxed);
                thread::sleep(CURSOR_POLL * 2);
                win::set_taskbar_visible(true);
            }
            _ => {}
        });
}
