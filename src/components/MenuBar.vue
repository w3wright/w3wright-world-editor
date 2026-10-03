<script setup lang="ts">
/**
 * The menu bar: the top-level categories, and the menus they pop out.
 *
 * # Why the menus pop out rather than swapping a panel beside them
 *
 * An earlier version opened a category's entries in a sidebar. That made the menu bar a
 * two-step navigation rather than a menu, and it spent a column of the window on a list of
 * five items. A menu that opens over the window and closes when something is picked is the
 * shape every desktop app has; nothing here needs a second pane.
 *
 * # What the popup has to get right
 *
 * Three things, each a way a hand-rolled menu goes wrong:
 *
 * 1. **It closes on anything that is not it.** A transparent full-window layer sits under
 *    the popup and closes on click — the same effect as a `mousedown` listener on the
 *    document, with no listener to leak and no `stopPropagation` to forget.
 * 2. **A click on it does not reach what is behind it.** The layer covers the screen, so a
 *    control under the popup cannot be hit through it.
 * 3. **Escape closes it and hands focus back to the button that opened it.** Without that
 *    return, a keyboard user is left wherever the browser decided to put them.
 *
 * ⚠️ Nothing here mirrors `openMenu` into a local ref. One source of truth, in `nav.ts`,
 * because the open menu is a fact about the window rather than about this component.
 */
import type { NavAction, NavEntry, NavScreen } from '../nav'
import { nextTick, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { closeMenu, isCurrentRoute, NAV, openMenu, toggleMenu } from '../nav'

const props = defineProps<{
  /** Whether a map is open, so `Close Map` can be disabled rather than fail. */
  hasMap: boolean
}>()

const emit = defineEmits<{
  /** A screen was chosen; the shell navigates. */
  go: [to: NavScreen['to']]
  /** An action was chosen; the shell dispatches it. */
  action: [id: NavAction['id']]
}>()

const route = useRoute()

/** The category buttons, so a closed menu can give focus back to its own. */
const buttons = ref<Record<string, HTMLButtonElement | null>>({})
/** The popup, so focus can move into it when it opens. */
const popup = ref<HTMLElement | null>(null)

watch(openMenu, (label) => {
  if (label)
    void nextTick(() => popup.value?.querySelector<HTMLElement>('.menu-item')?.focus())
})

/** The entries of the open menu. */
function entries(): NavEntry[] {
  return NAV.find(category => category.label === openMenu.value)?.entries ?? []
}

/** Escape closes the menu and returns focus to the button that opened it. */
function onEscape(label: string): void {
  closeMenu()
  void nextTick(() => buttons.value[label]?.focus())
}

/** A screen entry: close the menu first, then let the shell navigate. */
function onScreen(entry: NavScreen): void {
  closeMenu()
  emit('go', entry.to)
}

/** An action entry: close the menu first, then let the shell dispatch. */
function onAction(entry: NavAction): void {
  closeMenu()
  emit('action', entry.id)
}

/** `Close Map` has nothing to do without a map, so it says so instead of failing. */
function isDisabled(entry: NavEntry): boolean {
  // Only actions can be disabled today; a screen is always somewhere to go.
  if (entry.kind !== 'action')
    return false
  return entry.id === 'close' && !props.hasMap
}

/** Whether the route on display is this screen. */
function current(to: NavScreen['to']): boolean {
  return isCurrentRoute(String(route.path), to)
}
</script>

<template>
  <nav
    class="relative h-8 fyc flex-none gap-0.5 border-b border-b-line-soft px-1.5"
    aria-label="Main menu"
  >
    <button
      v-for="category in NAV"
      :key="category.label"
      :ref="(el) => { buttons[category.label] = (el as HTMLButtonElement | null) }"
      type="button"
      class="menu-btn"
      :class="{
        'menu-btn-open': category.label === openMenu,
        'menu-btn-planned': !category.built
      }"
      :disabled="!category.built"
      :title="category.built ? `${category.label} menu` : `${category.label} — not built yet. ${category.why}`"
      aria-haspopup="menu"
      :aria-expanded="category.label === openMenu"
      @click="toggleMenu(category.label)"
      @keydown.esc="onEscape(category.label)"
    >
      {{ category.label }}
    </button>

    <!--
      The outside-click layer and the popup are siblings, both after the buttons, so the
      popup paints above the layer and a click on an entry is not intercepted by it. The
      layer spans the window rather than the menu bar: a menu that only closed when you
      clicked back on the bar would not be a menu.
    -->
    <template v-if="openMenu !== null">
      <div class="fixed inset-0 z-40" @click="closeMenu()" />
      <div
        ref="popup"
        class="menu-popup z-50"
        role="menu"
        :aria-label="openMenu"
        @keydown.esc="onEscape(openMenu)"
      >
        <button
          v-for="entry in entries()"
          :key="entry.label"
          type="button"
          role="menuitem"
          class="menu-item"
          :disabled="isDisabled(entry)"
          :title="isDisabled(entry) ? 'No map is open' : entry.label"
          @click="entry.kind === 'action' ? onAction(entry) : onScreen(entry)"
        >
          <span>{{ entry.label }}</span>
          <!--
            A tick for the screen you are on, rather than a highlighted row: the menu closes
            on a click, so the tick is not standing in for an active state — it is what makes
            the menu readable at the moment it opens.
          -->
          <span
            v-if="entry.kind === 'screen' && current(entry.to)"
            class="menu-tick"
            aria-hidden="true"
          >✓</span>
        </button>

        <p v-if="entries().length === 0" class="menu-empty">
          Nothing here yet.
        </p>
      </div>
    </template>
  </nav>
</template>

<style scoped>
/*
 * The menu's rows and elevation, kept in CSS because it is a layer with its own anchoring
 * and per-state rows. `position: absolute` against the `relative` bar, so the popup floats
 * over the screen without pushing the layout; the inline `z-40`/`z-50` put it above the
 * panel. It is anchored to the bar rather than to the category button on purpose: a
 * per-button offset would have to be measured, and the popup would jump as labels change
 * width.
 *
 * ⚠️ The opaque surface is **not** here. It cannot be a utility: UnoCSS's `dark:` variant
 * compiles to a `.dark` ancestor class, and this app follows the system scheme instead (see
 * `base.css`), so `dark:bg-…` matches nothing and the popup stayed white on a dark window —
 * unreadable text on white, which is how it was caught. The surface lives in `base.css` with
 * the media query that every other themed value uses.
 */
.menu-popup {
  position: absolute;
  top: 100%;
  left: 0.375rem;
  min-width: 13rem;
  padding: 0.25rem;
  border: 1px solid rgb(128 128 128 / 0.35);
  border-radius: 6px;
  box-shadow: 0 8px 24px rgb(0 0 0 / 0.22);
}

.menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  width: 100%;
  padding: 0.3rem 0.5rem;
  font-size: 0.83rem;
  font-family: inherit;
  color: inherit;
  text-align: left;
  border: 0;
  border-radius: 4px;
  background: transparent;
  cursor: default;
}

.menu-item:hover:not(:disabled),
.menu-item:focus-visible {
  background: rgb(128 128 128 / 0.18);
  outline: none;
}

.menu-item:disabled {
  opacity: 0.4;
}

.menu-tick {
  opacity: 0.7;
}

/* Chrome hides the marker, because an open menu is shown by the popup and a screen's
 * category by the header — a third mark would be noise. */
.menu-btn {
  padding: 0.2rem 0.7rem;
  font-size: 0.82rem;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: inherit;
  font-family: inherit;
  cursor: default;
  opacity: 0.65;
}

.menu-btn:hover:not(:disabled) {
  opacity: 1;
  background: rgb(128 128 128 / 0.12);
}

.menu-btn-open {
  opacity: 1;
  background: rgb(128 128 128 / 0.22);
}

.menu-btn-planned {
  opacity: 0.32;
}
</style>
