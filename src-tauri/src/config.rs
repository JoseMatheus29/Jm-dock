use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DockConfig {
    pub enabled: bool,
    pub show_start_button: bool,
    pub hide_taskbar: bool,
    pub auto_hide: bool,
    pub start_with_windows: bool,
    pub icon_size: f64,
    pub magnification: f64,
    pub magnify_style: String,
    pub edge: Edge,
    pub alignment: String,
    #[serde(alias = "bottomGap")]
    pub edge_gap: f64,
    pub theme: Theme,
    pub saved_themes: Vec<SavedTheme>,
    pub custom_css: String,
    pub apps: Vec<PinnedApp>,
    /// Position of open apps that are not pinned, kept while they are closed.
    pub app_order: Vec<String>,
    pub widgets: Vec<WidgetSetting>,
    pub clock: ClockOptions,
}

/// Screen edge the dock sits on.
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetId {
    Media,
    System,
    Network,
    Volume,
    Battery,
    Clock,
}

impl WidgetId {
    const ALL: [WidgetId; 6] = [
        WidgetId::Media,
        WidgetId::System,
        WidgetId::Network,
        WidgetId::Volume,
        WidgetId::Battery,
        WidgetId::Clock,
    ];
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WidgetSetting {
    pub id: WidgetId,
    pub enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClockOptions {
    pub show_seconds: bool,
    pub show_date: bool,
    pub hour12: bool,
}

impl Default for ClockOptions {
    fn default() -> Self {
        Self {
            show_seconds: false,
            show_date: true,
            hour12: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    pub bar_color: String,
    pub bar_opacity: f64,
    pub border_color: String,
    pub border_opacity: f64,
    pub radius: f64,
    pub padding: f64,
    pub gap: f64,
    pub dot_color: String,
    pub active_color: String,
    pub label_background: String,
    pub label_color: String,
    pub widget_color: String,
    pub use_gradient: bool,
    pub bar_color2: String,
    pub gradient_angle: f64,
    pub border_width: f64,
    pub pattern: String,
    pub pattern_color: String,
    pub pattern_opacity: f64,
    pub pattern_size: f64,
    pub glow_color: String,
    pub glow_size: f64,
    pub font: String,
    /// A picture chosen by the user, as a data URL.
    pub bar_image: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedTheme {
    pub name: String,
    pub theme: Theme,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PinnedApp {
    pub name: String,
    pub path: String,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_start_button: true,
            hide_taskbar: false,
            auto_hide: false,
            start_with_windows: false,
            icon_size: 52.0,
            magnification: 1.6,
            magnify_style: "wave".into(),
            edge: Edge::default(),
            alignment: "center".into(),
            edge_gap: 8.0,
            theme: Theme::default(),
            saved_themes: Vec::new(),
            custom_css: String::new(),
            apps: default_apps(),
            app_order: Vec::new(),
            widgets: default_widgets(),
            clock: ClockOptions::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bar_color: "#18181c".into(),
            bar_opacity: 0.72,
            border_color: "#ffffff".into(),
            border_opacity: 0.16,
            radius: 22.0,
            padding: 9.0,
            gap: 6.0,
            dot_color: "#ffffff".into(),
            active_color: "#6ee7f9".into(),
            label_background: "#18181c".into(),
            label_color: "#f4f4f5".into(),
            widget_color: "#f4f4f5".into(),
            use_gradient: false,
            bar_color2: "#18181c".into(),
            gradient_angle: 135.0,
            border_width: 1.0,
            pattern: "none".into(),
            pattern_color: "#ffffff".into(),
            pattern_opacity: 0.12,
            pattern_size: 24.0,
            glow_color: "#6ee7f9".into(),
            glow_size: 0.0,
            font: "Segoe UI Variable".into(),
            bar_image: String::new(),
        }
    }
}

impl DockConfig {
    /// Keeps hand-edited values within what the layout can handle.
    fn normalize(&mut self) {
        self.icon_size = self.icon_size.clamp(24.0, 128.0);
        self.magnification = self.magnification.clamp(1.0, 3.0);
        self.edge_gap = self.edge_gap.clamp(0.0, 64.0);
        self.theme.border_width = self.theme.border_width.clamp(0.0, 6.0);
        self.theme.pattern_opacity = self.theme.pattern_opacity.clamp(0.0, 1.0);
        self.theme.pattern_size = self.theme.pattern_size.clamp(8.0, 80.0);
        self.theme.glow_size = self.theme.glow_size.clamp(0.0, 32.0);
        self.theme.bar_opacity = self.theme.bar_opacity.clamp(0.0, 1.0);
        self.theme.border_opacity = self.theme.border_opacity.clamp(0.0, 1.0);
        self.theme.radius = self.theme.radius.clamp(0.0, 64.0);
        self.theme.padding = self.theme.padding.clamp(0.0, 32.0);
        self.theme.gap = self.theme.gap.clamp(0.0, 32.0);

        // A hand-edited list may repeat widgets or miss ones added by an update.
        let mut widgets: Vec<WidgetSetting> = Vec::new();
        for widget in self.widgets.drain(..) {
            if !widgets.iter().any(|known| known.id == widget.id) {
                widgets.push(widget);
            }
        }
        for id in WidgetId::ALL {
            if !widgets.iter().any(|known| known.id == id) {
                widgets.push(WidgetSetting { id, enabled: false });
            }
        }
        self.widgets = widgets;
    }

    pub fn enabled_widgets(&self) -> Vec<WidgetId> {
        self.widgets
            .iter()
            .filter(|widget| widget.enabled)
            .map(|widget| widget.id)
            .collect()
    }
}

fn default_widgets() -> Vec<WidgetSetting> {
    let enabled = [WidgetId::Network, WidgetId::Volume, WidgetId::Clock];
    WidgetId::ALL
        .into_iter()
        .map(|id| WidgetSetting {
            id,
            enabled: enabled.contains(&id),
        })
        .collect()
}

fn default_apps() -> Vec<PinnedApp> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let candidates = [
        ("Explorador", format!(r"{windir}\explorer.exe")),
        (
            "Edge",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
        ),
        ("Bloco de Notas", format!(r"{windir}\System32\notepad.exe")),
        (
            "PowerShell",
            format!(r"{windir}\System32\WindowsPowerShell\v1.0\powershell.exe"),
        ),
    ];
    candidates
        .into_iter()
        .filter(|(_, path)| Path::new(path).exists())
        .map(|(name, path)| PinnedApp {
            name: name.into(),
            path,
        })
        .collect()
}

pub fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("dock.json"))
}

/// Reads the config, creating it on first run. A broken file is reported and
/// never overwritten, so the user's edits are not lost.
pub fn load(app: &AppHandle) -> Result<DockConfig, String> {
    let path = path(app)?;
    if !path.exists() {
        let config = DockConfig::default();
        save(app, &config)?;
        return Ok(config);
    }

    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: DockConfig =
        serde_json::from_str(&text).map_err(|e| format!("Erro no dock.json: {e}"))?;
    config.normalize();
    Ok(config)
}

pub fn save(app: &AppHandle, config: &DockConfig) -> Result<(), String> {
    let path = path(app)?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Validates a config coming from the settings window before it is stored.
pub fn store(app: &AppHandle, mut config: DockConfig) -> Result<DockConfig, String> {
    config.normalize();
    save(app, &config)?;
    Ok(config)
}
