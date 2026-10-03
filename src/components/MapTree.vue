<script lang="ts">
// Self-reference so a directory can nest its own kind. Vue infers the name from the file, so
// this is not a registration — it is what makes `<MapTree>` resolve inside this file's own
// template, which a component cannot otherwise do without importing itself.
</script>

<script setup lang="ts">
/**
 * One directory in the map tree, with its children.
 *
 * # Why this is recursive rather than flattened by the backend
 *
 * The backend sends the whole tree — it is 9 directories and 192 files on the machine this was
 * built against, and walking it lazily would mean a round trip per chevron. What is left for the
 * interface is disclosure state, which belongs here: whether a folder is open is a property of
 * this window, not of the installation, and two windows could reasonably disagree about it.
 *
 * # Why the open folders are held as a set of paths
 *
 * A path is stable across a re-read of the tree; an index is not, and the tree is re-read every
 * time the browser is opened. Keeping indices would make reopening the browser after a map was
 * added elsewhere open the wrong folders.
 */
import type { MapDir } from '../types'
import { ref } from 'vue'

const props = withDefaults(defineProps<{
  /** The directory to draw. */
  dir: MapDir
  /** How deep, for the indent. */
  depth?: number
  /** Which folders are open, shared by the whole tree. */
  open: Set<string>
  /** The file the user has selected, by path. */
  selected?: string | null
}>(), {
  depth: 0,
  selected: null
})

const emit = defineEmits<{
  /** A map file was chosen. */
  choose: [entry: { name: string, path: string }]
}>()

/**
 * Whether this folder is open.
 *
 * A local ref seeded from the shared set rather than reading the set directly: a `Set` is not
 * reactive in Vue, so a binding that only reads it would not re-render when another folder is
 * opened. The set is still what carries the state across a re-read, which is why both exist.
 *
 * The top level starts open, so a freshly opened browser already shows the first rank of folders
 * rather than one collapsed row.
 */
const expanded = ref(props.open.has(props.dir.path) || props.depth === 0)

/** Toggles this folder and records it in the shared set. */
function toggle(): void {
  expanded.value = !expanded.value
  if (expanded.value)
    props.open.add(props.dir.path)
  else
    props.open.delete(props.dir.path)
}

/** How many maps are under this folder, at any depth. */
function countMaps(dir: MapDir): number {
  return dir.files.length + dir.dirs.reduce((total, child) => total + countMaps(child), 0)
}

/** The indentation for a row at this depth. */
function indent(extra = 0): Record<string, string> {
  return { paddingLeft: `${0.25 + (props.depth + extra) * 0.85}rem` }
}
</script>

<template>
  <li class="tree-node">
    <button
      v-if="dir.name"
      type="button"
      class="tree-row"
      :style="indent()"
      :aria-expanded="expanded"
      @click="toggle"
    >
      <span class="tree-chevron">{{ expanded ? "▾" : "▸" }}</span>
      <span class="tree-name">{{ dir.name }}</span>
      <span v-if="!expanded && countMaps(dir)" class="tree-count">{{ countMaps(dir) }}</span>
    </button>

    <ul v-show="expanded || !dir.name" class="m-0 list-none p-0">
      <MapTree
        v-for="child in dir.dirs"
        :key="child.path"
        :dir="child"
        :depth="dir.name ? depth + 1 : depth"
        :open="open"
        :selected="selected"
        @choose="emit('choose', $event)"
      />
      <li v-for="file in dir.files" :key="file.path" class="m-0 list-none p-0">
        <button
          type="button"
          class="tree-row tree-file"
          :class="{ 'tree-current': selected === file.path }"
          :style="indent(dir.name ? 1 : 0)"
          :aria-current="selected === file.path ? 'true' : undefined"
          @click="emit('choose', file)"
        >
          <span class="tree-chevron" />
          <span class="tree-name">{{ file.name }}</span>
          <span v-if="file.size !== null" class="tree-size">{{ Math.round(file.size / 1024) }} KiB</span>
        </button>
      </li>
    </ul>
  </li>
</template>

<style scoped>
/*
 * Scoped rules rather than utility classes for the three things utilities cannot express here:
 * the indent is data-driven (one level per nesting depth), the chevron needs a fixed width so
 * names line up whether or not a row has one, and `:hover`/`[aria-current]` are state-dependent.
 */
.tree-node {
  list-style: none;
}

.tree-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  width: 100%;
  padding-top: 0.15rem;
  padding-right: 0.5rem;
  padding-bottom: 0.15rem;
  font-size: 0.85rem;
  color: inherit;
  text-align: left;
  cursor: pointer;
  background: none;
  border: 0;
  border-radius: 3px;
}

.tree-row:hover {
  background: rgb(0 0 0 / 5%);
}

.tree-chevron {
  flex: none;
  width: 0.8rem;
  font-size: 0.7rem;
  opacity: 0.65;
}

.tree-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-count,
.tree-size {
  flex: none;
  margin-left: auto;
  font-size: 0.75rem;
  opacity: 0.55;
}

.tree-file .tree-name {
  /* The extension is what the game uses to decide what it can load, so it is worth being able
   * to scan for it. */
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
}

.tree-current {
  background: rgb(47 111 235 / 14%);
}

.tree-current .tree-name {
  font-weight: 600;
}

@media (prefers-color-scheme: dark) {
  .tree-row:hover {
    background: rgb(255 255 255 / 8%);
  }

  .tree-current {
    background: rgb(47 111 235 / 28%);
  }
}
</style>
