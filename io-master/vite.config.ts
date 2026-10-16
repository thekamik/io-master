// Vite builds the page of the desktop interface: the Svelte
// components of ui/src become plain JavaScript and CSS files in
// ui/dist, which Tauri builds into the program (tauri.conf.json,
// "frontendDist"). Nothing is loaded from the network at run time.
import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

/**
 * Makes d3-color (used by Svelte Flow, the drawing of schemes) work
 * with the frozen prototypes of the window (tauri.conf.json,
 * "freezePrototype", a protection against prototype pollution).
 *
 * Its file define.js gives the prototype of a colour the properties
 * "constructor" and "toString" by assignment. Object.prototype is
 * frozen, so an assignment to a property it already has is refused
 * ("Attempted to assign to readonly property"), and the window would
 * not start. The same two functions, defining the properties instead
 * of assigning them, do the same without touching Object.prototype.
 */
function d3ColorWithFrozenPrototypes(): Plugin {
  const replacement = `function own(object, key, value) {
  Object.defineProperty(object, key, { value, writable: true, configurable: true, enumerable: key !== "constructor" });
}

export default function(constructor, factory, prototype) {
  constructor.prototype = factory.prototype = prototype;
  own(prototype, "constructor", constructor);
}

export function extend(parent, definition) {
  var prototype = Object.create(parent.prototype);
  for (var key in definition) own(prototype, key, definition[key]);
  return prototype;
}
`;

  return {
    name: "d3-color-with-frozen-prototypes",
    enforce: "pre",
    transform(code, id) {
      if (!id.replace(/\\/g, "/").endsWith("/node_modules/d3-color/src/define.js")) {
        return null;
      }

      // Only the file as it is known: a changed d3-color must be
      // looked at again, not patched blindly.
      if (!code.includes("prototype.constructor = constructor;")) {
        this.error("d3-color/src/define.js has changed; check the patch in vite.config.ts");
      }

      return { code: replacement, map: null };
    },
  };
}

export default defineConfig({
  root: "ui",
  plugins: [d3ColorWithFrozenPrototypes(), svelte()],
  // Relative paths, so that the page works from the protocol of
  // Tauri as well as from a file.
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // The web view is WebKitGTK 2.54: current JavaScript is fine.
    target: "es2022",
    // The page is read from the program itself, never over a network,
    // so a large bundle (pdf.js, Svelte Flow) costs nothing.
    chunkSizeWarningLimit: 3000,
  },
});
