import type { MapSummary, SavePreview, TerrainView } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

/**
 * The open map, shared across routes.
 *
 * # Why module-level state rather than a store
 *
 * There is exactly one open map, it belongs to the window rather than to any screen,
 * and every consumer is a route. A store would add a dependency and a provider to
 * manage a single value; module state is that value. If a second thing ever needs
 * sharing, that is the point to reconsider — not before.
 *
 * # Why the backend stays stateless
 *
 * Nothing here caches a parse. `summary` and `terrain` are the last answers the core
 * gave, and every action re-asks it. That is what keeps a save from leaving the
 * screen showing data from before the save: there is no cache to invalidate.
 */

/** The path in the box, which is not necessarily the open map. */
export const path = ref('')

/** The open map, or `null` when nothing has been opened. */
export const summary = ref<MapSummary | null>(null)

/** A message for the user, from the core, shown verbatim. */
export const error = ref<string | null>(null)

/** A command is in flight. */
export const busy = ref(false)

/** The save preview, once asked for. */
export const preview = ref<SavePreview | null>(null)

/**
 * The save preview could not be produced because the operation does not apply.
 *
 * Kept apart from `error` because the two need different words: a map the core cannot
 * rebuild is not a map that failed to open.
 */
export const previewNotApplicable = ref<string | null>(null)

/** The terrain of the open map, or `null` when it has not been asked for. */
export const terrain = ref<TerrainView | null>(null)

/** The terrain command is in flight. */
export const terrainBusy = ref(false)

/**
 * Marker the backend puts in front of a message that means "does not apply here".
 *
 * Kept in step with `NOT_APPLICABLE` in `commands.rs`. Both sides name it, because the
 * interface has to tell a limitation from a fault and the backend is the only thing
 * that knows which it is.
 */
export const NOT_APPLICABLE = 'not-applicable: '

/**
 * Opens whatever path is in the box.
 *
 * The backend decides whether it is a map and says why not when it is not, so there is
 * no validation here beyond "the box is not empty". A second opinion in the interface
 * would be a second set of rules to keep in step.
 *
 * @returns whether the map opened, so a caller can navigate on success only.
 */
export async function openPath(): Promise<boolean> {
  const target = path.value.trim()
  if (!target || busy.value)
    return false
  busy.value = true
  error.value = null
  preview.value = null
  previewNotApplicable.value = null
  // Opening a different map invalidates the previous map's terrain. Leaving it would
  // show one map's terrain under another map's name.
  terrain.value = null
  try {
    summary.value = await invoke<MapSummary>('open_map', { path: target })
    return true
  }
  catch (e) {
    // The core's message is the one a user can act on, shown verbatim rather than
    // replaced with something vaguer.
    summary.value = null
    error.value = String(e)
    return false
  }
  finally {
    busy.value = false
  }
}

/**
 * Loads the terrain on first use.
 *
 * Terrain is 25,921 tile points of detail for a 161×161 map and most screens never look
 * at it, so it is fetched when a route that shows it is entered rather than on open.
 * Already-loaded terrain is kept: re-entering the route must not re-parse.
 */
export async function loadTerrain(): Promise<void> {
  if (!summary.value || terrain.value || terrainBusy.value)
    return
  terrainBusy.value = true
  try {
    terrain.value = await invoke<TerrainView>('read_terrain', {
      path: summary.value.path
    })
  }
  catch (e) {
    terrain.value = null
    error.value = String(e)
  }
  finally {
    terrainBusy.value = false
  }
}

/**
 * Asks the backend what saving this map would change.
 *
 * Writes nothing. The answer matters because a rebuild is not a minimal change: it
 * preserves member content but rewrites the archive, so a save is a whole-file
 * operation. Showing that is the point.
 */
export async function checkSave(): Promise<void> {
  if (!summary.value || busy.value)
    return
  busy.value = true
  preview.value = null
  previewNotApplicable.value = null
  try {
    preview.value = await invoke<SavePreview>('preview_save', {
      path: summary.value.path
    })
  }
  catch (e) {
    preview.value = null
    const message = String(e)
    if (message.startsWith(NOT_APPLICABLE))
      previewNotApplicable.value = message.slice(NOT_APPLICABLE.length)
    else
      error.value = message
  }
  finally {
    busy.value = false
  }
}
