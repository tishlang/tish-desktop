/**
 * Injects window.__TISH_APP__ (and compat __TISH_DESKTOP__) using Tauri's global API.
 * Rebind-safe for Vite HMR — call installBridge() from UI boot and hot.accept.
 */
export function installBridge() {
  const g = typeof window !== "undefined" ? window : globalThis;
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
