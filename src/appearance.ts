import { DockConfig, Edge, SavedTheme, Theme } from "./config";

const BASE_THEME: Theme = {
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
  widgetColor: "#f4f4f5",
  useGradient: false,
  barColor2: "#18181c",
  gradientAngle: 135,
  borderWidth: 1,
  pattern: "none",
  patternColor: "#ffffff",
  patternOpacity: 0.12,
  patternSize: 24,
  glowColor: "#6ee7f9",
  glowSize: 0,
  font: "Segoe UI Variable",
  barImage: "",
};

const preset = (name: string, theme: Partial<Theme>): SavedTheme => ({
  name,
  theme: { ...BASE_THEME, ...theme },
});

export const PRESETS: SavedTheme[] = [
  preset("Grafite", {}),
  preset("Vidro claro", {
    barColor: "#f5f6fa",
    barOpacity: 0.62,
    borderOpacity: 0.7,
    radius: 26,
    gap: 7,
    dotColor: "#3f3f46",
    activeColor: "#2563eb",
    labelBackground: "#f5f6fa",
    labelColor: "#18181b",
    widgetColor: "#18181b",
  }),
  preset("Meia-noite", {
    barColor: "#0b1533",
    barOpacity: 0.86,
    borderColor: "#7aa2ff",
    borderOpacity: 0.34,
    radius: 18,
    padding: 8,
    dotColor: "#a5b8ff",
    activeColor: "#ffd166",
    labelBackground: "#0b1533",
    labelColor: "#e6ebff",
    widgetColor: "#e6ebff",
  }),
  preset("Aurora", {
    barColor: "#10302b",
    barOpacity: 0.8,
    borderColor: "#5eead4",
    borderOpacity: 0.4,
    radius: 24,
    dotColor: "#99f6e4",
    activeColor: "#a3e635",
    labelBackground: "#10302b",
    labelColor: "#ecfdf5",
    widgetColor: "#ecfdf5",
  }),
  preset("Sakura", {
    barColor: "#3a1526",
    barOpacity: 0.78,
    borderColor: "#fda4c8",
    borderOpacity: 0.42,
    radius: 28,
    gap: 7,
    dotColor: "#fecdd3",
    activeColor: "#fb7185",
    labelBackground: "#3a1526",
    labelColor: "#fff1f5",
    widgetColor: "#fff1f5",
  }),
  preset("Minimalista", {
    barColor: "#000000",
    barOpacity: 0,
    borderOpacity: 0,
    radius: 0,
    padding: 6,
    gap: 10,
    activeColor: "#ffffff",
    labelBackground: "#000000",
    labelColor: "#ffffff",
    widgetColor: "#ffffff",
  }),
  preset("Ninja", {
    barColor: "#f97316",
    barColor2: "#1a1a1a",
    useGradient: true,
    barOpacity: 0.95,
    gradientAngle: 120,
    borderColor: "#111111",
    borderOpacity: 1,
    borderWidth: 2,
    radius: 14,
    dotColor: "#ffffff",
    activeColor: "#facc15",
    labelBackground: "#111111",
    labelColor: "#fb923c",
    widgetColor: "#ffffff",
    pattern: "swirls",
    patternColor: "#000000",
    patternOpacity: 0.24,
    patternSize: 30,
    glowColor: "#f97316",
    glowSize: 8,
    font: "Bahnschrift",
  }),
  preset("Laço rosa", {
    barColor: "#ffc2dc",
    barColor2: "#fff1f7",
    useGradient: true,
    barOpacity: 0.96,
    gradientAngle: 180,
    borderColor: "#e6004c",
    borderOpacity: 0.9,
    borderWidth: 2,
    radius: 30,
    dotColor: "#e6004c",
    activeColor: "#e6004c",
    labelBackground: "#e6004c",
    labelColor: "#ffffff",
    widgetColor: "#8a0f3c",
    pattern: "bows",
    patternColor: "#e6004c",
    patternOpacity: 0.2,
    patternSize: 30,
    glowColor: "#ff7ab6",
    glowSize: 6,
    font: "Segoe Print",
  }),
  preset("Arcade neon", {
    barColor: "#12002b",
    barColor2: "#00203f",
    useGradient: true,
    barOpacity: 0.92,
    borderColor: "#22d3ee",
    borderOpacity: 0.9,
    radius: 12,
    dotColor: "#f0abfc",
    activeColor: "#22d3ee",
    labelBackground: "#12002b",
    labelColor: "#22d3ee",
    widgetColor: "#f0f9ff",
    pattern: "stripes",
    patternColor: "#d946ef",
    patternOpacity: 0.1,
    patternSize: 16,
    glowColor: "#d946ef",
    glowSize: 12,
    font: "Consolas",
  }),
  preset("Terminal retrô", {
    barColor: "#031a0b",
    barOpacity: 0.94,
    borderColor: "#22c55e",
    borderOpacity: 0.8,
    radius: 4,
    dotColor: "#86efac",
    activeColor: "#4ade80",
    labelBackground: "#031a0b",
    labelColor: "#4ade80",
    widgetColor: "#4ade80",
    pattern: "stripes",
    patternColor: "#22c55e",
    patternOpacity: 0.06,
    patternSize: 10,
    glowColor: "#22c55e",
    glowSize: 6,
    font: "Consolas",
  }),
  preset("Céu estrelado", {
    barColor: "#0b1033",
    barColor2: "#2a1a5e",
    useGradient: true,
    barOpacity: 0.92,
    borderColor: "#c7d2fe",
    borderOpacity: 0.35,
    radius: 26,
    dotColor: "#c7d2fe",
    activeColor: "#fde68a",
    labelBackground: "#0b1033",
    labelColor: "#fde68a",
    widgetColor: "#eef2ff",
    pattern: "stars",
    patternColor: "#ffffff",
    patternOpacity: 0.16,
    patternSize: 26,
  }),
  preset("Praia", {
    barColor: "#38bdf8",
    barColor2: "#fde68a",
    useGradient: true,
    barOpacity: 0.92,
    gradientAngle: 90,
    borderColor: "#ffffff",
    borderOpacity: 0.7,
    radius: 24,
    dotColor: "#0c4a6e",
    activeColor: "#0c4a6e",
    labelBackground: "#0c4a6e",
    labelColor: "#f0f9ff",
    widgetColor: "#0c4a6e",
    pattern: "dots",
    patternColor: "#ffffff",
    patternOpacity: 0.3,
    patternSize: 16,
  }),
];

/** Tiles drawn on a 24×24 grid. `{c}` is the pattern color. */
const PATTERNS: Record<string, string> = {
  dots: `<circle cx="12" cy="12" r="2.5" fill="{c}"/>`,
  stripes: `<path d="M-6 6 6-6M0 24 24 0M18 30 30 18" stroke="{c}" stroke-width="5"/>`,
  checker: `<path d="M0 0h12v12H0zM12 12h12v12H12z" fill="{c}"/>`,
  hearts: `<path d="M12 20s-7-4.600-7-10a4 4 0 0 1 7-2.600A4 4 0 0 1 19 10c0 5.400-7 10-7 10z" fill="{c}"/>`,
  stars: `<path d="m12 4 2.400 5.100 5.600.7-4.100 3.800 1.100 5.500L12 16.400 7 19.100l1.100-5.500L4 9.800l5.600-.7z" fill="{c}"/>`,
  sparks: `<path d="m12 3 2 7 7 2-7 2-2 7-2-7-7-2 7-2z" fill="{c}"/>`,
  swirls: `<path d="M12 12c0-1.500 2.200-1.500 2.200.4 0 2.300-3.600 2.800-5 .6C7.700 10.300 10 7 13.200 7.200c4 .3 5.900 4.500 4.300 8-1.900 4-7.900 4.900-11 1.600" fill="none" stroke="{c}" stroke-width="1.600" stroke-linecap="round"/>`,
  bows: `<path d="M12 12 5 8v8zM12 12l7-4v8z" fill="{c}"/><circle cx="12" cy="12" r="2" fill="{c}"/>`,
};

/** Turns "#rrggbb" into rgba(), leaving anything else as written. */
export function withAlpha(color: string, alpha: number): string {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color);
  if (!match) return color;
  const [red, green, blue] = match.slice(1).map((part) => parseInt(part, 16));
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function patternLayer(theme: Theme): string | null {
  const tile = PATTERNS[theme.pattern];
  if (!tile) return null;
  const color = withAlpha(theme.patternColor, theme.patternOpacity);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">${tile.split("{c}").join(color)}</svg>`;
  const url = `data:image/svg+xml,${encodeURIComponent(svg)}`;
  return `url("${url}") 0 0 / ${theme.patternSize}px ${theme.patternSize}px`;
}

/** The bar's CSS background: pattern on top of the color, on top of any picture. */
export function barBackground(theme: Theme): string {
  const first = withAlpha(theme.barColor, theme.barOpacity);
  const second = withAlpha(theme.barColor2, theme.barOpacity);
  const tint = theme.useGradient
    ? `linear-gradient(${theme.gradientAngle}deg, ${first}, ${second})`
    : `linear-gradient(${first}, ${first})`;
  const image = theme.barImage
    ? `url("${theme.barImage}") center / cover no-repeat`
    : null;
  return [patternLayer(theme), tint, image].filter(Boolean).join(", ");
}

const EDGES: Edge[] = ["bottom", "top", "left", "right"];
const customSheet = new CSSStyleSheet();

/** Applies the appearance settings to the dock page. */
export function applyAppearance(config: DockConfig) {
  const { theme } = config;
  const root = document.documentElement.style;
  root.setProperty("--bar-bg", barBackground(theme));
  root.setProperty("--bar-solid", withAlpha(theme.barColor, theme.barOpacity));
  root.setProperty(
    "--bar-border",
    withAlpha(theme.borderColor, theme.borderOpacity),
  );
  root.setProperty("--bar-border-width", `${theme.borderWidth}px`);
  root.setProperty("--bar-radius", `${theme.radius}px`);
  root.setProperty("--bar-padding", `${theme.padding}px`);
  root.setProperty(
    "--bar-glow",
    `0 0 ${theme.glowSize}px ${theme.glowSize > 0 ? theme.glowColor : "transparent"}`,
  );
  root.setProperty("--icon-gap", `${theme.gap}px`);
  root.setProperty("--dot", theme.dotColor);
  root.setProperty("--dot-active", theme.activeColor);
  root.setProperty("--label-bg", theme.labelBackground);
  root.setProperty("--label-text", theme.labelColor);
  root.setProperty("--widget-text", theme.widgetColor);
  root.setProperty("--icon-size", `${config.iconSize}px`);
  root.setProperty("--dock-font", `"${theme.font}", "Segoe UI", sans-serif`);
  root.setProperty("--edge-gap", `${config.edgeGap}px`);
  root.setProperty(
    "--stage-align",
    { left: "flex-start", center: "center", right: "flex-end" }[
      config.alignment
    ] ?? "center",
  );

  const classes = document.documentElement.classList;
  // Without a visible bar, text sits straight on whatever is behind the dock.
  classes.toggle("no-bar", theme.barOpacity < 0.25 && !theme.barImage);
  for (const edge of EDGES) classes.toggle(`edge-${edge}`, edge === config.edge);
  classes.toggle(
    "edge-vertical",
    config.edge === "left" || config.edge === "right",
  );

  customSheet.replaceSync(config.customCss);
  document.adoptedStyleSheets = [customSheet];
}
