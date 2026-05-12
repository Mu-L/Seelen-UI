import { invoke, RuntimeStyleSheet, SeelenCommand, SeelenEvent, Settings, subscribe } from "@seelen-ui/lib";
import type { FocusedApp, TwmReservation, TwmRuntimeTree, WindowManagerSettings } from "@seelen-ui/lib/types";

import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";

let layouts = lazyRune(() => invoke(SeelenCommand.WmGetRenderTree));
subscribe(SeelenEvent.WMTreeChanged, layouts.setByPayload);

let workspaces = lazyRune(() => invoke(SeelenCommand.StateGetVirtualDesktops));
subscribe(SeelenEvent.VirtualDesktopsChanged, workspaces.setByPayload);

let interactables = lazyRune(() => invoke(SeelenCommand.GetUserAppWindows));
subscribe(SeelenEvent.UserAppWindowsChanged, interactables.setByPayload);

let reservation = $state<TwmReservation | null>(null);
subscribe(SeelenEvent.WMSetReservation, (e) => {
  reservation = e.payload;
});

let forceRepositioning = $state(0);
subscribe(SeelenEvent.WMForceRetiling, () => {
  forceRepositioning++;
});

const [focusedAppInit, settingsInit] = await Promise.all([
  invoke(SeelenCommand.GetFocusedApp),
  Settings.getAsync(),
  layouts.init(),
  workspaces.init(),
  interactables.init(),
]);

let focusedApp = $state<FocusedApp>(focusedAppInit);
subscribe(SeelenEvent.GlobalFocusChanged, (e) => {
  focusedApp = e.payload;
});

let settings = $state<WindowManagerSettings>(settingsInit.byWidget["@seelen/window-manager"]);
Settings.onChange((s) => (settings = s.byWidget["@seelen/window-manager"]));

// =================================================
//                  CSS variables
// =================================================

$effect.root(() => {
  $effect(() => {
    const sheet = new RuntimeStyleSheet("@config/window-manager");
    sheet.addVariable("--config-padding", `${settings.workspacePadding}px`);
    sheet.addVariable("--config-containers-gap", `${settings.workspaceGap}px`);
    sheet.addVariable("--config-margin-top", `${settings.workspaceMargin.top}px`);
    sheet.addVariable("--config-margin-left", `${settings.workspaceMargin.left}px`);
    sheet.addVariable("--config-margin-right", `${settings.workspaceMargin.right}px`);
    sheet.addVariable("--config-margin-bottom", `${settings.workspaceMargin.bottom}px`);
    sheet.addVariable("--config-border-offset", `${settings.border.offset}px`);
    sheet.addVariable("--config-border-width", `${settings.border.width}px`);
    sheet.applyToDocument();
  });
});

// =================================================
//               Exported State Getters
// =================================================

export type State = _State;
class _State {
  getLayout(monitorId: string): TwmRuntimeTree | null {
    const activeWsId = workspaces.value?.monitors?.[monitorId]?.active_workspace;
    if (!activeWsId) return null;
    return layouts.value?.workspaces?.[activeWsId] ?? null;
  }
  get forceRepositioning() {
    return forceRepositioning;
  }
  get interactables() {
    return interactables.value;
  }
  get focusedApp() {
    return focusedApp;
  }
  get settings() {
    return settings;
  }
  get reservation() {
    return reservation;
  }
}

export const state = new _State();
