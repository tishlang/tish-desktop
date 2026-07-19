import { defineConfig } from "vite"
import tish from "@tishlang/vite-plugin-tish"

export default defineConfig({
  plugins: [
    tish({
      platform: process.env.TISH_PLATFORM,
      surface: process.env.TISH_SURFACE || "webview",
    }),
  ],
  server: {
    port: __DEV_PORT__,
    strictPort: true,
  },
  build: {
    outDir: "dist/ui",
    emptyOutDir: true,
  },
})
