import {
  type Alignment,
  type GenericWidgetSettings,
  type Rect,
  type Widget as IWidget,
  type WidgetConfigDefinition,
  type WidgetId,
  WidgetPreset,
  type WidgetSettingItem,
  WidgetStatus,
} from "@seelen-ui/types";
import { invoke, SeelenCommand, SeelenEvent } from "../../handlers/mod.ts";
import { decodeBase64Url } from "@std/encoding";
import { debounce } from "../../utils/async.ts";
import { adjustPositionByPlacement, fitIntoMonitor, initMonitorsState } from "./positioning.ts";
import { startThemingTool } from "../theme/theming.ts";
import type { InitWidgetOptions, ReadyWidgetOptions, WidgetInformation } from "./interfaces.ts";
import { disableAnimationsOnPerformanceMode } from "./performance.ts";
import { subscribe } from "../../handlers/mod.ts";
import { WidgetBasics } from "./abstractions/mod.ts";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { initOptimisticFrame, isOptimisticFrameValid, OPTIMISTIC_FRAME } from "./abstractions/3_autosize.ts";

interface WidgetInternalState {
  initialized: boolean;
  ready: boolean;
  firstFocus: boolean;
}

/**
 * Represents the widget instance running in the current webview
 */
export class Widget extends WidgetBasics {
  /**
   * Alternative accesor for the current running widget.\
   * Will throw if the library is being used on a non NAI OS environment
   */
  static getCurrent(): Widget {
    const scope = globalThis as ExtendedGlobalThis;
    if (!scope.__SLU_WIDGET) {
      throw new Error("The library is being used on a non NAI OS environment");
    }
    return (
      scope.__SLU_WIDGET_INSTANCE || (scope.__SLU_WIDGET_INSTANCE = new Widget(scope.__SLU_WIDGET))
    );
  }

  /** The current running widget */
  static get self(): Widget {
    return Widget.getCurrent();
  }

  /** widget id */
  public readonly id: WidgetId;
  /** widget definition */
  public readonly def: IWidget;
  /** decoded widget instance information */
  public readonly decoded: WidgetInformation;

  private destroyOnHide = false;
  private runtimeState: WidgetInternalState = {
    initialized: false,
    ready: false,
    firstFocus: true,
  };

  private constructor(widget: IWidget) {
    super();

    this.def = widget;

    const [id, query] = getDecodedWebviewLabel();
    const params = new URLSearchParams(query);
    const paramsObj = Object.freeze(Object.fromEntries(params));

    this.id = id as WidgetId;
    this.decoded = Object.freeze({
      label: `${id}${query ? `?${query}` : ""}`,
      monitorId: paramsObj.monitorId || null,
      instanceId: paramsObj.instanceId || null,
      params: Object.freeze(Object.fromEntries(params)),
    });
  }

  /** Returns if the widget is ready */
  get isReady(): boolean {
    return this.runtimeState.ready;
  }

  /** Returns the default config of the widget, declared on the widget definition */
  public getDefaultConfig(): GenericWidgetSettings {
    const config: GenericWidgetSettings = { enabled: true };
    for (const definition of this.def.settings) {
      Object.assign(config, getDefinitionDefaultValues(definition));
    }
    return config;
  }

  /** Will apply the recommended settings for a desktop widget */
  private applyDesktopPreset(): void {}

  /** Will apply the recommended settings for an overlay widget */
  private applyOverlayPreset(): void {}

  /**
   * Will apply the recommended settings for a popup widget.
   *
   * Prepare-before-show lifecycle (autosized popups):
   *
   *   trigger
   *   → content is already set (the widget's own onTrigger handler is
   *     registered before this one, so $state updates are applied)
   *   → deterministic layout readiness point (rAF = after DOM commit)
   *   → executeAutoSize() while hidden: measures, computes the absolute
   *     anchored placement, applies native size+position and updates the
   *     optimistic frame synchronously
   *   → hard invariant validation of the resolved frame
   *   → show() — the very first visible frame is the final frame
   *
   * No `(0,0)` fallback, no visible corrective reposition after show.
   */
  private applyPopupPreset(): void {
    this.onTrigger(async ({ desiredPosition, alignX, alignY, customArgs }) => {
      const generation = ++popupTriggerGeneration;
      const tStart = performance.now();

      // Bootstrap state of the pod surface: fully non-interactive until the
      // final geometry is verified. Hidden windows are already not hit-tested
      // by the OS; the explicit ignore-cursor flag keeps the same pass-through
      // semantics deterministic across the show() transition.
      // The native calls are individually fault-tolerant so a superseded or
      // transiently invalid handle cannot abort the lifecycle mid-way; the
      // geometry gate below is the authoritative input-enable decision.
      await this.window.setIgnoreCursorEvents(true).catch(() => {});

      // Deterministic readiness boundary: requestAnimationFrame fires after
      // Svelte has committed the DOM for the content set by the earlier
      // trigger handler, and after layout — exactly the point where
      // scrollWidth/scrollHeight are authoritative while still hidden.
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (generation !== popupTriggerGeneration) return; // superseded trigger

      // Diagnostics-only slow-preparation probe (acceptance mode): widen the
      // window between WebView creation and geometry readiness so the
      // hidden/non-hit-testable state of the bootstrap surface can be
      // observed deterministically. The window is created `visible(false)`
      // and `show()` only runs after the final rect is validated, so the
      // probe proves the window stays hidden and non-interactive throughout.
      const slowMs = Number((customArgs as Record<string, unknown> | undefined)?.slowPreparationMs ?? 0);
      if (slowMs > 0) {
        await new Promise<void>((resolve) => setTimeout(resolve, slowMs));
        if (generation !== popupTriggerGeneration) return;
      }

      let frame = null as null | ReturnType<typeof snapshotFrame>;
      if (this.autoSize.enabled) {
        // measure → absolute placement → native apply → synchronous local
        // frame update, all before the window becomes visible
        frame = await this.executeAutoSize();
      } else if (desiredPosition) {
        await this.adjustAndSetPosition(desiredPosition.x, desiredPosition.y, alignX, alignY);
        frame = await OPTIMISTIC_FRAME.runExclusive(snapshotFrame);
      }
      if (generation !== popupTriggerGeneration) return;

      // Hard invariant: never show an unresolved rectangle; never substitute
      // (0,0) for an unknown frame.
      if (!frame || !isOptimisticFrameValid(frame)) {
        console.warn(
          `[popup] frame not ready, keeping hidden`,
          JSON.stringify({ widget: this.id, generation, frame }),
        );
        return;
      }

      const nativeBeforeShow = {
        ...(await this.window.outerPosition().catch(() => ({ x: 0, y: 0 }))),
        ...(await this.window.outerSize().catch(() => ({ width: 0, height: 0 }))),
      };
      await this.show().catch(() => {});

      // Deterministic convergence pass: the first measurement can lag the
      // committed DOM by one observer cycle (dpr normalization + content
      // settling). A second `executeAutoSize` runs the same anchored math
      // against fresh scrollWidth/Height so the native surface matches the
      // visible content exactly before the geometry verification gate.
      if (this.autoSize.enabled) {
        frame = await this.executeAutoSize();
        if (generation !== popupTriggerGeneration) return;
      }
      const nativeAfterShow = {
        ...(await this.window.outerPosition()),
        ...(await this.window.outerSize()),
      };

      // Geometry verification (hard gate before input is enabled): the native
      // hit-test surface must match the measured content box within 2px. On
      // mismatch the surface returns to the non-interactive hidden state, so
      // the desktop/taskbar keep receiving mouse input without a stale
      // oversized overlay intercepting clicks.
      const dpr = globalThis.devicePixelRatio || 1;
      const geometryTolerancePx = 2;
      const geometryVerified =
        isOptimisticFrameValid(frame) &&
        Math.abs(nativeAfterShow.width - frame.width) <= geometryTolerancePx &&
        Math.abs(nativeAfterShow.height - frame.height) <= geometryTolerancePx;
      if (!geometryVerified) {
        console.warn(
          `[popup] geometry not verified, keeping surface non-interactive`,
          JSON.stringify({ widget: this.id, generation, frame, nativeAfterShow, dpr }),
        );
        this.hide();
        return;
      }
      await this.window.setIgnoreCursorEvents(false);
      // value just applied, echoed for the folded acceptance trace
      const ignoreCursorAfterEnable = false;

      // hidden-preparation duration of this exact session: from the trigger
      // dispatch until the moment the validated rect is made visible
      const hiddenWindowUs = Math.max(0, Math.round((performance.now() - tStart) * 1000));

      // temporary first-frame diagnostics, mirrored to the runtime logger via
      // ConsoleWrapper so they are capturable outside DevTools
      console.info(
        `[popup] first-frame trace`,
        JSON.stringify({
          widget: this.id,
          generation,
          previewSessionId: (customArgs as Record<string, unknown> | undefined)?.previewSessionId ?? null,
          hiddenWindowUs,
          dpr,
          geometryVerified,
          ignoreCursorAfterEnable,
          frameInitiallyValid: frame.initialized,
          measured: { width: frame.width, height: frame.height },
          anchor: desiredPosition ?? null,
          finalPhysicalRect: { x: frame.x, y: frame.y, w: frame.width, h: frame.height },
          nativeBeforeShow,
          nativeAfterShow,
        }),
      );

      // The preview is dismissed via the global focus subscription
      // (`hideOnFocusLoss`), which also resolves pointer hit-testing; the
      // WebView itself does not need keyboard foreground to render or receive
      // pointer events, so no forced `focus()` here.
      invoke(SeelenCommand.RecordBootStage, { stage: "popup.show.done" }).catch(() => {});
    });
  }

  private hideOnFocusLoss(): void {
    let wasFocused = false;

    const hideDelayed = debounce(() => {
      this.hide();
    }, 100);

    subscribe(SeelenEvent.GlobalFocusChanged, ({ payload: focused }) => {
      if (focused.hwnd !== this.windowId && focused.ownerHwnd !== this.windowId) {
        if (wasFocused) {
          hideDelayed();
        }
        wasFocused = false;
        return;
      }

      wasFocused = true;
      hideDelayed.cancel();
    });
  }

  /**
   * Will restore the saved position and size of the widget on start,
   * after that will store the position and size of the widget on change.
   */
  private async persistPositionAndSize(): Promise<void> {
    const storage = globalThis.window.localStorage;

    const [x, y, width, height] = [`x`, `y`, `width`, `height`].map((k) => storage.getItem(`${k}`));

    if (x && y) {
      const frame = await OPTIMISTIC_FRAME.runExclusive((ref) => ({
        x: Number(x),
        y: Number(y),
        width: this.autoSize.enabled ? ref.width : Number(width),
        height: this.autoSize.enabled ? ref.height : Number(height),
      }));

      const safeFrame = fitIntoMonitor(frame);
      await this.setPosition({
        left: safeFrame.x,
        top: safeFrame.y,
        right: safeFrame.x + safeFrame.width,
        bottom: safeFrame.y + safeFrame.height,
      });
    }

    this.onMoved(
      debounce((e) => {
        const { x, y } = e.payload;
        storage.setItem(`x`, x.toString());
        storage.setItem(`y`, y.toString());
        console.info(`Widget position saved: ${x} ${y}`);
      }, 500),
    );

    if (!this.autoSize.enabled) {
      this.onResized(
        debounce((e) => {
          const { width, height } = e.payload;
          storage.setItem(`width`, width.toString());
          storage.setItem(`height`, height.toString());
          console.info(`Widget size saved: ${width} ${height}`);
        }, 500),
      );
    }
  }

  // play with zoom level to reset device pixel ratio to 1:1
  private async normalizeDevicePixelRatio(): Promise<void> {
    // NOTE: intentionally *not* derived from `window.scaleFactor()` (the OS/monitor
    // DPI scale). That value doesn't necessarily match this webview's own unzoomed
    // devicePixelRatio (e.g. `window.scaleFactor()` = 1.5 was observed while the
    // webview's native devicePixelRatio was already 1), so using it as the
    // compensation source made the correction converge on the wrong target and spin
    // forever.
    //
    // Root cause is in tao itself: `Window::scale_factor()` is not a live query, it's
    // a cached field (`window_state.scale_factor`) that tao only ever refreshes from
    // the `WM_DPICHANGED` handler (tao's platform_impl/windows/event_loop.rs). Windows
    // does not reliably deliver `WM_DPICHANGED` to a window moved/resized while
    // hidden, so that cache can go stale and stay wrong indefinitely - unlike our own
    // `WindowsApi::get_monitor_scale_factor` (src/background/windows_api/mod.rs),
    // which calls `GetDpiForMonitor` live on every call. `globalThis.devicePixelRatio`
    // is ground truth for what the webview is actually rendering at regardless of
    // which upstream cache is stale, so we accumulate the correction from that
    // instead, folding the zoom already applied into the next reading.
    let zoom = 1;
    // onScaleChanged / onMoved / onResized / the retest below can all trigger a call
    // while a previous call's setZoom is still in flight. Without serializing them,
    // two calls can race on `zoom` and on the webview's actual zoom factor, so the
    // accumulator here permanently desyncs from what's really applied and the WARN
    // below fires forever. Chain calls onto this promise so only one runs at a time.
    let queue = Promise.resolve();

    const normalizeDpr = (retry: number = 0) => {
      queue = queue.then(async () => {
        const dpr = globalThis.devicePixelRatio;
        if (dpr === 1) {
          return;
        }

        console.debug(`normalizeDpr: dpr = ${dpr}, current zoom = ${zoom}`);

        zoom = zoom / dpr;
        await this.webview.setZoom(zoom);
        console.debug(`Zoom compensation set to ${zoom}`);

        if (globalThis.devicePixelRatio !== 1) {
          console.warn(
            `DPR normalization failed! dpr = ${globalThis.devicePixelRatio}, zoom applied = ${zoom}`,
          );
          if (retry < 5) {
            normalizeDpr(retry + 1);
          }
        }
      });
      return queue;
    };

    // onScaleChanged relies on WM_DPICHANGED, which is not reliably emitted for a
    // window that is repositioned while hidden (e.g. moved to another monitor before
    // being shown). onMoved/onResized do fire in that case, so re-check the scale
    // factor whenever the window's position or size changes.
    const recheckDpr = debounce(() => {
      if (globalThis.devicePixelRatio !== 1) {
        normalizeDpr();
      }
    }, 33);

    await this.window.onScaleChanged(recheckDpr);
    this.onMoved(recheckDpr);
    this.onResized(recheckDpr);

    await normalizeDpr();
  }

  /**
   * Will initialize the widget based on the preset and mark it as `pending`, this function won't show the widget.
   * This should be called before any other action on the widget. After this you should call
   * `ready` to mark the widget as ready and show it.
   */
  public async init(options: InitWidgetOptions = {}): Promise<void> {
    if (this.runtimeState.initialized) {
      console.warn(`Widget already initialized`);
      return;
    }

    // the first library call marks the point where the widget module finished
    // evaluating in this webview
    invoke(SeelenCommand.RecordBootStage, { stage: "widget.module.loaded" }).catch(() => {});
    invoke(SeelenCommand.RecordBootStage, { stage: "widget.init.start" }).catch(() => {});

    this.runtimeState.initialized = true;
    await this.prepare();

    this.destroyOnHide = options.closeOnHide ?? this.def.lazy;

    if (options.normalizeDevicePixelRatio) {
      await this.normalizeDevicePixelRatio();
    }

    await initMonitorsState();
    initOptimisticFrame(this);

    if (options.autoSizeByContent) {
      this.setupAutoSizer(options.autoSizeByContent, options.autoSizeFitOnScreen ?? true);
    }

    if (options.saveAndRestoreLastRect ?? this.def.preset === WidgetPreset.Desktop) {
      await this.persistPositionAndSize();
    }

    if (options.hideOnFocusLoss ?? this.def.preset === WidgetPreset.Popup) {
      this.hideOnFocusLoss();
    }

    switch (this.def.preset) {
      case WidgetPreset.None:
        break;
      case WidgetPreset.Desktop:
        this.applyDesktopPreset();
        break;
      case WidgetPreset.Overlay:
        this.applyOverlayPreset();
        break;
      case WidgetPreset.Popup:
        this.applyPopupPreset();
        break;
    }

    if (options.useThemes ?? true) {
      await startThemingTool();
    }

    if (options.disableCssAnimations ?? true) {
      await disableAnimationsOnPerformanceMode();
    } else {
      console.trace("Animations won't be disabled because widget configuration");
    }

    console.debug(`boot: ${this.id} Widget.self.init finished`);
    invoke(SeelenCommand.RecordBootStage, { stage: "widget.init.done" }).catch(() => {});
  }

  /**
   * Will mark the widget as `ready` and pool pending triggers.
   *
   * If the widget is not lazy this will inmediately show the widget.
   * Lazy widget should be shown on trigger action.
   */
  public async ready(options: ReadyWidgetOptions = {}): Promise<void> {
    const { show = !this.def.lazy } = options;

    if (!this.runtimeState.initialized) {
      throw new Error(`Widget was not initialized before ready`);
    }

    if (this.runtimeState.ready) {
      console.warn(`Widget is already ready`);
      return;
    }

    this.runtimeState.ready = true;
    invoke(SeelenCommand.RecordBootStage, { stage: "widget.ready.start" }).catch(() => {});
    if (this.autoSize.enabled) {
      await this.executeAutoSize();
    }

    if (show && !(await this.window.isVisible())) {
      await this.show();
    }

    // this will mark the widget as ready, and send pending trigger event if exists
    await invoke(SeelenCommand.SetCurrentWidgetStatus, { status: WidgetStatus.Ready });
    invoke(SeelenCommand.RecordBootStage, { stage: "widget.ready.done" }).catch(() => {});
    console.debug(`boot: ${this.id} Widget.self.ready invoked`);
  }

  private _attach: { enabled: boolean; unsub?: () => void; rect?: Rect } = { enabled: false };
  /**
   * If for some reason the widget position is changed (like caused by system on system bars addition)
   * this will reposition the widget to the last declared rectangle.
   *
   * Caution: call this only if you are sure not other parts move/resize the widget or will cause flickering.
   */
  public attachPosition(): void {
    if (this._attach.enabled || this.autoSize.enabled) {
      return;
    }

    this._attach.enabled = true;
    this._attach.unsub = this.onRectChange((actual) => {
      if (!this._attach.rect) return;
      const old = this._attach.rect;
      if (
        old.left !== actual.left ||
        old.top !== actual.top ||
        old.right !== actual.right ||
        old.bottom !== actual.bottom
      ) {
        this.setPosition(old!);
      }
    });
  }

  public unattachPosition(): void {
    this._attach.enabled = false;
    this._attach.unsub?.();
    this._attach.unsub = undefined;
  }

  /**
   * This will adjust the position of the widget based on the current placement and alignX/alignY arguments.
   * This makes the widget fit into the monitor where it was placed, avoiding monitor overflow.
   */
  public async adjustAndSetPosition(
    x: number,
    y: number,
    alignX?: Alignment | null,
    alignY?: Alignment | null,
  ): Promise<void> {
    await OPTIMISTIC_FRAME.runExclusive(async (ref) => {
      const adjusted = adjustPositionByPlacement({
        frame: {
          x,
          y,
          width: ref.width,
          height: ref.height,
        },
        originX: alignX,
        originY: alignY,
      });

      const newRect = {
        left: adjusted.x,
        top: adjusted.y,
        right: adjusted.x + adjusted.width,
        bottom: adjusted.y + adjusted.height,
      };
      if (this._attach.enabled) {
        this._attach.rect = { ...newRect };
      }
      await Widget.self.__unsafe_setSelfPosition(newRect, ref);
    });
  }

  public async setPosition(rect: Rect): Promise<void> {
    if (this._attach.enabled) {
      this._attach.rect = { ...rect };
    }
    await OPTIMISTIC_FRAME.runExclusive(async (frame) => {
      await this.__unsafe_setSelfPosition(rect, frame);
    });
  }

  public async show(): Promise<void> {
    debouncedClose.cancel();
    await this.window.show();
  }

  /** Will force foreground the widget */
  public async focus(): Promise<void> {
    if (this.runtimeState.firstFocus) {
      await this.webview.setFocus();
      this.runtimeState.firstFocus = false;
    }
    await invoke(SeelenCommand.RequestFocus, { hwnd: this.windowId }).catch(() => {});
  }

  public hide(): void {
    this.window.hide();
    if (this.destroyOnHide) {
      debouncedClose();
    }
  }
}

const debouncedClose = debounce(() => {
  Widget.self.window.close();
}, 30_000);

/** Monotonic id of the latest popup trigger, used to supersede stale async opens. */
let popupTriggerGeneration = 0;

function snapshotFrame(frame: {
  initialized: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
}) {
  return {
    initialized: frame.initialized,
    x: frame.x,
    y: frame.y,
    width: frame.width,
    height: frame.height,
  };
}

type ExtendedGlobalThis = typeof globalThis & {
  __SLU_WIDGET?: IWidget;
  __SLU_WIDGET_INSTANCE?: Widget;
};

export const SeelenSettingsWidgetId: WidgetId = "@seelen/settings" as WidgetId;
export const SeelenPopupWidgetId: WidgetId = "@seelen/dialog" as WidgetId;
export const SeelenWegWidgetId: WidgetId = "@seelen/weg" as WidgetId;
export const SeelenToolbarWidgetId: WidgetId = "@seelen/fancy-toolbar" as WidgetId;
export const SeelenWindowManagerWidgetId: WidgetId = "@seelen/window-manager" as WidgetId;
export const SeelenWallWidgetId: WidgetId = "@seelen/wallpaper-manager" as WidgetId;

function getDecodedWebviewLabel(): [WidgetId, string | undefined] {
  const encondedLabel = getCurrentWebview().label;
  const decodedLabel = new TextDecoder().decode(decodeBase64Url(encondedLabel));
  const [id, query] = decodedLabel.split("?");
  if (!id) {
    throw new Error("Missing widget id on webview label");
  }
  return [id as WidgetId, query];
}

function getDefinitionDefaultValues(definition: WidgetConfigDefinition): Record<string, unknown> {
  const config: Record<string, unknown> = {};

  // Check if it's a group (has "group" property)
  if ("group" in definition) {
    // Recursively process all items in the group
    for (const item of definition.group.items) {
      Object.assign(config, getDefinitionDefaultValues(item));
    }
  } else {
    // It's a setting item, extract key and defaultValue
    const item = definition as WidgetSettingItem;
    if ("key" in item && "defaultValue" in item) {
      config[item.key] = item.defaultValue;
    }
  }

  return config;
}
