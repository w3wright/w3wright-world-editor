// @ts-expect-error type error without @types/node package
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import { defineConfig } from 'vite'
import VueRouter from 'vue-router/vite'

const host = process.env.TAURI_DEV_HOST
const r = (p: string) => fileURLToPath(new URL(p, import.meta.url))

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    UnoCSS(),

    // File-based routing from `pages/`, via vue-router 5's own plugin — this is the
    // same thing `unplugin-vue-router` became, now shipped inside vue-router.
    //
    // ⚠️ It is listed **before** the Vue plugin. The router plugin has to see the
    // route files to generate them, and putting it after lets the Vue transform claim
    // them first.
    VueRouter({
      routesFolder: 'pages',
      // The generated route table and `RouteNamedMap`. Kept out of `src/` because it
      // is output, not source: it is gitignored and rewritten on every dev-server
      // start, so treating it as a hand-edited file would be a mistake.
      dts: r('.auto-generate/typed-router.d.ts')
    }),

    vue()
  ],

  resolve: {
    // `~` for `src/`. Pages live at the repository root and the code does not, so every
    // page would otherwise import through `../src/...`.
    alias: [{ find: /^~\//, replacement: `${r('src')}/` }]
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**']
    }
  }
}))
