<script setup lang="ts">
/**
 * The maps already installed, as a tree, so a map can be picked without a file dialog.
 *
 * # Why this is worth having when a file dialog exists
 *
 * The game's own map folders are where most maps are, and they are the one place the system
 * dialog is worst at: it opens wherever it was last used, and the user has to remember which of
 * `Maps`, `Maps\Download`, `Maps\FrozenThrone\Scenario` and the rest holds the map they mean.
 * This tree is that whole structure at once, and every folder starts collapsed except the top.
 *
 * # What is decided where
 *
 * The tree comes from the backend, one round trip, sorted there so the same installation always
 * lists the same way. ⚠️ It is a **directory listing**, not a claim about what is a map: the
 * extension filter is the game's, and the core decides when one is actually opened. The panel
 * therefore does not validate the choice — it opens what was clicked and shows whatever the core
 * says about it.
 */
import type { MapBrowser, MapDir } from '../types'
import { computed, onMounted, ref } from 'vue'
import { busy, error, listMaps } from '../composables/useSettings'
import MapTree from './MapTree.vue'

const props = defineProps<{
  /** The currently open map, so the tree can mark it. */
  current?: string | null
}>()

const emit = defineEmits<{
  /** The user picked a map to open. */
  pick: [path: string]
  /** The user dismissed the browser without picking anything. */
  close: []
}>()

const browser = ref<MapBrowser | null>(null)
const selected = ref<string | null>(props.current ?? null)

/** Which folders are open. Paths rather than indices: a re-read reorders nothing but may add. */
const open = ref<Set<string>>(new Set())

onMounted(async () => {
  browser.value = await listMaps()
})

/** The root directory, or `null` when the installation has no `Maps`. */
const tree = computed<MapDir | null>(() => browser.value?.tree ?? null)

/** Whether anything at all is worth drawing. */
const empty = computed(() =>
  tree.value === null || (tree.value.dirs.length === 0 && tree.value.files.length === 0)
)

/** Opens the selected map. */
function onOpen(): void {
  if (selected.value !== null)
    emit('pick', selected.value)
}

/** The file name of the selection, for the footer. */
const selectedName = computed(() => {
  const path = selected.value
  if (path === null)
    return null
  return path.split(/[\\/]/).pop() ?? path
})
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-8" @click.self="emit('close')">
    <div class="maps-modal">
      <header class="maps-head">
        <h2 class="m-0 text-0.95rem font-600">
          Open an installed map
        </h2>
        <button
          type="button"
          class="maps-close"
          aria-label="Close"
          @click="emit('close')"
        >
          ✕
        </button>
      </header>

      <p class="maps-where">
        <template v-if="browser?.mapsDir">
          <span class="mono">{{ browser.mapsDir }}</span>
          <span class="opacity-70"> · {{ browser.mapCount }} maps</span>
        </template>
        <template v-else-if="browser">
          the installation at <span class="mono">{{ browser.gameDir }}</span> has no
          <span class="mono">Maps</span> directory
        </template>
        <template v-else>
          looking…
        </template>
      </p>

      <div class="maps-body">
        <p v-if="error" class="m-0 px-3 py-2 text-0.85rem text-red-700">
          {{ error }}
        </p>
        <ul v-else-if="tree && !empty" class="m-0 list-none p-1">
          <MapTree
            :dir="tree"
            :open="open"
            :selected="selected"
            @choose="entry => (selected = entry.path)"
          />
        </ul>
        <p v-else-if="!busy" class="m-0 px-3 py-2 text-0.85rem opacity-70">
          No maps found. Set a game directory in Settings, or open a map from disk.
        </p>
      </div>

      <footer class="maps-foot">
        <span class="maps-picked">
          <template v-if="selectedName">
            <span class="opacity-60">selected </span>{{ selectedName }}
          </template>
        </span>
        <button
          type="button"
          class="maps-btn"
          @click="emit('close')"
        >
          Cancel
        </button>
        <button
          type="button"
          class="maps-btn primary"
          :disabled="selected === null"
          @click="onOpen"
        >
          Open
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/*
 * A dialog and a scroll region, which utilities cannot express: the modal needs a maximum height
 * derived from the viewport so it never grows past the window, and the body needs to be the only
 * scrolling part while the header and footer stay put. The rest of the layout is utilities.
 */
.maps-modal {
  display: flex;
  flex-direction: column;
  width: 36rem;
  max-width: 100%;
  max-height: 100%;
  overflow: hidden;
  background: #fff;
  border-radius: 6px;
  box-shadow: 0 10px 40px rgb(0 0 0 / 32%);
}

.maps-head {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  padding: 0.6rem 0.5rem 0.6rem 0.9rem;
  border-bottom: 1px solid rgb(0 0 0 / 10%);
}

.maps-close {
  padding: 0.2rem 0.45rem;
  font-size: 0.85rem;
  color: inherit;
  cursor: pointer;
  background: none;
  border: 0;
  border-radius: 3px;
}

.maps-close:hover {
  background: rgb(0 0 0 / 6%);
}

.maps-where {
  flex: none;
  margin: 0;
  padding: 0.4rem 0.9rem;
  overflow: hidden;
  font-size: 0.78rem;
  text-overflow: ellipsis;
  white-space: nowrap;
  border-bottom: 1px solid rgb(0 0 0 / 7%);
}

.maps-body {
  flex: 1;
  min-height: 12rem;
  overflow: auto;
}

.maps-foot {
  display: flex;
  flex: none;
  gap: 0.5rem;
  align-items: center;
  padding: 0.6rem 0.9rem;
  border-top: 1px solid rgb(0 0 0 / 10%);
}

.maps-picked {
  flex: 1;
  overflow: hidden;
  font-size: 0.8rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.maps-btn {
  padding: 0.35rem 0.85rem;
  font-size: 0.85rem;
  color: inherit;
  cursor: pointer;
  background: #fff;
  border: 1px solid rgb(0 0 0 / 18%);
  border-radius: 4px;
}

.maps-btn:hover:not(:disabled) {
  background: rgb(0 0 0 / 4%);
}

.maps-btn:disabled {
  cursor: default;
  opacity: 45%;
}

.maps-btn.primary {
  color: var(--on-accent);
  background: var(--accent);
  border-color: var(--accent);
}

.maps-btn.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

@media (prefers-color-scheme: dark) {
  .maps-modal {
    background: #2b2b2b;
  }

  .maps-head,
  .maps-foot {
    border-color: rgb(255 255 255 / 14%);
  }

  .maps-where {
    border-color: rgb(255 255 255 / 10%);
  }

  .maps-close:hover,
  .maps-btn:hover:not(:disabled) {
    background: rgb(255 255 255 / 8%);
  }

  .maps-btn {
    background: #2b2b2b;
    border-color: rgb(255 255 255 / 22%);
  }

  .maps-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
  }
}
</style>
