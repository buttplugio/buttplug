import { fileURLToPath, URL } from "url";
import { defineConfig, Plugin } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

function wasmEnvPlugin(): Plugin {
  const virtualId = "\0virtual:env";
  return {
    name: "wasm-env",
    enforce: "pre",
    resolveId(id) {
      if (id === "env") return virtualId;
    },
    load(id) {
      if (id === virtualId)
        return "export function now() { return Date.now(); }";
    },
  };
}

export default defineConfig({
  resolve: {
    alias: {
      "buttplug-wasm": fileURLToPath(new URL("../../src/index.ts", import.meta.url)),
      "@wasm": fileURLToPath(new URL("../../../crates/buttplug_wasm/pkg", import.meta.url)),
    },
  },
  optimizeDeps: {
    exclude: ["buttplug-wasm"],
  },
  server: {
    fs: {
      allow: [fileURLToPath(new URL("../../..", import.meta.url))],
    },
  },
  plugins: [wasmEnvPlugin(), wasm(), topLevelAwait()],
});
