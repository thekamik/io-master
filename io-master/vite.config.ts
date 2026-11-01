// Vite builds the page of the desktop interface: the Svelte
// components of ui/src become plain JavaScript and CSS files in
// ui/dist, which Tauri builds into the program (tauri.conf.json,
// "frontendDist"). Nothing is loaded from the network at run time.
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  root: "ui",
  plugins: [svelte()],
  // Relative paths, so that the page works from the protocol of
  // Tauri as well as from a file.
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // The web view is WebKitGTK 2.54: current JavaScript is fine.
    target: "es2022",
  },
});
