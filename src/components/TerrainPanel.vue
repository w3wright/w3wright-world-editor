<script setup lang="ts">
/**
 * The terrain view: what `.w3e` holds, as the core measured it.
 *
 * Read-only, and deliberately so. Nothing here derives a value — the height range, the
 * layer histogram and the flag counts all arrive computed from `war3-terrain`, because
 * recomputing them in the interface would be a second implementation of the format. See
 * `docs/03` §1.1.
 */
import type { TerrainView } from '../types'
import { computed } from 'vue'
import { formatBytes } from '../format'

const props = defineProps<{ terrain: TerrainView }>()

/** The histogram as rows with each bucket's share, biggest first. */
const layers = computed(() => {
  const info = props.terrain.info
  if (!info)
    return []
  const total = info.tileCount || 1
  return info.layerHistogram
    .map((count, index) => ({
      index,
      count,
      share: count / total,
      id: info.groundTextureIds[index] ?? null
    }))
    .filter(layer => layer.count > 0)
    .sort((a, b) => b.count - a.count)
})

/** A rough size for the tile point records, for the layout section. */
const recordBytes = computed(() => {
  const info = props.terrain.info
  return info ? info.tileCount * info.recordSize : 0
})

/** Percentages, because the template should not be doing arithmetic. */
function percent(share: number): string {
  return `${(share * 100).toFixed(1)}%`
}

/**
 * The bar width. Two decimals rather than one: at this scale a 0.1% step is about a
 * pixel, and one decimal would collapse several distinct layers onto the same bar.
 */
function barWidth(share: number): string {
  return `${(share * 100).toFixed(2)}%`
}
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        Terrain
      </h1>
      <p class="page-path">
        {{ terrain.path }}
      </p>
    </header>

    <template v-if="!terrain.ok || !terrain.info">
      <p class="m-0 mb-2.5 border-l-3 border-slate-400 pl-3 text-0.9rem">
        <strong>This map has no readable terrain.</strong>
        {{ terrain.error }}
      </p>
      <p class="m-0 max-w-46rem text-0.85rem opacity-70">
        That is a legitimate state rather than a failure: a map without
        <code class="text-0.9em font-mono">war3map.w3e</code> is still a valid archive, so
        the core models terrain as optional and this view reports the reason instead of
        calling it an error.
      </p>
    </template>

    <template v-else>
      <h2 class="section-head">
        layout
      </h2>
      <dl class="field-list">
        <dt class="field-term">
          format
        </dt>
        <dd class="field-value">
          v{{ terrain.info.version }} — {{ terrain.info.recordSize }} bytes per tile
          point, {{ terrain.info.textureBits }} texture bits
        </dd>

        <dt class="field-term">
          tile points
        </dt>
        <dd class="field-value">
          {{ terrain.info.width }} × {{ terrain.info.height }}
          <span class="text-0.85em opacity-60">
            ({{ terrain.info.tileCount }} points, {{ formatBytes(recordBytes) }})
          </span>
        </dd>

        <dt class="field-term">
          playable
        </dt>
        <dd class="field-value">
          {{ terrain.info.tileWidth }} × {{ terrain.info.tileHeight }} tiles
        </dd>

        <dt class="field-term">
          tileset
        </dt>
        <dd class="field-value">
          {{ terrain.info.tileset }}
          <span v-if="terrain.info.customTileset" class="opacity-60">(custom or mixed)</span>
        </dd>

        <dt class="field-term">
          centre
        </dt>
        <dd class="field-value mono">
          ({{ terrain.info.centerOffsetX.toFixed(1) }},
          {{ terrain.info.centerOffsetY.toFixed(1) }})
        </dd>

        <dt class="field-term">
          origin
        </dt>
        <dd class="field-value mono">
          ({{ terrain.info.worldOriginX.toFixed(1) }},
          {{ terrain.info.worldOriginY.toFixed(1) }})
        </dd>

        <dt class="field-term">
          height
        </dt>
        <dd class="field-value">
          {{ terrain.info.minWorldHeight.toFixed(1) }} –
          {{ terrain.info.maxWorldHeight.toFixed(1) }}
          <span class="opacity-60">world units</span>
        </dd>
      </dl>

      <h2 class="section-head">
        textures
      </h2>
      <dl class="field-list">
        <dt class="field-term">
          ground
        </dt>
        <dd class="field-value">
          {{ terrain.info.groundTextureCount }} listed,
          {{ terrain.info.usedGroundTextures.length }} referenced
          <span
            v-if="terrain.info.groundTextureIds.length"
            class="mono opacity-60"
          >
            [{{ terrain.info.groundTextureIds.join(" ") }}]
          </span>
        </dd>

        <dt class="field-term">
          cliff
        </dt>
        <dd class="field-value">
          {{ terrain.info.cliffTextureCount }}
          <span v-if="terrain.info.cliffTextureIds.length" class="mono opacity-60">
            [{{ terrain.info.cliffTextureIds.join(" ") }}]
          </span>
        </dd>
      </dl>

      <h2 class="section-head">
        layers
      </h2>
      <p v-if="layers.length === 0" class="m-0 text-0.85rem opacity-60">
        no tile points use a layer
      </p>
      <table v-else class="layers">
        <thead>
          <tr>
            <th>idx</th>
            <th>texture</th>
            <th class="text-right">
              points
            </th>
            <th class="text-right">
              share
            </th>
            <th>distribution</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="layer in layers" :key="layer.index">
            <td class="mono">
              {{ layer.index }}
            </td>
            <td class="mono">
              {{ layer.id ?? "—" }}
            </td>
            <td class="text-right mono">
              {{ layer.count.toLocaleString() }}
            </td>
            <td class="text-right mono">
              {{ percent(layer.share) }}
            </td>
            <td>
              <span class="bar" :style="{ width: barWidth(layer.share) }" />
            </td>
          </tr>
        </tbody>
      </table>

      <h2 class="section-head">
        flags
      </h2>
      <dl class="field-list">
        <dt class="field-term">
          ramp
        </dt>
        <dd class="field-value mono">
          {{ terrain.info.rampPoints.toLocaleString() }}
        </dd>
        <dt class="field-term">
          blight
        </dt>
        <dd class="field-value mono">
          {{ terrain.info.blightPoints.toLocaleString() }}
        </dd>
        <dt class="field-term">
          water
        </dt>
        <dd class="field-value mono">
          {{ terrain.info.waterPoints.toLocaleString() }}
        </dd>
        <dt class="field-term">
          boundary
        </dt>
        <dd class="field-value mono">
          {{ terrain.info.boundaryPoints.toLocaleString() }}
        </dd>
      </dl>
    </template>

    <template v-if="terrain.diagnostics.length">
      <h2 class="section-head">
        diagnostics
      </h2>
      <ul class="diag-list">
        <li
          v-for="(d, i) in terrain.diagnostics"
          :key="i"
          class="diag-item"
          :class="`diag-${d.severity}`"
        >
          <span class="diag-severity">{{ d.severity }}</span>
          <span class="diag-code mono">{{ d.code }}</span>
          <span class="diag-message">{{ d.message }}</span>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
/*
 * The histogram table, kept in CSS for reasons utilities cannot cover: a collapsed
 * border model with a per-cell top rule — the usual way to render a dense numeric table
 * without a border on every side — and a bar whose width arrives as an inline style,
 * because the width is data rather than a style choice.
 */
.layers {
  border-collapse: collapse;
  font-size: 0.85rem;
  width: 100%;
  max-width: 42rem;
}

.layers th {
  text-align: left;
  font-weight: 500;
  opacity: 0.6;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.2rem 0.5rem;
}

.layers td {
  padding: 0.15rem 0.5rem;
  border-top: 1px solid rgb(128 128 128 / 0.15);
}

/*
 * Makes a 0.1% difference visible where the number alone hides it. `currentColor` so the
 * bar follows the text colour, including under a high-contrast theme.
 */
.bar {
  display: block;
  height: 0.7rem;
  background: currentColor;
  opacity: 0.35;
  border-radius: 2px;
  min-width: 1px;
}
</style>
