<script setup lang="ts">
import { busy, error, summary } from '../composables/useOpenMap'
import { close, maximized, minimize, toggleMaximize, useWindowState } from '../composables/useWindow'
/**
 * The window's own title bar.
 *
 * # Why the window draws it
 *
 * The native one cannot hold the app's own state — which map is open, whether a command
 * is running — and on Windows it is drawn by the system in a font and colour that ignore
 * the app's light/dark scheme. `decorations: false` in `tauri.conf.json` removes it, and
 * everything it did has to be done here: the drag region, the double-click-to-maximize
 * behaviour, and the three buttons.
 *
 * # What this component may not do
 *
 * ⚠️ No map knowledge. It shows the *name* of the open map because the shell already has
 * it, and nothing here inspects a path or decides what a file is. That line is `docs/03`
 * §1.1's, and it applies to the chrome as much as to the panels.
 */
import { fileName } from '../format'

useWindowState()
</script>

<template>
  <header
    data-tauri-drag-region
    class="h-9 fbc flex-none select-none border-b border-b-line-soft bg-[rgb(128_128_128_/_0.06)] pl-3"
  >
    <!--
      ⚠️ Every non-button child of the bar carries the drag attribute, and every one of
      them is `h-full`. Tauri starts a drag only for the element the pointer is on, so a
      36px-tall bar whose text spans are 18px tall leaves two dead strips that a user will
      find by trying to drag the window by the title. The buttons below deliberately do not
      carry it: a press there must not also move the window.
    -->
    <span data-tauri-drag-region class="h-full min-w-0 fyc gap-2.5">
      <span data-tauri-drag-region class="flex-none text-0.8rem font-600 tracking-0.01em">W3wright World Editor</span>

      <span
        v-if="summary"
        data-tauri-drag-region
        class="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap text-0.78rem opacity-60"
        :title="summary.path"
      >
        — {{ fileName(summary.path) }}
      </span>

      <span
        v-if="busy"
        data-tauri-drag-region
        class="h-full fyc flex-none text-0.75rem opacity-60"
      >
        working…
      </span>
      <span
        v-else-if="error"
        data-tauri-drag-region
        class="h-full fyc flex-none text-0.75rem text-red-600"
        title="the last command failed; the message is in the content area"
      >
        error
      </span>
    </span>

    <!--
      The three controls. Not drag regions: a click on a button must not also start a
      window drag, and Tauri's drag-region handling skips elements it recognises as
      interactive — but the attribute is omitted here anyway so the behaviour does not
      rest on that detail.
    -->
    <span class="h-full fyc flex-none">
      <button
        type="button"
        class="win-btn"
        aria-label="Minimize"
        title="Minimize"
        @click="minimize"
      >
        <svg viewBox="0 0 10 10" class="win-glyph" aria-hidden="true">
          <path d="M0 5h10" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>

      <button
        type="button"
        class="win-btn"
        :aria-label="maximized ? 'Restore' : 'Maximize'"
        :title="maximized ? 'Restore' : 'Maximize'"
        @click="toggleMaximize"
      >
        <svg viewBox="0 0 10 10" class="win-glyph" aria-hidden="true">
          <rect
            x="0.5"
            y="0.5"
            width="9"
            height="9"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
          />
        </svg>
      </button>

      <button
        type="button"
        class="win-btn win-btn-close"
        aria-label="Close"
        title="Close"
        @click="close"
      >
        <svg viewBox="0 0 10 10" class="win-glyph" aria-hidden="true">
          <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
    </span>
  </header>
</template>

<style scoped>
/*
 * The window buttons, kept in CSS rather than utilities because their whole point is the
 * hover and active surfaces, which is a state rather than a set of declarations: Windows'
 * own buttons light up on hover and dim on press, and spelling that out as utility
 * variants on three elements would be longer and harder to read than this.
 *
 * The sizes are the platform's: a 46×36 hit target with a 10px glyph is what Segoe
 * Fluent's caption buttons use, and a smaller one reads as a web page rather than as a
 * window.
 */
.win-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2.9rem;
  height: 100%;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: default;
  outline: none;
}

.win-btn:hover {
  background: rgb(128 128 128 / 0.22);
}

.win-btn:active {
  background: rgb(128 128 128 / 0.34);
}

/* The close button is the one control with its own colour, and only on hover: at rest all
 * three must read as one group. `#c42b1c` is the system's own close-hover red. */
.win-btn-close:hover {
  background: #c42b1c;
  color: #fff;
}

.win-btn-close:active {
  background: #b1271a;
  color: #fff;
}

.win-glyph {
  width: 0.62rem;
  height: 0.62rem;
  display: block;
}
</style>
