import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// Three surfaces, three entry points. Tauri window URLs point at the built HTML
// files. Inputs are relative to the project root.
export default defineConfig({
  plugins: [tailwindcss(), react()],
  resolve: {
    // shadcn/ui's generated components import from `@/…`; components.json
    // points at the same root.
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  build: {
    target: "esnext",
    rollupOptions: {
      input: {
        launcher: "launcher.html",
        popover: "popover.html",
        settings: "settings.html",
      },
    },
  },
});
