import { defineConfig } from "vite";

// Tauri 2 前端配置：
// - 端口固定 5173，与 tauri.conf.json 的 devUrl 一致；strictPort 防止端口漂移
// - build.outDir 固定为 dist，与 tauri.conf.json 的 frontendDist 一致
// - clearScreen 关闭，避免清掉 cargo 的编译输出
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
