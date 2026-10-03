import { getCurrentWindow } from '@tauri-apps/api/window'
import { onBeforeUnmount, ref } from 'vue'

/**
 * The window's own controls, for a window that draws its own title bar.
 *
 * # Why a composable
 *
 * The state and the four calls belong together and only ever have one consumer at a time,
 * but the state is *window* state rather than component state: which is exactly the thing
 * that goes stale. `maximized` is kept in step with the window rather than toggled
 * locally, because a user can maximize by double-clicking the drag region or by dragging
 * the window against a screen edge, and a button whose glyph disagrees with the window is
 * worse than no glyph.
 *
 * # Why the failures are logged and swallowed
 *
 * A denied or missing permission comes back as a rejected promise. Letting it reach the
 * console as an unhandled rejection would produce a stack trace with no sentence in it;
 * these calls have no user-facing failure mode — if `close` does not work, the window is
 * still there and the title bar is still on screen — so the honest thing is one line that
 * names what failed.
 */

/** The current window, named once. The label is fixed in `tauri.conf.json`. */
const window = getCurrentWindow()

/** Whether the window is maximized, kept in step with the window itself. */
export const maximized = ref(false)

/**
 * Reads the window's real state.
 *
 * Called on resize rather than on the button, which is what makes a double-click on the
 * drag region and a Windows snap update the glyph too.
 */
export async function refreshMaximized(): Promise<void> {
  try {
    maximized.value = await window.isMaximized()
  }
  catch (e) {
    report('read the maximized state', e)
  }
}

/** Minimizes to the taskbar. */
export async function minimize(): Promise<void> {
  try {
    await window.minimize()
  }
  catch (e) {
    report('minimize', e)
  }
}

/** Maximizes, or restores when already maximized. */
export async function toggleMaximize(): Promise<void> {
  try {
    await window.toggleMaximize()
    await refreshMaximized()
  }
  catch (e) {
    report('toggle maximize', e)
  }
}

/** Closes the window. */
export async function close(): Promise<void> {
  try {
    await window.close()
  }
  catch (e) {
    report('close', e)
  }
}

function report(what: string, e: unknown): void {
  console.warn(`could not ${what} the window:`, e)
}

/**
 * Starts tracking the window's state, and stops when the caller unmounts.
 *
 * The unlisten is registered here rather than left to the component so that a title bar
 * that is remounted cannot leave a second listener behind — two listeners would each call
 * `refreshMaximized`, which is harmless but is exactly the leak that shows up later as a
 * window that resizes slowly.
 */
export function useWindowState(): void {
  let stop: (() => void) | null = null
  void refreshMaximized()
  void window
    .onResized(() => {
      void refreshMaximized()
    })
    .then((unlisten) => {
      stop = unlisten
    })
    .catch((e: unknown) => {
      report('watch for resizes', e)
    })
  onBeforeUnmount(() => stop?.())
}
