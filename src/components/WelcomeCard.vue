<script setup lang="ts">
/**
 * The screen the app opens on: one sentence and a button.
 *
 * # Why this is its own screen rather than the map overview's empty state
 *
 * The overview's empty state is what you see *after* closing a map. The welcome screen is what
 * you see before you have ever had one open, and the only useful thing on it is the way in.
 * Sharing one component between the two meant writing copy for the more informative case and
 * showing it in both.
 *
 * ⚠️ **It is deliberately this short.** An earlier version explained the menus, the read-only
 * guarantee and where the data comes from; a first run that opens with three paragraphs is one
 * a user scrolls past looking for the button. What is left is the action, one sentence naming
 * the keyboard-free route to it again later, and the read-only fact — which a user about to
 * point this at a map they care about is entitled to know before they click.
 */
import { game } from '../composables/useSettings'

defineEmits<{ open: [], browse: [] }>()
</script>

<template>
  <section class="welcome">
    <h1 class="page-title">
      Open a Warcraft III map
    </h1>

    <p class="m-0 mb-5 mt-4 flex items-center gap-2.5">
      <button type="button" class="welcome-btn" @click="$emit('open')">
        Open a map…
      </button>
      <!--
        Offered only when it would work. A second button that leads to "no game directory is set"
        is worse than one button: at that point the user has to go and find Settings before
        anything they clicked does anything.
      -->
      <button
        v-if="game?.hasMaps"
        type="button"
        class="welcome-btn secondary"
        @click="$emit('browse')"
      >
        Browse installed maps
      </button>
    </p>

    <p class="m-0 max-w-38rem text-0.82rem opacity-60">
      Read-only: nothing here writes to your file. Later, <strong>File ▸ Open Map…</strong>
      opens another.
    </p>
  </section>
</template>

<style scoped>
/*
 * Centred, and not with the page frame every other screen uses. This is the only screen whose
 * content is not a body of measurements, and a single heading and button pinned to the top-left
 * of an empty 1180×780 window reads as an unfinished screen rather than a deliberate one.
 *
 * ⚠️ `place-content`, not `place-items`. In a full-height grid, `place-items: center` centres
 * each row inside its own share of the height, which spread the heading, the button and the note
 * into three separate bands with a gap between them. `place-content` centres the track set, so
 * the three lines stay together as one block.
 */
.welcome {
  display: grid;
  place-content: center;
  justify-items: center;
  row-gap: 0.25rem;
  height: 100%;
  padding: 2rem;
}

/*
 * The one button on this screen is the only thing to press, so it is larger than the toolbar
 * buttons and filled rather than outlined. A scoped rule rather than a shortcut because this is
 * the app's only primary action — a shortcut used at one call site hides more than it saves.
 *
 * The colour is a literal rather than a theme token: it is the one accent in an otherwise grey
 * app, and a token for a single use is a variable nobody can change safely.
 */
.welcome-btn {
  padding: 0.5rem 1.15rem;
  font-size: 0.92rem;
  font-family: inherit;
  color: #fff;
  border: 0;
  border-radius: 6px;
  background: #2f6feb;
  cursor: default;
}

.welcome-btn:hover {
  background: #2a63d2;
}

.welcome-btn:active {
  background: #2559bd;
}

/*
 * The second way in, outlined rather than filled. Both buttons lead to a map, but one uses the
 * system dialog and works anywhere while the other depends on a game being installed — so they
 * are not two equal choices, and only the one that always works gets the fill.
 */
.welcome-btn.secondary {
  color: inherit;
  background: transparent;
  border: 1px solid rgb(0 0 0 / 22%);
}

.welcome-btn.secondary:hover {
  background: rgb(0 0 0 / 5%);
}

@media (prefers-color-scheme: dark) {
  .welcome-btn.secondary {
    border-color: rgb(255 255 255 / 26%);
  }

  .welcome-btn.secondary:hover {
    background: rgb(255 255 255 / 8%);
  }
}

.welcome-btn:focus-visible {
  outline: 2px solid #2f6feb;
  outline-offset: 2px;
}
</style>
