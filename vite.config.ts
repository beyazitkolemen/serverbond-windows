import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    strictPort: true,
    watch: { ignored: ["**/target/**", "**/src-tauri/**"] },
  },
  // Keep in sync with bundle.windows.minimumWebview2Version.
  // Evergreen still updates normally; this is a syntax floor, not a pinned runtime.
  build: { target: "chrome109", cssTarget: "chrome109" },
});
