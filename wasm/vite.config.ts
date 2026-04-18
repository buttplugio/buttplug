import { resolve } from 'path';
import { fileURLToPath, URL } from "url";
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'buttplug-wasm',
      fileName: () => 'buttplug-wasm.mjs',
      formats: ['es'],
    },
    outDir: 'dist',
  },
  resolve: {
    alias: {
      "@wasm": fileURLToPath(new URL("../crates/buttplug_wasm/pkg", import.meta.url)),
    },
  },
  plugins: [
    wasm(),
    topLevelAwait(),
    dts({
      exclude: ['tests'],
    }),
  ],
});
