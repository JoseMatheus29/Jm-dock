import { Theme } from "./config";

type Hsl = { h: number; s: number; l: number };

export interface Palette {
  dark: boolean;
  background: string;
  text: string;
  variables: Record<string, string>;
}

/** Below this contrast with the cards, an accent gets hard to see. */
const MIN_ACCENT_CONTRAST = 3.5;
const ACCENT_STEP = 0.03;
/** Bars brighter than this get a light window. */
const LIGHT_BAR_LUMINANCE = 0.5;

function channels(hex: string): [number, number, number] {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!match) return [0, 0, 0];
  const [red, green, blue] = match.slice(1).map((part) => parseInt(part, 16) / 255);
  return [red, green, blue];
}

function toHsl(hex: string): Hsl {
  const [red, green, blue] = channels(hex);
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  const l = (max + min) / 2;
  if (delta === 0) return { h: 0, s: 0, l };
  const s = delta / (1 - Math.abs(2 * l - 1));
  const sector =
    max === red
      ? ((green - blue) / delta + 6) % 6
      : max === green
        ? (blue - red) / delta + 2
        : (red - green) / delta + 4;
  return { h: sector * 60, s, l };
}

function toHex({ h, s, l }: Hsl): string {
  const chroma = (1 - Math.abs(2 * l - 1)) * s;
  const channel = (offset: number) => {
    const k = (offset + h / 30) % 12;
    const value = l - chroma * 0.5 * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(value * 255)
      .toString(16)
      .padStart(2, "0");
  };
  return `#${channel(0)}${channel(8)}${channel(4)}`;
}

function luminance(hex: string): number {
  const [red, green, blue] = channels(hex).map((value) =>
    value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(first: string, second: string): number {
  const a = luminance(first);
  const b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function isColorful(hex: string): boolean {
  const { s, l } = toHsl(hex);
  return s > 0.25 && l > 0.12 && l < 0.94;
}

/** The theme's most colorful highlight, moved until it reads on the cards. */
function accentFor(theme: Theme, surface: string, dark: boolean): string {
  const source =
    [theme.activeColor, theme.barColor, theme.borderColor].find(isColorful) ??
    theme.activeColor;
  const color = toHsl(source);
  while (contrast(toHex(color), surface) < MIN_ACCENT_CONTRAST) {
    color.l += dark ? ACCENT_STEP : -ACCENT_STEP;
    if (color.l <= 0 || color.l >= 1) break;
  }
  return toHex(color);
}

/** Colors for the settings window, derived from the bar's theme. */
export function settingsPalette(theme: Theme): Palette {
  const brightness = theme.useGradient
    ? (luminance(theme.barColor) + luminance(theme.barColor2)) / 2
    : luminance(theme.barColor);
  const dark = brightness <= LIGHT_BAR_LUMINANCE;

  const { h, s } = toHsl(theme.barColor);
  const tone = (saturation: number, l: number) =>
    toHex({ h, s: Math.min(s, saturation), l });

  const background = dark ? tone(0.38, 0.09) : tone(0.6, 0.945);
  const surface = dark ? tone(0.34, 0.13) : tone(0.6, 0.985);
  const field = dark ? tone(0.3, 0.175) : tone(0.5, 0.915);
  const line = dark ? tone(0.26, 0.23) : tone(0.4, 0.855);
  const text = dark ? tone(0.2, 0.95) : tone(0.4, 0.12);
  const muted = dark ? tone(0.16, 0.7) : tone(0.2, 0.36);
  const accent = accentFor(theme, surface, dark);
  const accentText = contrast(accent, "#ffffff") >= contrast(accent, "#101014")
    ? "#ffffff"
    : "#101014";

  return {
    dark,
    background,
    text,
    variables: {
      "--bg": background,
      "--surface": surface,
      "--field": field,
      "--line": line,
      "--text": text,
      "--muted": muted,
      "--accent": accent,
      "--accent-text": accentText,
      "--danger": dark ? "#ff9d9d" : "#b91c1c",
    },
  };
}
