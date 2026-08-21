import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// Three surfaces, three entry points. Tauri window URLs point at the built HTML
// files. Inputs are relative to the project root.
//
// Two frameworks on purpose, for the length of the shadcn/ui migration only:
// Settings is React (shadcn/ui is React-only), Launcher and Popover are still
// Svelte. Each plugin claims its own extensions, so they do not collide — and
// the three surfaces never share a component, only `src/lib/*.ts`, which is
// framework-agnostic.
export default defineConfig({
  plugins: [tailwindcss(), svelte(), react()],
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
