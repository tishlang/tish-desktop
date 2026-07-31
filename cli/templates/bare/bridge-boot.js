import { installBridge } from "@tishlang/tish-desktop-api/bridge"

installBridge()
if (import.meta.hot) {
  import.meta.hot.accept(() => installBridge())
}
