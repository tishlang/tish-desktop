/**
 * Injects window.__TISH_APP__ (and compat __TISH_DESKTOP__).
 * Rebind-safe for Vite HMR — call installBridge() from UI boot and hot.accept.
 *
 * Prefer an existing **host** bridge (tish-macos / tish-ios WK bootstrap with
 * `webkit.messageHandlers.tish` + `__dispatch`) over the Tauri transport. Nested
 * `<webview bridge>` pages load this module after document-start injection; overwriting
 * with Tauri broke `state:changed` sync and `invoke` in hybrid SC4.
 */
export function installBridge() {
  const g = typeof window !== "undefined" ? window : globalThis;

  const existing = g.__TISH_APP__ || g.__TISH_DESKTOP__;
  const hostWk =
    typeof g.webkit !== "undefined" &&
    g.webkit?.messageHandlers?.tish != null;
  // Nested host WK (tish-macos/ios bootstrap): never replace with Tauri.
  // Bootstrap has `__dispatch` for broker → pane events (`state:changed`).
  if (hostWk) {
    if (existing && typeof existing.__dispatch === "function") {
      g.__TISH_APP__ = existing;
      g.__TISH_DESKTOP__ = existing;
      return existing;
    }
    console.warn(
      "[tish-app] WK host bridge present but __dispatch missing — skip Tauri install (reload if sync is broken)"
    );
    if (existing) return existing;
  }

  if (!g.__TAURI__?.core?.invoke) {
    console.warn("[tish-app] Tauri core.invoke not available yet");
  }

  const api = {
    protocol: "desktop/v1",
    surface: "webview",
    getCurrentWindowLabel() {
      try {
        return g.__TAURI__?.webviewWindow?.getCurrentWebviewWindow?.()?.label ?? "main";
      } catch {
        return "main";
      }
    },
    async invoke(cmd, args = {}) {
      const core = g.__TAURI__?.core;
      if (!core?.invoke) {
        throw new Error("Tauri core.invoke unavailable");
      }
      return core.invoke("desktop_invoke", { cmd, args });
    },
    async listen(eventName, handler) {
      const event = g.__TAURI__?.event;
      if (!event?.listen) {
        throw new Error("Tauri event.listen unavailable");
      }
      return event.listen(eventName, (e) => handler(e.payload));
    },
    async emit(eventName, payload) {
      const event = g.__TAURI__?.event;
      if (!event?.emit) return;
      return event.emit(eventName, payload);
    },
  };

  g.__TISH_APP__ = api;
  g.__TISH_DESKTOP__ = api; // compat alias
  return api;
}

export function getBridge() {
  const g = typeof window !== "undefined" ? window : globalThis;
  return g.__TISH_APP__ || g.__TISH_DESKTOP__ || installBridge();
}
