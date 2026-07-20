import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  root: "web",
  publicDir: "../static",
  plugins: [wasm()],
  base: "/",
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "esnext",
    minify: true,
    sourcemap: false
  },
  server: {
    host: "127.0.0.1",
    port: 3000
  },
  optimizeDeps: {
    exclude: ["./src/wasm"]
  }
});
