import { resolve } from 'path';
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';

export default defineConfig({
  build: {
    target: 'esnext',
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'buttplug-wasm',
      fileName: () => 'buttplug-wasm.mjs',
      formats: ['es'],
    },
    outDir: 'dist',
    rollupOptions: {
      external: ['buttplug-wasm-blob', 'buttplug', 'eventemitter3'],
    },
  },
  plugins: [
    dts({
      exclude: ['tests'],
    }),
  ],
});
