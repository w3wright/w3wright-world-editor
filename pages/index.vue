<script setup lang="ts">
/**
 * The map screen: what the core says about the open map.
 *
 * The open box lives in the shell rather than here, because the shell outlives the
 * route. An input inside a page is destroyed when the route changes, which would clear
 * what the user typed the moment they looked at another screen.
 */
import MapInfoPanel from '~/components/MapInfoPanel.vue'
import {
  checkSave,
  preview,
  previewNotApplicable,
  summary
} from '~/composables/useOpenMap'
import { formatBytes } from '~/format'

/** The save-preview line, with the byte-identical case spelled out. */
function firstDifference(value: number | null): string {
  return value === null ? 'none — byte identical' : `offset ${value}`
}
</script>

<template>
  <div class="flex flex-col">
    <p v-if="!summary" class="hint">
      Enter the path to a map and press Open. Everything shown comes from the
      <code class="text-0.85em font-mono">w3wright</code> core; this screen holds no
      format knowledge of its own.
    </p>

    <template v-else>
      <p class="mx-6 mt-3">
        <button type="button" class="toolbar-btn" @click="checkSave">
          What would saving change?
        </button>
      </p>

      <section v-if="previewNotApplicable" class="callout-scope">
        <h2 class="callout-head">
          This question cannot be answered for this map
        </h2>
        <p class="callout-note">
          {{ previewNotApplicable }}
        </p>
        <p class="callout-because">
          The core is not refusing out of caution here — the member named above cannot be
          relocated at all, so no rebuild of this archive could be written without
          producing a member that cannot be read back. This is a limit of rebuilding, not
          a problem with your map.
        </p>
      </section>

      <section v-if="preview" class="callout-attention">
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
          Editing and saving needs an in-place patch path in the core. Until that exists
          this reports rather than writes, so a save cannot rewrite your map without
          telling you.
        </p>
      </section>

      <MapInfoPanel :summary="summary" />
    </template>
  </div>
</template>
