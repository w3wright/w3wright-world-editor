<script setup lang="ts">
/**
 * What a screen says when there is no map for it to show.
 *
 * # Why this is a component rather than a line per screen
 *
 * Four screens need it and they differ in one clause — no terrain, no units, no doodads, no
 * object data — so the alternative is four copies of the same heading and button with four
 * chances for the wording to drift.
 *
 * ⚠️ **Short on purpose.** This replaced a version that explained, on every one of the four
 * screens, that a map without the relevant member is a valid archive and that the core models
 * the file as optional. That is true, it is why the *panel* says it when the map is open, and
 * it is not what a user needs when the reason there is nothing here is that they have not
 * opened a file yet. The screen's own `what` clause carries the distinction instead.
 */
defineProps<{
  /**
   * What this screen would show, as a short noun phrase — "no terrain", "no units".
   *
   * ⚠️ It is not a sentence and the component does not treat it as one: it is appended to
   * "This screen shows", so a caller that writes a sentence produces a sentence spliced into
   * another. Keeping it a noun phrase is what makes one template work for four screens.
   */
  what: string
}>()

defineEmits<{ open: [] }>()
</script>

<template>
  <section class="page-body">
    <h1 class="page-title">
      No map is open
    </h1>

    <p class="m-0 mb-4 text-0.9rem opacity-70">
      This screen shows {{ what }}.
    </p>

    <button type="button" class="toolbar-btn" @click="$emit('open')">
      Open a map…
    </button>
  </section>
</template>
