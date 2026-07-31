import { installWebBridge } from "@tishlang/tish-app-api/web"

installWebBridge()
if (import.meta.hot) {
  import.meta.hot.accept(() => installWebBridge())
}
