import { defineConfig } from "vite"
import tish from "@tishlang/vite-plugin-tish"

export default defineConfig({
  plugins: [tish({ jsxImportSource: "lattish" })],
  server: {
    port: __DEV_PORT__,
    strictPort: true,
  },
  build: {
    outDir: "dist/ui",
    emptyOutDir: true,
  },
})
