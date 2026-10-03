import type { GameView, MapBrowser, SettingsView, StatusView } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

/**
 * The user's settings and the game the app would use, shared across the window.
 *
 * # Why module state here too
 *
 * Three places need the same answer: the title bar and the map overview want to know whether
 * launching is possible at all, the settings screen edits it, and the map browser decides from it
 * whether there is anything to browse. They are not a parent and a child — the title bar sits
 * outside the router view — so passing the value down would mean threading it through the shell
 * for no reason. This is the same argument `useOpenMap` makes for the open map, and the same
 * point applies: a store earns its place when a second *kind* of thing needs sharing.
 *
 * # Why the status is cached and re-read rather than pushed
 *
 * ⚠️ `status` is `null` until something asks. The window starts on the welcome screen, and
 * probing the registry for a game installation is a subprocess — doing it during startup would
 * pay for it on every launch, including the ones where the user opens a map from disk and never
 * touches the game directory. [`ensureStatus`] is called by the things that need the answer.
 */

/** The settings as the backend holds them. */
export const settings = ref<SettingsView | null>(null)

/** The game the app would use, and where the settings file is. */
export const status = ref<StatusView | null>(null)

/** A settings or game command is in flight. */
export const busy = ref(false)

/** A message for the user, shown verbatim. */
export const error = ref<string | null>(null)

/**
 * A one-off message for a thing that worked.
 *
 * Kept apart from `error` because the panel shows them in different places and with different
 * weight: "saved" belongs next to the button, and a failure belongs where it can be read.
 */
export const notice = ref<string | null>(null)

/**
 * How long a notice stays up.
 *
 * ⚠️ A notice that never expires is worse than none: it becomes a permanent line of text the
 * user has to read every time they look at the window, and they cannot dismiss it because there
 * is nothing to click. Six seconds is long enough to read one sentence and short enough that
 * nobody has to.
 */
const NOTICE_MS = 6000

let noticeTimer: ReturnType<typeof setTimeout> | null = null

/** Shows a notice, replacing any that is up and restarting the clock. */
export function showNotice(message: string): void {
  notice.value = message
  if (noticeTimer !== null)
    clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => {
    notice.value = null
    noticeTimer = null
  }, NOTICE_MS)
}

/** Takes a notice down. */
export function clearNotice(): void {
  if (noticeTimer !== null) {
    clearTimeout(noticeTimer)
    noticeTimer = null
  }
  notice.value = null
}

/** The game the app would use, or `null` when none was found. */
export const game = computed<GameView | null>(() => status.value?.game ?? null)

/**
 * Whether the app knows how to start the game.
 *
 * A map still has to be open to launch one, which is a separate question — this is only about
 * the installation. Callers that care about both say both.
 */
export const canLaunch = computed(() => game.value !== null)

/** Reads the settings and the game status, unless they have already been read. */
export async function ensureStatus(): Promise<void> {
  if (status.value !== null)
    return
  await refreshStatus()
}

/** Reads the settings and the game status again. */
export async function refreshStatus(): Promise<void> {
  busy.value = true
  error.value = null
  try {
    const [loaded, found] = await Promise.all([
      invoke<SettingsView>('settings_get'),
      invoke<StatusView>('war3_status')
    ])
    settings.value = loaded
    status.value = found
  }
  catch (e) {
    settings.value = null
    status.value = null
    error.value = String(e)
  }
  finally {
    busy.value = false
  }
}

/**
 * Checks one directory without saving it.
 *
 * Returns the installation, or `null` and leaves the reason in `error`. The check is the
 * backend's: "is this a Warcraft III directory" is answered by looking for `War3.exe`, and a
 * second opinion in the interface would be a second set of rules to keep in step.
 */
export async function checkWar3Dir(dir: string): Promise<GameView | null> {
  busy.value = true
  error.value = null
  clearNotice()
  try {
    return await invoke<GameView>('war3_check', { dir })
  }
  catch (e) {
    error.value = String(e)
    return null
  }
  finally {
    busy.value = false
  }
}

/**
 * Saves a game directory, or clears it.
 *
 * Passing an empty string clears the setting rather than storing an empty path — the two would
 * behave the same on read, but a file with `"war3Dir": ""` in it reads as a mistake.
 *
 * @returns whether it was saved.
 */
export async function saveWar3Dir(dir: string): Promise<boolean> {
  busy.value = true
  error.value = null
  clearNotice()
  try {
    settings.value = await invoke<SettingsView>('settings_save', {
      settings: { war3Dir: dir.trim(), path: settings.value?.path ?? null }
    })
    // The game may now resolve to a different installation, so the cached answer is stale.
    status.value = null
    await refreshStatus()
    // The notice expires on its own, so a save and a launch do not stack up as two lines the
    // user has to find and dismiss.
    showNotice(dir.trim() ? 'Saved.' : 'Cleared.')
    return true
  }
  catch (e) {
    error.value = String(e)
    return false
  }
  finally {
    busy.value = false
  }
}

/** Lists the maps installed under the game's `Maps` directory. */
export async function listMaps(): Promise<MapBrowser | null> {
  busy.value = true
  error.value = null
  try {
    return await invoke<MapBrowser>('maps_list')
  }
  catch (e) {
    error.value = String(e)
    return null
  }
  finally {
    busy.value = false
  }
}

/**
 * Starts the game on one map.
 *
 * `error` carries the reason when it fails — no installation, a map that has been moved, or a
 * game that would not start. All three are the user's to act on, so all three are shown.
 */
export async function launchGame(map: string): Promise<boolean> {
  busy.value = true
  error.value = null
  clearNotice()
  try {
    const started = await invoke<{ pid: number }>('game_launch', { map })
    // ⚠️ The process id is shown rather than a bare "started", because a command that returns
    // nothing after spawning leaves the panel unable to tell "started" from "the click did
    // nothing" — and starting a game is exactly the kind of action whose only visible effect is
    // in another window. The pid is also what a user would need to find or end the process.
    showNotice(`Started Warcraft III on this map (pid ${started.pid}).`)
    return true
  }
  catch (e) {
    error.value = String(e)
    return false
  }
  finally {
    busy.value = false
  }
}

/** Forgets the cached status, so the next [`ensureStatus`] reads it again. */
export function forgetStatus(): void {
  status.value = null
  settings.value = null
  error.value = null
  clearNotice()
}
