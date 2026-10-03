import type { RouteLocationRaw } from 'vue-router'
import { ref } from 'vue'

/**
 * What the app can be asked to do, and where each thing lives.
 *
 * # One table, one navigation
 *
 * The menu bar is the whole of this app's navigation: its categories pop out over the window
 * and each entry goes somewhere or does something. There is no second list.
 *
 * That is an end point rather than an omission. Two earlier shapes were tried and both were
 * worse: a sidebar whose contents the menu bar swapped (navigation in two steps, and a menu
 * where a table of contents belonged), then a sidebar listing every screen beside pop-out
 * menus (which duplicated the menu and spent 13.5rem of width on a list of five items). A
 * screen here is one panel of read-only measurements, so a second pane has nothing to hold.
 *
 * # Screens and actions are different kinds of entry
 *
 * `Open Map…` and `Close Map` are not routes. Giving them one would put the window in a state
 * with no map and a URL claiming otherwise, so they are `action` entries and the component
 * that owns the shell dispatches them. `built` on a category means "this category has
 * anything to do"; an unimplemented category is listed and disabled rather than hidden,
 * because a reader comparing the app against `docs/03` §1.1 should be able to see that
 * `Edit` is planned and unbuilt — which a missing menu entry does not say.
 */

/** One screen: somewhere the window can be. */
export interface NavScreen {
  kind: 'screen'
  label: string
  /** A route path, which the router's generated table resolves. */
  to: RouteLocationRaw
}

/** One thing the window can do that is not a place. */
export interface NavAction {
  kind: 'action'
  label: string
  /** Matched by the component that dispatches it. */
  id: 'open' | 'browse' | 'close' | 'play'
  /**
   * What has to be true for this entry to be usable.
   *
   * Declared here rather than worked out in `MenuBar`, because it is a fact about what the
   * action *does* — `Close Map` needs a map open, launching needs a map **and** a game
   * installation — and a component that guessed at it would be the second place the rule lives.
   * An entry with nothing here is always usable.
   */
  requires?: {
    /** A map has to be open. */
    map?: boolean
    /** A Warcraft III installation has to be configured. */
    game?: boolean
  }
}

export type NavEntry = NavScreen | NavAction

/** One top-level category, as the menu bar lists it. */
export interface NavCategory {
  label: string
  /** `false` for a category the design set names but this increment does not implement. */
  built: boolean
  /** Why it is not built, for the disabled entry's tooltip. Empty when `built`. */
  why: string
  entries: NavEntry[]
}

/**
 * The menu, in the order `docs/03` §1.1 draws it.
 *
 * ⚠️ `File`, `Map` and `Objects` hold the read-only views that exist, plus the `File` actions.
 * `Edit` is the increment after this one — object data editing, which needs the save loop the
 * design set has verified but this app does not expose — and `Build` / `Help` arrive with
 * `war3-build` and a first-run document respectively. None of the three is stubbed, and each says
 * why on hover.
 *
 * `File` holds three ways in, and they are three because they answer different questions:
 * `Open Map…` is the system dialog and works for a map anywhere, `Browse Installed Maps…` skips
 * the remembering of which folder the game keeps maps in, and `Settings…` is where the game
 * directory those two need is set. Only the first works without a game installed, which is why
 * it is first.
 *
 * ⚠️ **`Run` is its own category rather than an entry in `File`.** Launching is the one action
 * here whose effect is *outside this window* — it starts another program — and putting it among
 * two ways of opening a file, a way of closing one and a settings screen would have made `File`
 * mean "everything". It sits next to `File` because it is the second thing a user reaches for
 * after opening a map, and it is the only category whose entries are all actions.
 */
export const NAV: NavCategory[] = [
  {
    label: 'File',
    built: true,
    why: '',
    entries: [
      { kind: 'action', label: 'Open Map…', id: 'open' },
      { kind: 'action', label: 'Browse Installed Maps…', id: 'browse' },
      { kind: 'action', label: 'Close Map', id: 'close', requires: { map: true } },
      { kind: 'screen', label: 'Settings', to: '/settings' }
    ]
  },
  {
    label: 'Run',
    built: true,
    why: '',
    entries: [
      { kind: 'action', label: 'Play in Warcraft III', id: 'play', requires: { map: true, game: true } }
    ]
  },
  {
    label: 'Edit',
    built: false,
    why: 'Editing arrives with the save loop. Object data is the first write the design set dares to make, because it round-trips byte for byte; nothing in this increment writes to a map.',
    entries: []
  },
  {
    label: 'Map',
    built: true,
    why: '',
    entries: [
      { kind: 'screen', label: 'Overview', to: '/map' },
      { kind: 'screen', label: 'Terrain', to: '/terrain' }
    ]
  },
  {
    label: 'Objects',
    built: true,
    why: '',
    entries: [
      { kind: 'screen', label: 'Units', to: '/units' },
      { kind: 'screen', label: 'Doodads', to: '/doodads' },
      { kind: 'screen', label: 'Object Data', to: '/objects' }
    ]
  },
  {
    label: 'Build',
    built: false,
    why: 'Build wraps `war3-build`, a core crate this app does not depend on yet. The command line already does it; the panel that shows its diagnostics is increment 5.',
    entries: []
  },
  {
    label: 'Help',
    built: false,
    why: 'There is nothing to document beyond what each panel says about itself, in place.',
    entries: []
  }
]

/**
 * The category whose menu is open, by label, or `null`.
 *
 * ⚠️ Module state rather than a `MenuBar` ref, and the reason is where the *menu* lives rather
 * than convenience: the popup is anchored to the bar but is a full-window layer while it is
 * open, and `MenuBar` has to re-run a `watch` on this to move focus into the popup. Keeping the
 * flag beside the table it belongs to also means the next reader finds the menu's state in
 * `nav.ts` rather than inside a component, exactly as `useOpenMap` keeps the open map out of a
 * panel.
 */
export const openMenu = ref<string | null>(null)

/** Closes whatever menu is open. */
export function closeMenu(): void {
  openMenu.value = null
}

/** Opens a category's menu, or closes it when it is already the open one. */
export function toggleMenu(label: string): void {
  openMenu.value = openMenu.value === label ? null : label
}

/** The entries of a category, or an empty list for a label that has none. */
export function entriesOf(label: string): NavEntry[] {
  return NAV.find(category => category.label === label)?.entries ?? []
}

/** The path of a route without its query, fragment or trailing slash. */
function normalise(route: string): string {
  return route.split(/[?#]/)[0].replace(/\/+$/, '') || '/'
}

/**
 * The category holding a route, or `null` when no category does.
 *
 * Matching is on the path alone: a query string is no part of a screen's identity, so
 * `/map?verbose=1` still belongs to `Map`. Returning `null` for an unknown route lets a
 * caller say so rather than pointing at an arbitrary category.
 */
export function categoryFor(route: string): NavCategory | null {
  const path = normalise(route)
  for (const category of NAV) {
    const found = category.entries.some(
      entry => entry.kind === 'screen' && normalise(String(entry.to)) === path
    )
    if (found)
      return category
  }
  return null
}

/** Whether a route is the one on display. */
export function isCurrentRoute(route: string, to: RouteLocationRaw): boolean {
  return normalise(String(to)) === normalise(route)
}

/** The action with this id, for a component that wants its label. */
export function actionFor(id: NavAction['id']): NavAction | null {
  for (const category of NAV) {
    for (const entry of category.entries) {
      if (entry.kind === 'action' && entry.id === id)
        return entry
    }
  }
  return null
}
