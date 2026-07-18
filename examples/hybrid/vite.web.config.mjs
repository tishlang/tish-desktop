import { defineConfig } from "vite"
import path from "node:path"
import { fileURLToPath } from "node:url"
import tish from "@tishlang/vite-plugin-tish"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "../..")

export default defineConfig({
  plugins: [
    tish({
      jsxImportSource: "lattish",
      platform: "web",
      surface: "web",
    }),
    {
      name: "hybrid-web-index",
      configureServer(server) {
        server.middlewares.use((req, _res, next) => {
          if (req.url === "/" || req.url === "/index.html") {
            req.url = "/index.web.html"
          }
          next()
        })
      },
    },
  ],
  resolve: {
    alias: {
      "@tish-desktop/desktop-api/bridge": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/desktop-api/web": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/desktop-api/web-bridge": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/app-api/web": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
      ),
      "@tish-desktop/app-api/web-bridge": path.resolve(
        root,
        "packages/desktop-api/src/web-bridge.js"
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
    port: 5180,
    strictPort: true,
    fs: { allow: [root, path.resolve(root, "..")] },
  },
  build: {
    outDir: "dist/web",
    emptyOutDir: true,
    rollupOptions: {
      input: path.resolve(__dirname, "index.web.html"),
    },
  },
})
