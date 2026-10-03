<script setup lang="ts">
import EmptyState from '~/components/EmptyState.vue'
/**
 * The map overview screen.
 *
 * The save preview lives here rather than under `Edit`, because it is a **report** and not
 * an edit: it answers "what would saving change" by rebuilding the archive in memory and
 * comparing two byte strings, and it writes nothing. Putting it behind an Edit menu entry
 * would promise an edit that does not exist.
 */
import MapInfoPanel from '~/components/MapInfoPanel.vue'
import { checkSave, preview, summary } from '~/composables/useOpenMap'
import { formatBytes } from '~/format'

defineEmits<{ open: [] }>()

/** The save-preview line, with the byte-identical case spelled out. */
function firstDifference(value: number | null): string {
  return value === null ? 'none — byte identical' : `offset ${value}`
}
</script>

<template>
  <EmptyState
    v-if="!summary"
    what="the map summary"
    @open="$emit('open')"
  />

  <div v-else class="flex flex-col">
    <MapInfoPanel :summary="summary" />

    <section class="page-body pt-0">
      <h2 class="section-head mt-0">
        saving
      </h2>
      <p class="m-0 mb-3 max-w-46rem text-0.85rem opacity-70">
        This asks the core to rebuild the archive in memory and compare the two byte
        strings. It writes nothing.
      </p>
      <p class="m-0">
        <button type="button" class="toolbar-btn" @click="checkSave">
          What would saving change?
        </button>
      </p>
    </section>

    <section v-if="preview && !preview.ok" class="callout-scope">
      <h2 class="callout-head">
        This question cannot be answered for this map
      </h2>
      <p class="callout-note">
        {{ preview.notApplicable }}
      </p>
      <p class="callout-because">
        The core is not refusing out of caution here — the member named above cannot be
        relocated at all, so no rebuild of this archive could be written without producing a
        member that cannot be read back. This is a limit of rebuilding, not a problem with
        your map.
      </p>
    </section>

    <section v-if="preview && preview.ok" class="callout-attention">
      <h2 class="callout-head">
        If this map were saved
      </h2>
      <p class="callout-note">
        {{ preview.note }}
      </p>
      <dl class="field-list max-w-40rem">
        <dt class="field-term">
          original
        </dt>
        <dd class="field-value">
          {{ formatBytes(preview.originalBytes) }}
        </dd>
        <dt class="field-term">
          rebuilt
        </dt>
        <dd class="field-value">
          {{ formatBytes(preview.rebuiltBytes) }}
        </dd>
        <dt class="field-term">
          members
        </dt>
        <dd class="field-value">
          {{ preview.memberCount }} carried through unchanged
        </dd>
        <dt class="field-term">
          first difference
        </dt>
        <dd class="field-value mono">
          {{ firstDifference(preview.firstDifference) }}
        </dd>
      </dl>
      <p class="callout-because">
        Editing and saving needs an in-place patch path in the core. Until that exists this
        reports rather than writes, so a save cannot rewrite your map without telling you.
      </p>
    </section>
  </div>
</template>
