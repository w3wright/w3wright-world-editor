<script setup lang="ts">
/**
 * The shell: the window frame and the one place the open map lives.
 *
 * The frame is drawn by the app rather than by the system — see `TitleBar.vue` for why — so
 * this component owns the layout: a title bar, a menu bar, and the screen below them. The
 * menu is the only navigation, so the screen gets the full width.
 *
 * Nothing here knows about maps beyond "one may be open". Every question of what a map
 * *contains* belongs to a panel and, behind it, to `crates/`; see `docs/03` §1.1.
 */
import { useRouter } from 'vue-router'
import MenuBar from './components/MenuBar.vue'
import TitleBar from './components/TitleBar.vue'
import { closeMap, error, openPickedMap, summary } from './composables/useOpenMap'

const router = useRouter()

/**
 * File ▸ Open Map…: ask the system, then show the map.
 *
 * Navigating on success keeps the two facts in step: after opening a map, that map's screen is
 * what is showing. Opening from the welcome screen, the terrain screen or anywhere else lands
 * on the overview, so the map that was opened is the one on display.
 */
async function onOpen(): Promise<void> {
  if (await openPickedMap() !== null)
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

function onAction(id: 'open' | 'close'): void {
  void (id === 'open' ? onOpen() : onClose())
}
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden">
    <TitleBar />
    <MenuBar
      :has-map="summary !== null"
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
        Every screen handles its own "no map is open" case, because the sentence that belongs
        there is different for each one — "no map to summarise" and "no terrain to show" are
        not the same statement. The shell therefore renders unconditionally and holds no
        opinion about what a screen needs.
      -->
      <RouterView @open="onOpen" @close="onClose" />
    </main>
  </div>
</template>
