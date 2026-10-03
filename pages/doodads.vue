<script setup lang="ts">
/**
 * The doodads screen.
 *
 * The parse is deferred to this route because this is the largest record list a map holds —
 * 5,317 of them on `(4)LostTemple.w3m` — and most visits never look at it.
 */
import { onMounted } from 'vue'
import DoodadPanel from '~/components/DoodadPanel.vue'
import EmptyState from '~/components/EmptyState.vue'
import { doodads, doodadsBusy, loadDoodads, summary } from '~/composables/useOpenMap'

defineEmits<{ open: [] }>()

onMounted(loadDoodads)
</script>

<template>
  <EmptyState
    v-if="!summary"
    what="the doodads placed on a map"
    @open="$emit('open')"
  />

  <p v-else-if="doodadsBusy" class="hint">
    Reading doodads…
  </p>

  <DoodadPanel v-else-if="doodads" :doodads="doodads" />

  <p v-else class="hint">
    Nothing was read. Reloading may help; the reason is shown with the errors.
  </p>
</template>
