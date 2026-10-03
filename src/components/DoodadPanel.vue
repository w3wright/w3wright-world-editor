<script setup lang="ts">
/**
 * The doodad view: what `war3map.doo` holds, as the core measured it.
 *
 * Read-only, and shaped like `UnitPanel` because the two files share a record header
 * and a set of questions. `war3 map doodads` prints the same aggregates.
 *
 * ⚠️ This is the view that makes the detail cap necessary: `(4)LostTemple.w3m` holds
 * **5,317** doodads against 121 units on the same map. Every one of them is counted in
 * the figures; the list below stops early and says so.
 */
import type { DoodadRecord, DoodadView } from '../types'
import { computed } from 'vue'
import ObjectName from './ObjectName.vue'

const props = defineProps<{ doodads: DoodadView }>()

/** Problems only, worst first. */
const orderedDiagnostics = computed(() => {
  const rank: Record<string, number> = { error: 0, warn: 1, info: 2 }
  return [...props.doodads.diagnostics].sort(
    (a, b) => (rank[a.severity] ?? 3) - (rank[b.severity] ?? 3)
  )
})

/** How many records the head leaves out, or `0` when it is the whole file. */
const hidden = computed(() => {
  const info = props.doodads.info
  return info ? Math.max(0, info.records - info.shown) : 0
})

/**
 * A world position, at the precision the command line prints.
 */
function position(record: DoodadRecord): string {
  return `(${record.x.toFixed(1)}, ${record.y.toFixed(1)}, ${record.z.toFixed(1)})`
}

/*
 * On the flags column: `record.flagsName` is a string the backend already composed
 * with `DoodadFile::flags_name`. The byte is a small integer rather than independent
 * bits, and which values mean what is a format conclusion — so there is deliberately no
 * `switch` on `record.flags` here. The raw value is printed beside the name, for anyone
 * comparing against a hex dump.
 */
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        Doodads
      </h1>
      <p class="page-path">
        {{ doodads.path }}
      </p>
    </header>

    <template v-if="!doodads.ok || !doodads.info">
      <p class="m-0 mb-2.5 border-l-3 border-slate-400 pl-3 text-0.9rem">
        <strong>This map has no readable doodads.</strong>
        {{ doodads.reason }}
      </p>
      <p class="m-0 max-w-46rem text-0.85rem opacity-70">
        A map without <code class="text-0.9em font-mono">war3map.doo</code> is a valid
        archive, so this is a state rather than a failure. The reason above is the core's
        own sentence — the same one
        <code class="text-0.9em font-mono">war3 map doodads</code> prints.
      </p>
    </template>

    <template v-else>
      <h2 class="section-head">
        file
      </h2>
      <dl class="field-list">
        <dt class="field-term">
          format
        </dt>
        <dd class="field-value">
          v{{ doodads.info.version }}/sub{{ doodads.info.subversion }}
        </dd>

        <dt class="field-term">
          records
        </dt>
        <dd class="field-value">
          {{ doodads.info.records.toLocaleString() }}
          <span class="text-0.85em opacity-60">
            in {{ doodads.info.types.distinct }} distinct types
          </span>
        </dd>

        <dt class="field-term">
          special
        </dt>
        <dd class="field-value">
          {{ doodads.info.special.toLocaleString() }}
          <span class="text-0.85em opacity-60">
            version word v{{ doodads.info.specialVersion }}
          </span>
        </dd>
      </dl>
      <p
        v-if="doodads.info.special > 0"
        class="m-0 mt-1.5 max-w-46rem text-0.8rem opacity-60"
      >
        Special doodads are the ones cliffs are made of; the World Editor does not offer
        them for editing. Their types:
        <span class="mono">
          {{ doodads.info.specialKinds.map(k => `${k.key} ×${k.count}`).join(", ") }}
        </span>
      </p>

      <h2 class="section-head">
        most common types
      </h2>
      <table class="data-table max-w-26rem">
        <thead>
          <tr>
            <th>type</th>
            <th class="text-right">
              count
            </th>
            <th>share</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="type in doodads.info.types.leaders" :key="type.key">
            <td class="mono">
              <ObjectName :id="type.key" kind="doodad" show-id />
            </td>
            <td class="text-right mono">
              {{ type.count.toLocaleString() }}
            </td>
            <td>
              <span
                class="bar"
                :style="{ width: `${(type.count / doodads.info.types.total * 100).toFixed(2)}%` }"
              />
            </td>
          </tr>
        </tbody>
      </table>
      <p
        v-if="doodads.info.types.leaders.length < doodads.info.types.distinct"
        class="m-0 mt-1.5 max-w-46rem text-0.8rem opacity-60"
      >
        The {{ doodads.info.types.leaders.length }} commonest of
        {{ doodads.info.types.distinct }} types — the same list
        <code class="text-0.9em font-mono">war3 map doodads</code> prints. The shares
        above are of all {{ doodads.info.types.total.toLocaleString() }} records.
      </p>

      <h2 class="section-head">
        records
        <span class="ml-1.5 opacity-80">{{ doodads.info.shown.toLocaleString() }}</span>
      </h2>
      <p
        v-if="hidden > 0"
        class="m-0 mb-2 max-w-46rem border-l-3 border-amber-600 pl-3 text-0.85rem"
      >
        The first {{ doodads.info.shown.toLocaleString() }} of
        {{ doodads.info.records.toLocaleString() }} records, in file order;
        {{ hidden.toLocaleString() }} more are not listed. Every one of them is still
        counted above, and the type shares are unaffected — sending the whole list would
        be
        {{ doodads.info.records.toLocaleString() }} rows this screen cannot use.
      </p>
      <table class="data-table records-table">
        <thead>
          <tr>
            <th>type</th>
            <th>var</th>
            <th>position</th>
            <th>flags</th>
            <th>raw</th>
            <th>life</th>
            <th>editor</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(record, i) in doodads.records" :key="i">
            <td class="mono">
              <ObjectName :id="record.kind" kind="doodad" show-id />
            </td>
            <td class="mono">
              {{ record.variation }}
            </td>
            <td class="mono">
              {{ position(record) }}
            </td>
            <td class="text-left">
              {{ record.flagsName }}
            </td>
            <td class="mono">
              {{ record.flags }}
            </td>
            <td class="mono">
              {{ record.life }}%
            </td>
            <td class="mono">
              {{ record.editorId }}
            </td>
          </tr>
        </tbody>
      </table>

      <template v-if="doodads.diagnostics.length">
        <h2 class="section-head">
          diagnostics
        </h2>
        <ul class="diag-list">
          <li
            v-for="(d, i) in orderedDiagnostics"
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
    </template>
  </section>
</template>
