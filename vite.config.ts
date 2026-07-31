import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Three surfaces, three entry points. Tauri window URLs point at the built HTML
// files. Inputs are relative to the project root, so no node builtins here.
export default defineConfig({
  plugins: [svelte()],
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
