<script setup lang="ts">
/**
 * The objects screen.
 *
 * The seven object files are read from the archive rather than through the map model,
 * because the map model does not carry them — see `read_objects` in `commands.rs`.
 */
import { onMounted } from 'vue'
import EmptyState from '~/components/EmptyState.vue'
import ObjectPanel from '~/components/ObjectPanel.vue'
import { loadObjects, objects, objectsBusy, summary } from '~/composables/useOpenMap'

defineEmits<{ open: [] }>()

onMounted(loadObjects)
</script>

<template>
  <EmptyState
    v-if="!summary"
    what="a map's object data"
    @open="$emit('open')"
  />

  <p v-else-if="objectsBusy" class="hint">
    Reading objects…
  </p>

  <ObjectPanel v-else-if="objects" :objects="objects" />

  <p v-else class="hint">
    Nothing was read. Reloading may help; the reason is shown with the errors.
  </p>
</template>
