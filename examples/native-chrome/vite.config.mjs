import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [tish({ jsxImportSource: "lattish" })],
  resolve: {
    alias: {
      "@tishlang/tish-desktop-api/bridge": path.resolve(
        __dirname,
        "../../packages/desktop-api/src/bridge.js"
      ),
      "@tishlang/tish-desktop-api": path.resolve(
        __dirname,
        "../../packages/desktop-api/src/desktopHost.tish"
      ),
      "@tishlang/tish-desktop-shared": path.resolve(__dirname, "../../packages/shared/src/index.tish"),
      "@tishlang/tish-desktop-ui-theme": path.resolve(
        __dirname,
        "../../packages/ui-theme/src/index.tish"
      ),
      lattish: path.resolve(__dirname, "node_modules/lattish/src/Lattish.tish"),
      "lattish/jsx-runtime": path.resolve(__dirname, "node_modules/lattish/src/jsx-runtime.tish"),
      "tish-tailwind/tw": path.resolve(__dirname, "node_modules/tish-tailwind/src/tw.tish"),
      "tish-tailwind": path.resolve(__dirname, "node_modules/tish-tailwind/src/tw.tish"),
    },
  },
  server: { port: 5175, strictPort: true },
  build: { outDir: "dist/ui", emptyOutDir: true },
})
