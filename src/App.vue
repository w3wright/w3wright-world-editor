<script setup lang="ts">
import { useRouter } from 'vue-router'
/**
 * The shell: the window chrome, and the one place the open box lives.
 *
 * The box is here rather than in a page because this component outlives the route. An
 * input inside a page is destroyed on navigation, which would clear what the user typed
 * the moment they looked at another screen.
 */
import {
  busy,
  error,
  openPath,
  path,
  summary,
  terrainBusy
} from './composables/useOpenMap'
import { fileName } from './format'

const router = useRouter()

/**
 * Opens the map in the box and returns to the map screen.
 *
 * Navigating on success keeps the two facts in step: after opening a map, that map's
 * screen is what is showing. Opening from the terrain screen must not leave the terrain
 * of the previous map on screen.
 */
async function open(): Promise<void> {
  path.value = path.value.trim()
  if (await openPath())
    await router.push('/')
}
</script>

<template>
  <div class="h-screen flex flex-col">
    <header class="fyc flex-none gap-3 border-b border-b-line-soft px-3 py-2">
      <span class="whitespace-nowrap text-0.9rem font-600">W3wright World Editor</span>

      <form class="min-w-0 flex flex-1 gap-1.5" @submit.prevent="open">
        <input
          v-model="path"
          type="text"
          spellcheck="false"
          placeholder="Path to a .w3x or .w3m map"
          aria-label="Map path"
          class="min-w-0 flex-1 border border-line rounded-md bg-[transparent] px-2.5 py-1.5 text-0.85rem text-[inherit] font-mono outline-none"
        >
        <button type="submit" class="toolbar-btn" :disabled="busy || !path.trim()">
          {{ busy ? "Opening…" : "Open" }}
        </button>
      </form>

      <span
        v-if="summary"
        class="max-w-16rem overflow-hidden text-ellipsis whitespace-nowrap text-0.8rem opacity-60"
        :title="summary.path"
      >
        {{ fileName(summary.path) }}
      </span>

      <nav v-if="summary" class="flex gap-1">
        <RouterLink to="/" class="view-tab">
          Map
        </RouterLink>
        <RouterLink to="/terrain" class="view-tab">
          {{ terrainBusy ? "Terrain…" : "Terrain" }}
        </RouterLink>
      </nav>
    </header>

    <main class="flex-1 overflow-auto">
      <p
        v-if="error"
        class="mx-6 mt-4 max-w-56rem flex flex-col gap-1 border-l-3 border-red-600 px-4 py-3 text-0.9rem"
      >
        <strong>The core reported a problem.</strong>
        <span class="break-all text-0.85rem font-mono opacity-80">{{ error }}</span>
      </p>

      <RouterView />
    </main>
  </div>
</template>

<style scoped>
/*
 * Kept in CSS rather than utilities because it is a state, not a set of declarations:
 * `router-link-active` is applied by the router, and the three rules only make sense
 * together — the inactive tab is dimmed, the active one is not, and the active one also
 * gains weight and a surface. Spelling that out as three utility strings on one element
 * with a variant would be longer and harder to read than the CSS it replaces.
 *
 * The active tab is marked by weight and a border as well as by colour, so it survives a
 * colour-blind reading and a high-contrast theme.
 */
.view-tab {
  padding: 0.35rem 0.8rem;
  font-size: 0.85rem;
  border-radius: 6px;
  border: 1px solid transparent;
  color: inherit;
  text-decoration: none;
  cursor: pointer;
  opacity: 0.6;
}

.view-tab:hover {
  opacity: 1;
}

.view-tab.router-link-active {
  opacity: 1;
  font-weight: 600;
  border-color: rgb(128 128 128 / 0.4);
  background: rgb(128 128 128 / 0.1);
}
</style>
