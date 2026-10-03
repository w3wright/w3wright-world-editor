<script setup lang="ts">
/**
 * The units screen.
 *
 * Loading happens here rather than in the shell, so a map that is opened but never
 * inspected for units does not pay for the parse. `loadUnits` keeps what it has already
 * fetched, so returning to this route does not re-parse. See `pages/terrain.vue`, which
 * this mirrors.
 */
import { onMounted } from 'vue'
import EmptyState from '~/components/EmptyState.vue'
import UnitPanel from '~/components/UnitPanel.vue'
import { loadUnits, summary, units, unitsBusy } from '~/composables/useOpenMap'

defineEmits<{ open: [] }>()

onMounted(loadUnits)
</script>

<template>
  <EmptyState
    v-if="!summary"
    what="the units placed on a map"
    @open="$emit('open')"
  />

  <p v-else-if="unitsBusy" class="hint">
    Reading units…
  </p>

  <UnitPanel v-else-if="units" :units="units" />

  <p v-else class="hint">
    Nothing was read. Reloading may help; the reason is shown with the errors.
  </p>
</template>
