import { invoke } from "@tauri-apps/api/core";
import { DockConfig, WidgetData, WidgetId } from "./config";

const VOLUME_STEP = 0.05;
const CLOCK_TICK_MS = 1000;
/** Fifths of a full signal, matching the five bars Windows reports. */
const MAX_BARS = 5;

const svg = (body: string) =>
  `<svg viewBox="0 0 24 24" aria-hidden="true">${body}</svg>`;

const ICONS = {
  speaker: svg(
    `<path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5z"/><path class="line" d="M15 9.5a3.5 3.5 0 0 1 0 5M17.5 7a7 7 0 0 1 0 10"/>`,
  ),
  muted: svg(
    `<path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5z"/><path class="line" d="m15.5 9.5 5 5m0-5-5 5"/>`,
  ),
  wired: svg(
    `<rect class="line" x="3" y="5" width="18" height="12" rx="2"/><path class="line" d="M9 20.500h6M12 17v3.500"/>`,
  ),
  offline: svg(
    `<path class="line" d="M3 3l18 18M2 9a15 15 0 0 1 5-3.2M22 9a15 15 0 0 0-9.500-3.900M5.500 12.500a10 10 0 0 1 3.500-2M18.500 12.500a10 10 0 0 0-2-1.400M9 16a5 5 0 0 1 6 0"/><circle cx="12" cy="19.500" r="1.300"/>`,
  ),
  play: svg(`<path d="M7 4.500v15l12-7.500z"/>`),
  pause: svg(`<path d="M6.500 4.500h4v15h-4zM13.500 4.500h4v15h-4z"/>`),
  next: svg(`<path d="M5 5v14l10-7zM16 5h3v14h-3z"/>`),
  previous: svg(`<path d="M19 5v14L9 12zM5 5h3v14H5z"/>`),
};

function wifiIcon(bars: number): string {
  // Arcs light up from the inside out as the signal gets stronger.
  const arcs = [
    "M9 16a4.500 4.500 0 0 1 6 0",
    "M6 12.800a9 9 0 0 1 12 0",
    "M3 9.500a13.500 13.500 0 0 1 18 0",
  ];
  const lit = Math.ceil((bars / MAX_BARS) * arcs.length);
  const paths = arcs
    .map(
      (arc, index) =>
        `<path class="line${index < lit ? "" : " dim"}" d="${arc}"/>`,
    )
    .join("");
  return svg(`${paths}<circle cx="12" cy="19.500" r="1.500"/>`);
}

function batteryIcon(percent: number, charging: boolean): string {
  const fill = Math.max(1, Math.round((percent / 100) * 13));
  const bolt = charging ? `<path class="bolt" d="M11.500 7 8 12.500h3L10 17l4-6h-3z"/>` : "";
  return svg(
    `<rect class="line" x="2.500" y="7" width="17" height="10" rx="2"/><path d="M20.500 10h1.500v4h-1.500z"/><rect x="4.500" y="9" width="${fill}" height="6" rx="0.500"/>${bolt}`,
  );
}

function setIcon(host: HTMLElement, markup: string) {
  // Skips the DOM write once a second when nothing changed.
  if (host.dataset.markup !== markup) {
    host.dataset.markup = markup;
    host.innerHTML = markup;
  }
}

function setText(host: Element, text: string) {
  if (host.textContent !== text) host.textContent = text;
}

function buildButton(className: string, inner: string, onClick: () => void) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `widget ${className}`;
  button.innerHTML = `${inner}<span class="label"></span>`;
  button.addEventListener("click", onClick);
  return button;
}

export class WidgetBar {
  readonly element = document.createElement("div");
  private readonly widgets: Record<WidgetId, HTMLElement>;
  private clock: DockConfig["clock"] = {
    showSeconds: false,
    showDate: true,
    hour12: false,
  };
  private volumeLevel = 0;

  constructor(
    private readonly onError: (error: unknown) => void,
    private readonly onVisibilityChange: () => void,
  ) {
    this.element.id = "widgets";
    const call = (command: string) => () => {
      invoke(command).catch(onError);
    };

    const volume = buildButton(
      "volume",
      `<span class="glyph"></span>`,
      call("open_quick_settings"),
    );
    volume.addEventListener(
      "wheel",
      (event) => {
        const step = event.deltaY < 0 ? VOLUME_STEP : -VOLUME_STEP;
        this.volumeLevel = Math.min(1, Math.max(0, this.volumeLevel + step));
        // The icon has no number, so the label answers the wheel right away.
        setText(
          volume.querySelector(".label")!,
          `Volume ${Math.round(this.volumeLevel * 100)}%`,
        );
        invoke("set_volume", { level: this.volumeLevel }).catch(onError);
      },
      { passive: true },
    );

    this.widgets = {
      clock: buildButton(
        "clock text",
        `<span class="main"></span><span class="sub"></span>`,
        call("open_calendar"),
      ),
      volume,
      network: buildButton(
        "network",
        `<span class="glyph"></span>`,
        call("open_quick_settings"),
      ),
      battery: buildButton(
        "battery",
        `<span class="glyph"></span>`,
        call("open_quick_settings"),
      ),
      system: buildButton(
        "system text",
        `<span class="main"></span><span class="sub"></span>`,
        call("open_task_manager"),
      ),
      media: this.buildMedia(),
    };

    this.tickClock();
    setInterval(() => this.tickClock(), CLOCK_TICK_MS);
  }

  private buildMedia(): HTMLElement {
    const media = document.createElement("div");
    media.className = "widget media capsule";
    media.innerHTML = `
      <img class="cover" alt="" hidden />
      <div class="track"><span class="main"></span><span class="sub"></span></div>
      <div class="controls"></div>`;
    const controls = media.querySelector(".controls")!;
    for (const [action, title] of [
      ["previous", "Anterior"],
      ["toggle", "Tocar ou pausar"],
      ["next", "Próxima"],
    ]) {
      const button = document.createElement("button");
      button.type = "button";
      button.dataset.action = action;
      button.title = title;
      button.setAttribute("aria-label", title);
      if (action !== "toggle") {
        button.innerHTML = ICONS[action as "next" | "previous"];
      }
      button.addEventListener("click", () => {
        invoke("media_action", { action }).catch(this.onError);
      });
      controls.append(button);
    }
    return media;
  }

  private tickClock() {
    const now = new Date();
    const { showSeconds, showDate, hour12 } = this.clock;
    const time = now.toLocaleTimeString("pt-BR", {
      hour: "2-digit",
      minute: "2-digit",
      second: showSeconds ? "2-digit" : undefined,
      hour12,
    });
    const date = now.toLocaleDateString("pt-BR", {
      weekday: "short",
      day: "2-digit",
      month: "2-digit",
    });

    const clock = this.widgets.clock;
    setText(clock.querySelector(".main")!, time);
    setText(clock.querySelector(".sub")!, showDate ? date : "");
    clock.querySelector<HTMLElement>(".sub")!.hidden = !showDate;
    setText(clock.querySelector(".label")!, now.toLocaleDateString("pt-BR", { dateStyle: "full" }));
  }

  /** Shows the selected widgets, in the selected order. */
  applyConfig(config: DockConfig) {
    this.clock = config.clock;
    this.tickClock();

    // Neighbours share one capsule; music is wide enough to stand on its own.
    const groups: HTMLElement[] = [];
    let capsule: HTMLElement | null = null;
    for (const { id, enabled } of config.widgets) {
      if (!enabled) continue;
      if (id === "media") {
        groups.push(this.widgets.media);
        capsule = null;
        continue;
      }
      if (!capsule) {
        capsule = document.createElement("div");
        capsule.className = "capsule";
        groups.push(capsule);
      }
      capsule.append(this.widgets[id]);
    }
    this.element.replaceChildren(...groups);
    this.syncCapsules();
    this.onVisibilityChange();
  }

  /** An empty capsule would still draw its background. */
  private syncCapsules() {
    for (const capsule of this.element.querySelectorAll<HTMLElement>(
      "div.capsule:not(.media)",
    )) {
      capsule.hidden = Array.from(capsule.children).every(
        (widget) => (widget as HTMLElement).hidden,
      );
    }
  }

  update(data: WidgetData) {
    const { volume, network, battery, system, media } = this.widgets;

    volume.hidden = !data.volume;
    if (data.volume) {
      this.volumeLevel = data.volume.level;
      const percent = `${Math.round(data.volume.level * 100)}%`;
      setIcon(
        volume.querySelector(".glyph")!,
        data.volume.muted ? ICONS.muted : ICONS.speaker,
      );
      setText(
        volume.querySelector(".label")!,
        data.volume.muted ? "Sem som" : `Volume ${percent}`,
      );
    }

    network.hidden = !data.network;
    if (data.network) {
      const { kind, bars, name } = data.network;
      const glyph = network.querySelector<HTMLElement>(".glyph")!;
      setIcon(
        glyph,
        kind === "wifi"
          ? wifiIcon(bars)
          : kind === "wired"
            ? ICONS.wired
            : ICONS.offline,
      );
      setText(
        network.querySelector(".label")!,
        kind === "offline" ? "Sem internet" : name || "Conectado",
      );
    }

    battery.hidden = !data.battery;
    if (data.battery) {
      const { percent, charging } = data.battery;
      setIcon(battery.querySelector(".glyph")!, batteryIcon(percent, charging));
      setText(
        battery.querySelector(".label")!,
        charging ? `Carregando, ${percent}%` : `Bateria ${percent}%`,
      );
    }

    system.hidden = !data.system;
    if (data.system) {
      setText(system.querySelector(".main")!, `CPU ${data.system.cpu}%`);
      setText(system.querySelector(".sub")!, `RAM ${data.system.memory}%`);
      setText(system.querySelector(".label")!, "Abrir o Gerenciador de Tarefas");
    }

    media.hidden = !data.media;
    if (data.media) {
      const { title, artist, playing, thumbnail } = data.media;
      setText(media.querySelector(".main")!, title);
      setText(media.querySelector(".sub")!, artist);
      media.title = artist ? `${title} • ${artist}` : title;
      const cover = media.querySelector<HTMLImageElement>(".cover")!;
      cover.hidden = !thumbnail;
      if (thumbnail && cover.getAttribute("src") !== thumbnail) {
        cover.src = thumbnail;
      }
      setIcon(
        media.querySelector<HTMLElement>('[data-action="toggle"]')!,
        playing ? ICONS.pause : ICONS.play,
      );
    }

    this.syncCapsules();
    this.onVisibilityChange();
  }

  hasVisibleWidgets(): boolean {
    return Array.from(this.element.children).some(
      (group) => !(group as HTMLElement).hidden,
    );
  }
}
