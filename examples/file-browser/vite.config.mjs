import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [tish({ jsxImportSource: "lattish" })],
  resolve: {
    alias: {
      "@tish-desktop/desktop-api": path.resolve(__dirname, "../../packages/desktop-api/src/desktopHost.tish"),
      "@tish-desktop/desktop-api/bridge": path.resolve(__dirname, "../../packages/desktop-api/src/bridge.js"),
      "@tish-desktop/shared": path.resolve(__dirname, "../../packages/shared/src/index.tish"),
      "@tish-desktop/ui-theme": path.resolve(
        __dirname,
        "../../packages/ui-theme/src/index.tish"
      ),
      lattish: path.resolve(__dirname, "../../../lattish/src/Lattish.tish"),
      "lattish/jsx-runtime": path.resolve(__dirname, "../../../lattish/src/jsx-runtime.tish"),
      "tish-tailwind/tw": path.resolve(__dirname, "../../../tish-tailwind/src/tw.tish"),
      "tish-tailwind": path.resolve(__dirname, "../../../tish-tailwind/src/tw.tish"),
    },
  },
  server: { port: 5174, strictPort: true },
  build: { outDir: "dist/ui", emptyOutDir: true },
})
