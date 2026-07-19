import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "__ROOT__")

export default defineConfig({
  plugins: [
    tish({
      platform: process.env.TISH_PLATFORM,
      surface: process.env.TISH_SURFACE || "webview",
    }),
  ],
  resolve: {
    alias: {
      "@tishlang/tish-desktop-api/bridge": path.resolve(
        root,
        "packages/desktop-api/src/bridge.js"
      ),
      "@tishlang/tish-desktop-api": path.resolve(
        root,
        "packages/desktop-api/src/appApi.tish"
      ),
      "@tishlang/tish-app-api": path.resolve(root, "packages/app-api/src/index.tish"),
      "@tishlang/tish-desktop-shared": path.resolve(root, "packages/shared/src/index.tish"),
    },
  },
  server: {
    port: __DEV_PORT__,
    strictPort: true,
    fs: { allow: [root, path.resolve(root, "..")] },
  },
  build: {
    outDir: "dist/ui",
    emptyOutDir: true,
  },
})
