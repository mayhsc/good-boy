import { defineConfig } from "vite"

export default defineConfig({
  server: {
    // wasm-pack writes to ../pkg, outside the web root.
    fs: { allow: [".."] },
  },
  build: {
    // index.ts uses top-level await.
    target: "es2022",
  },
})
