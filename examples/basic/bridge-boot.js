import { installBridge } from "../../packages/desktop-api/src/bridge.js"

installBridge()
if (import.meta.hot) {
  import.meta.hot.accept(() => installBridge())
}
