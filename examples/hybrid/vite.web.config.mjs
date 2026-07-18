import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "../..")

export default defineConfig({
  plugins: [
    tish({
      platform: "web",
      surface: "web",
    }),
  ],
  resolve: {
    alias: {
      "@tish-desktop/desktop-api/bridge": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/desktop-api/web-bridge": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/desktop-api": path.resolve(
        root,
        "packages/desktop-api/src/appApi.tish"
      ),
      "@tish-desktop/app-api": path.resolve(root, "packages/app-api/src/index.tish"),
      "@tish-desktop/shared": path.resolve(root, "packages/shared/src/index.tish"),
    },
  },
  server: {
    port: 5180,
    strictPort: true,
    fs: { allow: [root, path.resolve(root, "..")] },
  },
  build: {
    outDir: "dist/web",
    emptyOutDir: true,
  },
})
