/**
 * Pure-browser stub bridge (`--surface web`).
 * Plan contract: state.* uses `path` + revision; unsupported → `{ code, capability, platform }`.
 */

const memory = new Map();
let rev = 0;
const listeners = new Map();

function unsupported(capability) {
  return {
    ok: false,
    code: "unsupported",
    capability,
    platform: "web",
    message: `${capability} is not supported on web`,
  };
}

function emitLocal(eventName, payload) {
  const set = listeners.get(eventName);
  if (set) {
    for (const h of set) {
      try {
        h(payload);
      } catch (e) {
        console.error("[tish-app web-bridge]", e);
      }
    }
  }
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(`tish:${eventName}`, { detail: payload }));
  }
}

function pathOf(args) {
  return args.path ?? args.key;
}

async function invokeState(cmd, args = {}) {
  const path = pathOf(args);
  if (cmd === "state.get") {
    return {
      ok: true,
      path,
      value: memory.has(path) ? memory.get(path) : null,
      revision: rev,
    };
  }
  if (cmd === "state.set") {
    memory.set(path, args.value);
    rev += 1;
    const payload = {
      path,
      value: args.value,
      revision: rev,
      source: "web",
    };
    emitLocal("state:changed", payload);
    return { ok: true, path, value: args.value, revision: rev };
  }
  if (cmd === "state.patch") {
    const prev = memory.get(path) ?? {};
    const patch = args.value ?? args.patch ?? {};
    const next =
      typeof prev === "object" && prev && typeof patch === "object" && patch
        ? { ...prev, ...patch }
        : patch;
    memory.set(path, next);
    rev += 1;
    emitLocal("state:changed", { path, value: next, revision: rev, source: "web" });
    return { ok: true, path, value: next, revision: rev };
  }
  if (cmd === "state.keys") {
    return { ok: true, keys: [...memory.keys()] };
  }
  if (cmd === "state.delete") {
    const deleted = memory.delete(path);
    if (deleted) {
      rev += 1;
      emitLocal("state:changed", { path, value: null, revision: rev, source: "web" });
    }
    return { ok: true, deleted, revision: rev };
  }
  return null;
}

async function invokeNotification(cmd, args = {}) {
  if (cmd === "notification.permissionState") {
    const perm =
      typeof Notification !== "undefined" ? Notification.permission : "denied";
    return { ok: true, state: perm };
  }
  if (cmd === "notification.requestPermission") {
    if (typeof Notification === "undefined") return unsupported("notification");
    const state = await Notification.requestPermission();
    return { ok: true, state };
  }
  if (cmd === "notification.show") {
    if (typeof Notification === "undefined") return unsupported("notification");
    if (Notification.permission !== "granted") {
      await Notification.requestPermission();
    }
    if (Notification.permission !== "granted") {
      return { ok: false, code: "denied", capability: "notification", message: "permission denied" };
    }
    new Notification(args.title || "", { body: args.body || "" });
    return { ok: true };
  }
  return null;
}

export function installWebBridge() {
  const g = typeof window !== "undefined" ? window : globalThis;

  const api = {
    protocol: "desktop/v1",
    surface: "web",
    getCurrentWindowLabel() {
      return "web";
    },
    async invoke(cmd, args = {}) {
      const stateRes = await invokeState(cmd, args);
      if (stateRes) return stateRes;
      const noteRes = await invokeNotification(cmd, args);
      if (noteRes) return noteRes;
      if (cmd === "ping") {
        return { ok: true, protocol: "desktop/v1", surface: "web", ts: Date.now() };
      }
      return unsupported(cmd.split(".")[0] || cmd);
    },
    async listen(eventName, handler) {
      if (!listeners.has(eventName)) listeners.set(eventName, new Set());
      listeners.get(eventName).add(handler);
      return () => listeners.get(eventName)?.delete(handler);
    },
    async emit(eventName, payload) {
      emitLocal(eventName, payload);
    },
  };

  g.__TISH_APP__ = api;
  g.__TISH_DESKTOP__ = api;
  return api;
}

export function getBridge() {
  const g = typeof window !== "undefined" ? window : globalThis;
  return g.__TISH_APP__ || g.__TISH_DESKTOP__ || installWebBridge();
}
