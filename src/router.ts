import type { RouteRecordRaw } from 'vue-router'
import { createRouter, createWebHashHistory } from 'vue-router'
import { routes } from 'vue-router/auto-routes'

/**
 * The router, with routes generated from `pages/`.
 *
 * # Why the route table is generated
 *
 * A hand-written table is a second place a file's existence is recorded, and the two
 * drift the first time someone moves a file. `vue-router/vite` reads `pages/` and
 * writes the table plus its types at dev-server start, so adding a screen is adding a
 * file.
 *
 * `routes` comes from `vue-router/auto-routes`, the virtual module the plugin emits.
 *
 * # Why hash history
 *
 * The app is served from `tauri://localhost` (and `http://localhost:1420` in dev), not
 * from a web server that rewrites unknown paths to `index.html`. With HTML5 history, a
 * reload on any route but `/` asks the asset layer for a file that does not exist and
 * the window comes up blank. A hash is always part of the document, so it survives.
 *
 * # Why `/` is added rather than being a page
 *
 * The first screen is the welcome screen, and it lives at `/welcome`. `pages/` has no index
 * screen, because a page file at `pages/index.vue` would also be the route `/` — the same
 * screen reachable at two paths, which means two answers to "is anything highlighted in the
 * menu". One redirect is the whole fix.
 *
 * ⚠️ `/welcome` is deliberately **not** in `nav.ts`. It is not a place to navigate to; it is
 * where the window starts and where closing a map returns to. Listing it in the menu would put
 * an entry there that means "leave what you are doing".
 */
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/welcome' },
    ...(routes as RouteRecordRaw[])
  ]
})
