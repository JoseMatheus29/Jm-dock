//! Reads what the dock's widgets show. Each source fails on its own, so a
//! machine without a battery or a playing app just leaves that widget empty.

use std::time::{Duration, Instant};

use base64::Engine;
use serde::Serialize;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as MediaManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
use windows::Storage::Streams::{DataReader, IInputStream, IRandomAccessStreamWithContentType};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::core::Interface;

use crate::config::WidgetId;

/// The network barely changes and is the slowest source to ask.
const NETWORK_REFRESH: Duration = Duration::from_secs(5);
const MAX_THUMBNAIL_BYTES: u64 = 2 * 1024 * 1024;
const NO_BATTERY: u8 = 128;
const UNKNOWN_PERCENT: u8 = 255;

#[derive(Clone, Default, PartialEq, Serialize)]
pub struct WidgetData {
    pub volume: Option<VolumeData>,
    pub network: Option<NetworkData>,
    pub battery: Option<BatteryData>,
    pub system: Option<SystemData>,
    pub media: Option<MediaData>,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct VolumeData {
    pub level: f32,
    pub muted: bool,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct NetworkData {
    pub kind: &'static str,
    pub bars: u8,
    pub name: String,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct BatteryData {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct SystemData {
    pub cpu: u8,
    pub memory: u8,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct MediaData {
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub thumbnail: Option<String>,
}

/// Keeps what has to survive between reads, such as the previous CPU sample.
#[derive(Default)]
pub struct Collector {
    media_manager: Option<MediaManager>,
    cpu_sample: Option<(u64, u64)>,
    network: Option<(Instant, NetworkData)>,
    thumbnail: Option<(String, Option<String>)>,
}

impl Collector {
    pub fn collect(&mut self, wanted: &[WidgetId]) -> WidgetData {
        let mut data = WidgetData::default();
        for id in wanted {
            match id {
                WidgetId::Volume => data.volume = read_volume(),
                WidgetId::Network => data.network = self.read_network(),
                WidgetId::Battery => data.battery = read_battery(),
                WidgetId::System => data.system = self.read_system(),
                WidgetId::Media => data.media = self.read_media(),
                WidgetId::Clock => {}
            }
        }
        data
    }

    fn read_network(&mut self) -> Option<NetworkData> {
        if let Some((read_at, network)) = &self.network {
            if read_at.elapsed() < NETWORK_REFRESH {
                return Some(network.clone());
            }
        }
        let network = read_network();
        self.network = Some((Instant::now(), network.clone()));
        Some(network)
    }

    fn read_system(&mut self) -> Option<SystemData> {
        let (mut idle, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;

        // Kernel time already includes the idle time.
        let idle = filetime_ticks(idle);
        let total = filetime_ticks(kernel) + filetime_ticks(user);
        let cpu = match self.cpu_sample.replace((idle, total)) {
            Some((last_idle, last_total)) if total > last_total => {
                let busy = (total - last_total).saturating_sub(idle - last_idle);
                (busy * 100 / (total - last_total)) as u8
            }
            _ => 0,
        };

        let mut memory = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        unsafe { GlobalMemoryStatusEx(&mut memory) }.ok()?;
        Some(SystemData {
            cpu,
            memory: memory.dwMemoryLoad as u8,
        })
    }

    fn read_media(&mut self) -> Option<MediaData> {
        if self.media_manager.is_none() {
            self.media_manager = MediaManager::RequestAsync().ok()?.join().ok();
        }
        let session = self.media_manager.as_ref()?.GetCurrentSession().ok()?;
        let properties = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let title = properties.Title().ok()?.to_string();
        if title.is_empty() {
            return None;
        }
        let artist = properties.Artist().map(|a| a.to_string()).unwrap_or_default();
        let playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .is_ok_and(|status| status == PlaybackStatus::Playing);

        // Reading the cover is the costly part, so it only happens per track.
        let track = format!("{title}\n{artist}");
        let thumbnail = match &self.thumbnail {
            Some((known, thumbnail)) if *known == track => thumbnail.clone(),
            _ => {
                let thumbnail = properties.Thumbnail().ok().and_then(|cover| {
                    read_thumbnail(cover.OpenReadAsync().ok()?.join().ok()?)
                });
                self.thumbnail = Some((track, thumbnail.clone()));
                thumbnail
            }
        };

        Some(MediaData {
            title,
            artist,
            playing,
            thumbnail,
        })
    }
}

fn filetime_ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

fn read_thumbnail(stream: IRandomAccessStreamWithContentType) -> Option<String> {
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_THUMBNAIL_BYTES {
        return None;
    }
    let content_type = stream.ContentType().ok()?.to_string();
    let input: IInputStream = stream.cast().ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{content_type};base64,{encoded}"))
}

fn endpoint_volume() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate(CLSCTX_ALL, None)
    }
}

fn read_volume() -> Option<VolumeData> {
    let endpoint = endpoint_volume().ok()?;
    unsafe {
        Some(VolumeData {
            level: endpoint.GetMasterVolumeLevelScalar().ok()?,
            muted: endpoint.GetMute().ok()?.as_bool(),
        })
    }
}

fn read_network() -> NetworkData {
    let offline = NetworkData {
        kind: "offline",
        bars: 0,
        name: String::new(),
    };
    let Ok(profile) = NetworkInformation::GetInternetConnectionProfile() else {
        return offline;
    };
    let connected = profile.GetNetworkConnectivityLevel().is_ok_and(|level| {
        level == NetworkConnectivityLevel::InternetAccess
            || level == NetworkConnectivityLevel::ConstrainedInternetAccess
    });
    if !connected {
        return offline;
    }

    let wifi = profile.IsWlanConnectionProfile().unwrap_or(false);
    NetworkData {
        kind: if wifi { "wifi" } else { "wired" },
        bars: profile
            .GetSignalBars()
            .and_then(|bars| bars.Value())
            .unwrap_or(0),
        name: profile.ProfileName().map(|n| n.to_string()).unwrap_or_default(),
    }
}

fn read_battery() -> Option<BatteryData> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    if status.BatteryFlag & NO_BATTERY != 0 || status.BatteryLifePercent == UNKNOWN_PERCENT {
        return None;
    }
    Some(BatteryData {
        percent: status.BatteryLifePercent,
        charging: status.ACLineStatus == 1,
    })
}

/// Runs `action` with COM set up, for commands that use Tauri's worker threads.
fn with_com<T>(action: impl FnOnce() -> T) -> T {
    let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let result = action();
    if com.is_ok() {
        unsafe { CoUninitialize() };
    }
    result
}

/// Sets up COM for a thread that lives as long as the app.
pub fn init_thread() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}

pub fn set_volume(level: f32) -> Result<(), String> {
    with_com(|| {
        let endpoint = endpoint_volume().map_err(|e| e.to_string())?;
        unsafe { endpoint.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null()) }
            .map_err(|e| e.to_string())
    })
}

pub fn media_action(action: &str) -> Result<(), String> {
    with_com(|| {
        let run = || -> windows::core::Result<()> {
            let manager = MediaManager::RequestAsync()?.join()?;
            let session = manager.GetCurrentSession()?;
            let request = match action {
                "next" => session.TrySkipNextAsync()?,
                "previous" => session.TrySkipPreviousAsync()?,
                _ => session.TryTogglePlayPauseAsync()?,
            };
            request.join()?;
            Ok(())
        };
        run().map_err(|e| e.to_string())
    })
}
