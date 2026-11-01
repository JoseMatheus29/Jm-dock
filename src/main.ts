import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  applyAppearance,
  AppWindow,
  DockConfig,
  PRESETS,
  WidgetData,
} from "./config";
import { WidgetBar } from "./widgets";

interface DockItem {
  key: string;
  label: string;
  path: string;
  windows: AppWindow[];
}

/** Room above the magnified icons for the name label. */
const LABEL_SPACE = 40;
const SETTLE_MS = 180;
const ERROR_MS = 6000;
const START_KEY = "start";
const START_LOGO = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3h8.5v8.5H3zM12.5 3H21v8.5h-8.5zM3 12.5h8.5V21H3zM12.5 12.5H21V21h-8.5z"/></svg>`;

const dock = document.querySelector<HTMLElement>("#dock")!;
const errorEl = document.querySelector<HTMLElement>("#error")!;
const separator = document.createElement("div");
separator.className = "separator";
const widgetBar = new WidgetBar(showError, () => {
  widgetBar.element.hidden = !widgetBar.hasVisibleWidgets();
});

let config: DockConfig = {
  enabled: true,
  showStartButton: true,
  hideTaskbar: false,
  autoHide: false,
  iconSize: 52,
  magnification: 1.6,
  alignment: "center",
  bottomGap: 8,
  theme: PRESETS[0].theme,
  savedThemes: [],
  customCss: "",
  apps: [],
  widgets: [],
  clock: { showSeconds: false, showDate: true, hour12: false },
};
let windows: AppWindow[] = [];
/** Open apps that are not pinned, in order of appearance so they never reshuffle. */
let runningOrder: string[] = [];
let items: DockItem[] = [];
let pinnedCount = 0;
let restCenters: number[] = [];
let cursorX: number | null = null;
let revealed = false;
let settleTimer: number | undefined;
let errorTimer: number | undefined;

const iconEls = new Map<string, HTMLButtonElement>();

function appKey(path: string): string {
  return path.slice(path.lastIndexOf("\\") + 1).toLowerCase();
}

function appName(exe: string): string {
  const file = exe.slice(exe.lastIndexOf("\\") + 1);
  return file.replace(/\.exe$/i, "");
}

function showError(error: unknown) {
  errorEl.textContent = String(error);
  errorEl.hidden = false;
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => (errorEl.hidden = true), ERROR_MS);
}

function buildItems(): DockItem[] {
  const byApp = new Map<string, AppWindow[]>();
  for (const window of windows) {
    const key = appKey(window.exe);
    byApp.set(key, [...(byApp.get(key) ?? []), window]);
  }

  const start: DockItem[] = config.showStartButton
    ? [{ key: START_KEY, label: "Menu Iniciar", path: "", windows: [] }]
    : [];
  const pinned = config.apps.map((app) => ({
    key: appKey(app.path),
    label: app.name,
    path: app.path,
    windows: byApp.get(appKey(app.path)) ?? [],
  }));
  const pinnedKeys = new Set(pinned.map((item) => item.key));
  pinnedCount = start.length + pinned.length;

  runningOrder = runningOrder.filter((key) => byApp.has(key));
  for (const key of byApp.keys()) {
    if (!pinnedKeys.has(key) && !runningOrder.includes(key)) {
      runningOrder.push(key);
    }
  }

  const running = runningOrder
    .filter((key) => !pinnedKeys.has(key))
    .map((key) => {
      const appWindows = byApp.get(key)!;
      return {
        key,
        label: appName(appWindows[0].exe),
        path: appWindows[0].exe,
        windows: appWindows,
      };
    });

  return [...start, ...pinned, ...running];
}

function createIcon(item: DockItem): HTMLButtonElement {
  const { key } = item;
  const button = document.createElement("button");
  button.className = "icon";
  button.innerHTML = `<span class="fallback"></span><span class="dot"></span><span class="label"></span>`;
  button.addEventListener("click", () => {
    const current = items.find((candidate) => candidate.key === key);
    if (current) activate(current, button).catch(showError);
  });
  iconEls.set(key, button);

  const fallback = button.querySelector<HTMLElement>(".fallback")!;
  if (key === START_KEY) {
    fallback.classList.add("start");
    fallback.innerHTML = START_LOGO;
  } else {
    fallback.textContent = item.label.charAt(0).toUpperCase();
    loadIcon(button, item.path).catch(showError);
  }
  return button;
}

async function loadIcon(button: HTMLButtonElement, path: string) {
  const src = await invoke<string | null>("get_icon", { path });
  if (!src) return;
  const img = document.createElement("img");
  img.src = src;
  img.alt = "";
  button.querySelector(".fallback")?.replaceWith(img);
}

/** Launches the app, or brings up, cycles or minimizes its windows. */
async function activate(item: DockItem, button: HTMLButtonElement) {
  if (item.key === START_KEY) {
    await invoke("open_start_menu");
    return;
  }

  const appWindows = item.windows;
  if (appWindows.length === 0) {
    button.classList.remove("launching");
    void button.offsetWidth;
    button.classList.add("launching");
    await invoke("launch_app", { path: item.path });
    return;
  }

  // The list is ordered topmost first, so the last one is the least recent.
  const isActive = appWindows.some((window) => window.active);
  if (isActive && appWindows.length === 1) {
    await invoke("minimize_window", { hwnd: appWindows[0].hwnd });
  } else {
    const target = isActive ? appWindows[appWindows.length - 1] : appWindows[0];
    await invoke("focus_window", { hwnd: target.hwnd });
  }
}

function render() {
  items = buildItems();
  const keys = new Set(items.map((item) => item.key));
  for (const [key, button] of iconEls) {
    if (!keys.has(key)) {
      button.remove();
      iconEls.delete(key);
    }
  }

  const wanted: HTMLElement[] = [];
  items.forEach((item, index) => {
    if (index === pinnedCount && index > 0) wanted.push(separator);

    const button = iconEls.get(item.key) ?? createIcon(item);
    button.classList.toggle("running", item.windows.length > 0);
    button.classList.toggle(
      "active",
      item.windows.some((window) => window.active),
    );
    button.querySelector(".label")!.textContent = item.label;
    wanted.push(button);
  });
  wanted.push(widgetBar.element);

  const current = Array.from(dock.children);
  const changed =
    current.length !== wanted.length ||
    wanted.some((element, index) => element !== current[index]);
  if (changed) {
    dock.replaceChildren(...wanted);
    measure();
  }
}

function iconButtons(): HTMLButtonElement[] {
  return items.map((item) => iconEls.get(item.key)!);
}

function setSize(button: HTMLElement, size: number) {
  button.style.width = `${size}px`;
  button.style.height = `${size}px`;
}

/** Takes the resting geometry and hands it to Rust for click-through testing. */
function measure() {
  const base = config.iconSize;
  const peak = base * config.magnification;
  const buttons = iconButtons();

  dock.classList.remove("settling");
  // A fixed height lets magnified icons rise out of the bar instead of growing it.
  dock.style.height = `${base}px`;
  buttons.forEach((button) => setSize(button, base));
  const rect = dock.getBoundingClientRect();
  restCenters = buttons.map((button) => {
    const box = button.getBoundingClientRect();
    return box.left + box.width / 2;
  });
  magnify();

  invoke("set_layout", {
    layout: {
      x: rect.left,
      width: rect.width,
      height: rect.height,
      // Read from the config because rect.bottom moves with the slide animation.
      bottom: config.bottomGap,
      extra: (peak - base) * 2,
      windowHeight: Math.ceil(
        peak + LABEL_SPACE + rect.height - base + config.bottomGap,
      ),
    },
  }).catch(showError);
}

/**
 * Sizes icons by their distance to the cursor. Distances use the resting
 * centers, since measuring the moving icons would feed back into itself.
 */
function magnify() {
  if (cursorX === null) return;
  const x = cursorX;
  const base = config.iconSize;
  const peak = base * config.magnification;
  const range = base * 2.5;
  iconButtons().forEach((button, index) => {
    const distance = Math.abs(x - restCenters[index]);
    const strength =
      distance < range ? (Math.cos((Math.PI * distance) / range) + 1) / 2 : 0;
    setSize(button, base + (peak - base) * strength);
  });
}

function settle() {
  cursorX = null;
  dock.classList.add("settling");
  iconButtons().forEach((button) => setSize(button, config.iconSize));
  clearTimeout(settleTimer);
  settleTimer = setTimeout(() => dock.classList.remove("settling"), SETTLE_MS);
}

function updateSlide() {
  dock.classList.toggle("slid-away", config.autoHide && !revealed);
}

function applyConfig(next: DockConfig) {
  config = next;
  updateSlide();
  try {
    applyAppearance(config);
  } catch (error) {
    showError(error);
  }
  render();
  widgetBar.applyConfig(config);
  measure();
}

document.addEventListener("mousemove", (event) => {
  clearTimeout(settleTimer);
  dock.classList.remove("settling");
  cursorX = event.clientX;
  magnify();
});
document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  invoke("open_settings").catch(showError);
});
window.addEventListener("resize", measure);
// A widget changing width moves the icons, whose positions Rust and magnify rely on.
new ResizeObserver(measure).observe(widgetBar.element);

await Promise.all([
  listen<boolean>("dock-hover", ({ payload: hovering }) => {
    if (!hovering) settle();
  }),
  listen<boolean>("dock-reveal", ({ payload }) => {
    revealed = payload;
    updateSlide();
  }),
  listen<AppWindow[]>("windows-changed", ({ payload }) => {
    windows = payload;
    render();
  }),
  listen<DockConfig>("config-changed", ({ payload }) => applyConfig(payload)),
  listen<WidgetData>("widgets-changed", ({ payload }) =>
    widgetBar.update(payload),
  ),
]);

windows = await invoke<AppWindow[]>("get_windows");
try {
  applyConfig(await invoke<DockConfig>("get_config"));
} catch (error) {
  applyConfig(config);
  showError(error);
}
widgetBar.update(await invoke<WidgetData>("get_widget_data"));
