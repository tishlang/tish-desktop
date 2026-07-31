import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "__ROOT__")

export default defineConfig({
  plugins: [tish({ jsxImportSource: "lattish" })],
  resolve: {
    alias: {
      "@tishlang/tish-desktop-api/bridge": path.resolve(
        root,
        "packages/desktop-api/src/bridge.js"
      ),
      "@tishlang/tish-desktop-api": path.resolve(
        root,
        "packages/desktop-api/src/desktopHost.tish"
      ),
      "@tishlang/tish-desktop-shared": path.resolve(root, "packages/shared/src/index.tish"),
      "@tishlang/tish-desktop-ui-theme": path.resolve(root, "packages/ui-theme/src/index.tish"),
      lattish: path.resolve(root, "../lattish/src/Lattish.tish"),
      "lattish/jsx-runtime": path.resolve(root, "../lattish/src/jsx-runtime.tish"),
      "tish-tailwind/tw": path.resolve(root, "../tish-tailwind/src/tw.tish"),
      "tish-tailwind": path.resolve(root, "../tish-tailwind/src/tw.tish"),
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
