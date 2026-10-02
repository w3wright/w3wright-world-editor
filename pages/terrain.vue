<script setup lang="ts">
/**
 * The terrain screen.
 *
 * Loading happens here rather than in the shell so the parse is paid for when this
 * route is entered, not when a map is opened: terrain for a 161×161 map is 25,921 tile
 * points, and most visits never look at it. `loadTerrain` keeps what it has already
 * fetched, so returning to this route does not re-parse.
 */
import { onMounted } from 'vue'
import TerrainPanel from '~/components/TerrainPanel.vue'
import { loadTerrain, summary, terrain, terrainBusy } from '~/composables/useOpenMap'

onMounted(loadTerrain)
</script>

<template>
  <div class="flex flex-col">
    <p v-if="!summary" class="hint">
      No map is open.
      <RouterLink to="/" class="color-inherit underline">
        Open one
      </RouterLink>
      to see its terrain.
    </p>

    <p v-else-if="terrainBusy" class="hint">
      Reading terrain…
    </p>

    <TerrainPanel v-else-if="terrain" :terrain="terrain" />

    <p v-else class="hint">
      Nothing was read. Reloading may help; the reason is shown with the errors.
    </p>
  </div>
</template>
