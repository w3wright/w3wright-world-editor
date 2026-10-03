<script setup lang="ts">
/**
 * The shell: the window frame and the one place the open map lives.
 *
 * The frame is drawn by the app rather than by the system — see `TitleBar.vue` for why — so this
 * component owns the layout: a title bar, a menu bar, and the screen below them. The menu is the
 * only navigation, so the screen gets the full width.
 *
 * Nothing here knows about maps beyond "one may be open". Every question of what a map
 * *contains* belongs to a panel and, behind it, to `crates/`; see `docs/03` §1.1.
 */
import type { NavAction } from './nav'
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import MapBrowser from './components/MapBrowser.vue'
import MenuBar from './components/MenuBar.vue'
import TitleBar from './components/TitleBar.vue'
import { closeMap, error, openPath, openPickedMap, summary } from './composables/useOpenMap'
import { clearNotice, ensureStatus, game, launchGame, notice } from './composables/useSettings'

const router = useRouter()

/**
 * Whether the installed-map browser is showing.
 *
 * ⚠️ Deliberately not a route. The browser is a *detour* from whatever screen is behind it — a
 * user opens it from the map overview, changes their mind, and expects to land back on the map
 * overview. A route would make dismissing it a navigation, and the screen underneath would have
 * to be re-entered rather than revealed. It is the one thing here that overlays instead of
 * navigating, and that is what makes it different from `Settings`.
 */
const browsing = ref(false)

// The title bar's launch button and the browser both need to know whether there is a game. Asked
// once at startup rather than on first use, so the button does not appear a beat after the window.
onMounted(() => {
  void ensureStatus()
})

/**
 * File ▸ Open Map…: ask the system, then show the map.
 *
 * Navigating on success keeps the two facts in step: after opening a map, that map's screen is
 * what is showing. Opening from the welcome screen, the terrain screen or anywhere else lands on
 * the overview, so the map that was opened is the one on display.
 */
async function onOpen(): Promise<void> {
  if (await openPickedMap() !== null)
    await router.push('/map')
}

/**
 * A map chosen in the browser: open it, then show it.
 *
 * The navigation happens on success only, so a file the core refuses leaves the user on the
 * screen they were on, with the core's reason in the banner above it — rather than on an empty
 * overview panel that would say "no map is open" about a map that was clicked.
 */
async function onPickInstalled(path: string): Promise<void> {
  browsing.value = false
  if (await openPath(path))
    await router.push('/map')
}

/**
 * File ▸ Close Map: drop the map and its parses, and go back to the way in.
 *
 * The welcome screen rather than the overview: closing a map and being shown a panel that says
 * no map is open is a dead end with an explanation. Nothing was open before the welcome screen
 * and nothing is open after this, so it is the same state and gets the same screen.
 */
async function onClose(): Promise<void> {
  closeMap()
  await router.push('/welcome')
}

/**
 * A menu action: Run ▸ Play in Warcraft III.
 *
 * The path comes from the open map rather than from a field, so there is nothing to get wrong
 * here — the only way this fails is the game itself, and the backend says which way.
 */
async function onLaunch(): Promise<void> {
  if (summary.value)
    await launchGame(summary.value.path)
}

function onAction(id: NavAction['id']): void {
  switch (id) {
    case 'open':
      void onOpen()
      break
    case 'browse':
      // Opened even with no game configured: the browser says what is missing and points at
      // Settings, which is more use than a menu entry that silently does nothing.
      browsing.value = true
      break
    case 'close':
      void onClose()
      break
    case 'play':
      void onLaunch()
      break
  }
}
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden">
    <TitleBar />
    <MenuBar
      :has-map="summary !== null"
      :has-game="game !== null"
      @go="to => router.push(to)"
      @action="onAction"
    />

    <main class="min-h-0 flex flex-1 flex-col overflow-auto">
      <p
        v-if="error"
        class="mx-6 mt-4 max-w-56rem flex flex-none flex-col gap-1 border-l-3 border-red-600 px-4 py-3 text-0.9rem"
      >
        <strong>The core reported a problem.</strong>
        <span class="break-all text-0.85rem font-mono opacity-80">{{ error }}</span>
      </p>

      <!--
        A thing that worked, in one line. ⚠️ It carries a dismiss button even though it expires on
        its own: a message about something that happened in another window (starting the game) is
        one a user may want to get rid of before it times out, and without the button their only
        option is to wait. `role="status"` rather than `alert` — this is not a problem.
      -->
      <p
        v-if="notice"
        role="status"
        class="mx-6 mt-4 max-w-56rem flex flex-none items-center gap-3 border-l-3 border-green-600 px-4 py-2.5 text-0.85rem"
      >
        <span>{{ notice }}</span>
        <button
          type="button"
          class="ml-auto cursor-pointer border-0 bg-transparent text-0.8rem opacity-60"
          aria-label="Dismiss"
          @click="clearNotice"
        >
          ✕
        </button>
      </p>

      <!--
        Every screen handles its own "no map is open" case, because the sentence that belongs
        there is different for each one — "no map to summarise" and "no terrain to show" are
        not the same statement. The shell therefore renders unconditionally and holds no
        opinion about what a screen needs.

        A screen may also ask for the installed-map browser, which is the one thing here that
        overlays rather than navigates — the welcome screen offers it beside "open a map" for the
        same reason the File menu does.
      -->
      <RouterView @open="onOpen" @browse="browsing = true" @close="onClose" />
    </main>

    <MapBrowser
      v-if="browsing"
      :current="summary?.path ?? null"
      @pick="onPickInstalled"
      @close="browsing = false"
    />
  </div>
</template>
