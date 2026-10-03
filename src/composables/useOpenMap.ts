import type {
  DoodadView,
  MapSummary,
  ObjectView,
  SavePreview,
  TerrainView,
  UnitView
} from '../types'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
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
 *
 * # Why there is no path field any more
 *
 * A path typed by hand was the only way in until the window drew its own title bar. With
 * no native menu there is nowhere for a stray text box to live, and a dialog is strictly
 * better at the same job — it cannot produce a typo, and on Windows it offers the game's
 * own map folders. So `openPath` now *takes* a path rather than reading one out of a
 * field, and the dialog is the caller.
 */

/** The open map, or `null` when nothing has been opened. */
export const summary = ref<MapSummary | null>(null)

/** A message for the user, from the core, shown verbatim. */
export const error = ref<string | null>(null)

/** A command is in flight. */
export const busy = ref(false)

/** The save preview, once asked for. */
export const preview = ref<SavePreview | null>(null)

/** The terrain of the open map, or `null` when it has not been asked for. */
export const terrain = ref<TerrainView | null>(null)

/** The terrain command is in flight. */
export const terrainBusy = ref(false)

/** The units of the open map, or `null` when they have not been asked for. */
export const units = ref<UnitView | null>(null)

/** The units command is in flight. */
export const unitsBusy = ref(false)

/** The doodads of the open map, or `null` when they have not been asked for. */
export const doodads = ref<DoodadView | null>(null)

/** The doodads command is in flight. */
export const doodadsBusy = ref(false)

/** The object data of the open map, or `null` when it has not been asked for. */
export const objects = ref<ObjectView | null>(null)

/** The objects command is in flight. */
export const objectsBusy = ref(false)

/**
 * Clears the per-map data a new map makes meaningless.
 *
 * One list rather than a line per view: opening a different map invalidates every
 * parse of the previous one, and a view added without a line here is one that shows
 * another map's data. `summary` is set by the caller once the new map really opened.
 */
function forgetViews(): void {
  terrain.value = null
  units.value = null
  doodads.value = null
  objects.value = null
  terrainBusy.value = false
  unitsBusy.value = false
  doodadsBusy.value = false
  objectsBusy.value = false
}

/**
 * Opens a map from a path the caller already has.
 *
 * The backend decides whether it is a map and says why not when it is not, so there is no
 * validation here beyond "a path was given". A second opinion in the interface would be a
 * second set of rules to keep in step — and with the dialog as the only source, the
 * interesting failures are all the core's.
 *
 * @returns whether the map opened, so a caller can navigate on success only.
 */
export async function openPath(target: string): Promise<boolean> {
  const wanted = target.trim()
  if (!wanted || busy.value)
    return false
  busy.value = true
  error.value = null
  preview.value = null
  // Opening a different map invalidates everything read from the previous map.
  // Leaving it would show one map's terrain under another map's name.
  forgetViews()
  try {
    summary.value = await invoke<MapSummary>('open_map', { path: wanted })
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
 * Asks the system for a map, opens it, and returns the path it opened.
 *
 * # Why the filter does not decide anything
 *
 * The extensions below are what the *picker* offers, not a claim about the format: the
 * core is what decides whether a file is a map, and it will say so if one of these is not.
 * They are here because a dialog showing every replay and screenshot in a folder is a
 * worse dialog, and because `.w3m` / `.w3x` / `.w3n` are the names the game itself uses.
 *
 * # Why this returns the path rather than navigating
 *
 * The composable has no router, and giving it one would make "where the window is" a
 * concern of "what the core said" — the two facts this codebase keeps apart everywhere
 * else. The caller that owns the shell does the push.
 *
 * @returns the opened path, or `null` when the dialog was cancelled or the map could not
 * be read. A cancelled dialog is not a failure and leaves everything as it was.
 */
export async function openPickedMap(): Promise<string | null> {
  if (busy.value)
    return null
  let chosen: string | null = null
  try {
    chosen = await openDialog({
      multiple: false,
      directory: false,
      title: 'Open a Warcraft III map',
      filters: [{ name: 'Warcraft III map', extensions: ['w3x', 'w3m', 'w3n'] }]
    })
  }
  catch (e) {
    // The dialog itself failing is a real fault — a missing plugin registration, a denied
    // permission — and it must not look like the user cancelling.
    error.value = `the file dialog could not be opened: ${String(e)}`
    return null
  }
  // `null` is a cancelled dialog, which is not an error and not a reason to clear the map
  // that may already be open.
  if (chosen === null)
    return null
  return (await openPath(chosen)) ? chosen : null
}

/** Closes the open map and everything read from it. */
export function closeMap(): void {
  summary.value = null
  error.value = null
  preview.value = null
  forgetViews()
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
 * Loads the units on first use, for the same reason and with the same caching as
 * [`loadTerrain`].
 */
export async function loadUnits(): Promise<void> {
  if (!summary.value || units.value || unitsBusy.value)
    return
  unitsBusy.value = true
  try {
    units.value = await invoke<UnitView>('read_units', {
      path: summary.value.path
    })
  }
  catch (e) {
    units.value = null
    error.value = String(e)
  }
  finally {
    unitsBusy.value = false
  }
}

/** Loads the doodads on first use. */
export async function loadDoodads(): Promise<void> {
  if (!summary.value || doodads.value || doodadsBusy.value)
    return
  doodadsBusy.value = true
  try {
    doodads.value = await invoke<DoodadView>('read_doodads', {
      path: summary.value.path
    })
  }
  catch (e) {
    doodads.value = null
    error.value = String(e)
  }
  finally {
    doodadsBusy.value = false
  }
}

/** Loads the object data on first use. */
export async function loadObjects(): Promise<void> {
  if (!summary.value || objects.value || objectsBusy.value)
    return
  objectsBusy.value = true
  try {
    objects.value = await invoke<ObjectView>('read_objects', {
      path: summary.value.path
    })
  }
  catch (e) {
    objects.value = null
    error.value = String(e)
  }
  finally {
    objectsBusy.value = false
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
  try {
    preview.value = await invoke<SavePreview>('preview_save', {
      path: summary.value.path
    })
  }
  catch (e) {
    preview.value = null
    // ⚠️ Not classified here any more. A map the core cannot rebuild is a **limitation of that
    // map**, not a failure, and the distinction lives in the core's own error type
    // (`MpqError::is_not_applicable`). This used to look for a `not-applicable: ` marker that
    // `commands.rs` glued onto the message, which meant both sides had to keep a string in step.
    error.value = String(e)
  }
  finally {
    busy.value = false
  }
}
