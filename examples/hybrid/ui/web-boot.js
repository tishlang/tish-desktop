import { installWebBridge } from "@tish-desktop/app-api/web"

installWebBridge()
if (import.meta.hot) {
  import.meta.hot.accept(() => installWebBridge())
}
