import { installBridge } from "@tish-desktop/desktop-api/bridge"

installBridge()
if (import.meta.hot) {
  import.meta.hot.accept(() => installBridge())
}
