import { defineConfig } from "vite"
import path from "node:path"
import { existsSync } from "node:fs"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "../..")
const localTish = path.resolve(root, "../tish/target/debug/tish")
const tishPath =
  process.env.TISH_PATH || (existsSync(localTish) ? localTish : "tish")

export default defineConfig({
  plugins: [
    tish({
      tishPath,
      jsxImportSource: "lattish",
      platform: process.env.TISH_PLATFORM || "macos",
      surface: process.env.TISH_SURFACE || "webview",
    }),
  ],
  resolve: {
    alias: {
      "@tish-desktop/desktop-api/bridge": path.resolve(
        root,
        "packages/desktop-api/src/bridge.js"
      ),
      "@tish-desktop/desktop-api": path.resolve(
        root,
        "packages/desktop-api/src/appApi.tish"
      ),
      "@tish-desktop/app-api": path.resolve(root, "packages/app-api/src/index.tish"),
      "@tish-desktop/shared": path.resolve(root, "packages/shared/src/index.tish"),
      "@tish-desktop/ui-theme": path.resolve(
        root,
        "packages/ui-theme/src/index.tish"
      ),
      lattish: path.resolve(root, "../lattish/src/Lattish.tish"),
      "lattish/jsx-runtime": path.resolve(
        root,
        "../lattish/src/jsx-runtime.tish"
      ),
      "tish-tailwind/tw": path.resolve(root, "../tish-tailwind/src/tw.tish"),
      "tish-tailwind": path.resolve(root, "../tish-tailwind/src/tw.tish"),
    },
  },
  server: {
    port: 5177,
    strictPort: true,
    fs: { allow: [root, path.resolve(root, "..")] },
  },
  build: {
    outDir: "dist/ui",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, "index.html"),
        chrome: path.resolve(__dirname, "chrome.html"),
      },
    },
  },
})
