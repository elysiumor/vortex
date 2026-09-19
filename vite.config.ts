import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  clearScreen: false,
  server: {
    port: 3420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 3421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
