import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  root: "ui",
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  // WebView2 is Chromium, kept current by Windows.
  build: { outDir: "dist", emptyOutDir: true, target: "chrome120" },
  css: { modules: { localsConvention: "camelCaseOnly" } },
});
