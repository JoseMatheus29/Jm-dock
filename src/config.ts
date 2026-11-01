export interface Theme {
  barColor: string;
  barOpacity: number;
  borderColor: string;
  borderOpacity: number;
  radius: number;
  padding: number;
  gap: number;
  dotColor: string;
  activeColor: string;
  labelBackground: string;
  labelColor: string;
  widgetColor: string;
  useGradient: boolean;
  barColor2: string;
  gradientAngle: number;
  borderWidth: number;
  pattern: string;
  patternColor: string;
  patternOpacity: number;
  patternSize: number;
  glowColor: string;
  glowSize: number;
  font: string;
  barImage: string;
}

export interface SavedTheme {
  name: string;
  theme: Theme;
}

export interface PinnedApp {
  name: string;
  path: string;
}

export type Alignment = "left" | "center" | "right";
export type Edge = "bottom" | "top" | "left" | "right";
export type MagnifyStyle = "wave" | "single" | "off";

export interface DockConfig {
  enabled: boolean;
  showStartButton: boolean;
  hideTaskbar: boolean;
  autoHide: boolean;
  startWithWindows: boolean;
  iconSize: number;
  magnification: number;
  magnifyStyle: MagnifyStyle;
  edge: Edge;
  alignment: Alignment;
  edgeGap: number;
  theme: Theme;
  savedThemes: SavedTheme[];
  customCss: string;
  apps: PinnedApp[];
  appOrder: string[];
  widgets: WidgetSetting[];
  clock: ClockOptions;
}

export type WidgetId =
  | "media"
  | "system"
  | "network"
  | "volume"
  | "battery"
  | "clock";

export interface WidgetSetting {
  id: WidgetId;
  enabled: boolean;
}

export interface ClockOptions {
  showSeconds: boolean;
  showDate: boolean;
  hour12: boolean;
}

export interface WidgetData {
  volume: { level: number; muted: boolean } | null;
  network: { kind: "wifi" | "wired" | "offline"; bars: number; name: string } | null;
  battery: { percent: number; charging: boolean } | null;
  system: { cpu: number; memory: number } | null;
  media: {
    title: string;
    artist: string;
    playing: boolean;
    thumbnail: string | null;
  } | null;
}

export interface AppWindow {
  hwnd: number;
  title: string;
  exe: string;
  active: boolean;
}

