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

export interface DockConfig {
  enabled: boolean;
  showStartButton: boolean;
  hideTaskbar: boolean;
  autoHide: boolean;
  iconSize: number;
  magnification: number;
  alignment: Alignment;
  bottomGap: number;
  theme: Theme;
  savedThemes: SavedTheme[];
  customCss: string;
  apps: PinnedApp[];
}

export interface AppWindow {
  hwnd: number;
  title: string;
  exe: string;
  active: boolean;
}

export const PRESETS: SavedTheme[] = [
  {
    name: "Grafite",
    theme: {
      barColor: "#18181c",
      barOpacity: 0.72,
      borderColor: "#ffffff",
      borderOpacity: 0.16,
      radius: 22,
      padding: 9,
      gap: 6,
      dotColor: "#ffffff",
      activeColor: "#6ee7f9",
      labelBackground: "#18181c",
      labelColor: "#f4f4f5",
    },
  },
  {
    name: "Vidro claro",
    theme: {
      barColor: "#f5f6fa",
      barOpacity: 0.62,
      borderColor: "#ffffff",
      borderOpacity: 0.7,
      radius: 26,
      padding: 9,
      gap: 7,
      dotColor: "#3f3f46",
      activeColor: "#2563eb",
      labelBackground: "#f5f6fa",
      labelColor: "#18181b",
    },
  },
  {
    name: "Meia-noite",
    theme: {
      barColor: "#0b1533",
      barOpacity: 0.86,
      borderColor: "#7aa2ff",
      borderOpacity: 0.34,
      radius: 18,
      padding: 8,
      gap: 6,
      dotColor: "#a5b8ff",
      activeColor: "#ffd166",
      labelBackground: "#0b1533",
      labelColor: "#e6ebff",
    },
  },
  {
    name: "Aurora",
    theme: {
      barColor: "#10302b",
      barOpacity: 0.8,
      borderColor: "#5eead4",
      borderOpacity: 0.4,
      radius: 24,
      padding: 9,
      gap: 6,
      dotColor: "#99f6e4",
      activeColor: "#a3e635",
      labelBackground: "#10302b",
      labelColor: "#ecfdf5",
    },
  },
  {
    name: "Sakura",
    theme: {
      barColor: "#3a1526",
      barOpacity: 0.78,
      borderColor: "#fda4c8",
      borderOpacity: 0.42,
      radius: 28,
      padding: 9,
      gap: 7,
      dotColor: "#fecdd3",
      activeColor: "#fb7185",
      labelBackground: "#3a1526",
      labelColor: "#fff1f5",
    },
  },
  {
    name: "Minimalista",
    theme: {
      barColor: "#000000",
      barOpacity: 0,
      borderColor: "#ffffff",
      borderOpacity: 0,
      radius: 0,
      padding: 6,
      gap: 10,
      dotColor: "#ffffff",
      activeColor: "#ffffff",
      labelBackground: "#000000",
      labelColor: "#ffffff",
    },
  },
];

/** Turns "#rrggbb" into rgba(), leaving anything else as written. */
export function withAlpha(color: string, alpha: number): string {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color);
  if (!match) return color;
  const [red, green, blue] = match.slice(1).map((part) => parseInt(part, 16));
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

const customSheet = new CSSStyleSheet();

/** Applies the appearance settings to the dock page. */
export function applyAppearance(config: DockConfig) {
  const { theme } = config;
  const root = document.documentElement.style;
  root.setProperty("--bar-bg", withAlpha(theme.barColor, theme.barOpacity));
  root.setProperty(
    "--bar-border",
    withAlpha(theme.borderColor, theme.borderOpacity),
  );
  root.setProperty("--bar-radius", `${theme.radius}px`);
  root.setProperty("--bar-padding", `${theme.padding}px`);
  root.setProperty("--icon-gap", `${theme.gap}px`);
  root.setProperty("--dot", theme.dotColor);
  root.setProperty("--dot-active", theme.activeColor);
  root.setProperty("--label-bg", theme.labelBackground);
  root.setProperty("--label-text", theme.labelColor);
  root.setProperty("--bottom-gap", `${config.bottomGap}px`);
  root.setProperty(
    "--stage-align",
    { left: "flex-start", center: "center", right: "flex-end" }[
      config.alignment
    ] ?? "center",
  );

  customSheet.replaceSync(config.customCss);
  document.adoptedStyleSheets = [customSheet];
}
