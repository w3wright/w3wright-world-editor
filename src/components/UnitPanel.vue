<script setup lang="ts">
/**
 * The unit view: what `war3mapUnits.doo` holds, as the core measured it.
 *
 * Read-only. Nothing here derives a value — the histograms arrive computed from the
 * backend, because `war3 map units` prints the same lists and two implementations of
 * "most common types" would be free to disagree about a map. See `docs/03` §1.1.
 *
 * The record list is a **head**, not the whole file: the backend caps it and says how
 * many it kept, and the header below says so in words rather than leaving a partial
 * list to look complete.
 */
import type { UnitRecord, UnitView } from '../types'
import { computed } from 'vue'
import ObjectName from './ObjectName.vue'

const props = defineProps<{ units: UnitView }>()

/** Problems only, worst first, so an error is not below an info line. */
const orderedDiagnostics = computed(() => {
  const rank: Record<string, number> = { error: 0, warn: 1, info: 2 }
  return [...props.units.diagnostics].sort(
    (a, b) => (rank[a.severity] ?? 3) - (rank[b.severity] ?? 3)
  )
})

/** How many records the head leaves out, or `0` when it is the whole file. */
const hidden = computed(() => {
  const info = props.units.info
  return info ? Math.max(0, info.records - info.shown) : 0
})

/**
 * `.doo`'s "no value" sentinel for hit points and mana.
 *
 * ⚠️ `-1` means **the type's default**, not zero and not unknown. The default itself
 * lives in the type's object data, so resolving it belongs to the object editor rather
 * than this screen — the core reports the stored value and this prints what it means,
 * which is the only thing the interface may say about it.
 */
function orDefault(value: number): string {
  return value < 0 ? 'default' : value.toLocaleString()
}

/** A world position, at the precision the command line prints. */
function position(record: UnitRecord): string {
  return `(${record.x.toFixed(1)}, ${record.y.toFixed(1)}, ${record.z.toFixed(1)})`
}

/**
 * The item figure: inventory entries, drop sets and ability modifications together.
 *
 * A count, not a listing. An `*` marks a record that also names a random item table,
 * which is a different thing from the items it was given by hand.
 */
function itemFigure(record: UnitRecord): number {
  return record.inventory + record.dropSets + record.abilities
}
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        Units
      </h1>
      <p class="page-path">
        {{ units.path }}
      </p>
    </header>

    <template v-if="!units.ok || !units.info">
      <p class="m-0 mb-2.5 border-l-3 border-slate-400 pl-3 text-0.9rem">
        <strong>This map has no readable units.</strong>
        {{ units.reason }}
      </p>
      <p class="m-0 max-w-46rem text-0.85rem opacity-70">
        A map without <code class="text-0.9em font-mono">war3mapUnits.doo</code> is a
        valid archive, so the core models the file as optional and this view reports the
        reason rather than calling it an error. It is the core's own sentence — the same
        one <code class="text-0.9em font-mono">war3 map units</code> prints.
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
          v{{ units.info.version }}/sub{{ units.info.subversion }}
        </dd>

        <dt class="field-term">
          records
        </dt>
        <dd class="field-value">
          {{ units.info.records.toLocaleString() }}
          <span class="text-0.85em opacity-60">
            in {{ units.info.types.distinct }} distinct types
          </span>
        </dd>

        <dt class="field-term">
          levelled
        </dt>
        <dd class="field-value">
          {{ units.info.levelled.toLocaleString() }}
          <span class="text-0.85em opacity-60">above hero level 1</span>
        </dd>

        <dt class="field-term">
          item data
        </dt>
        <dd class="field-value">
          {{ units.info.placedItems.toLocaleString() }}
          <span class="text-0.85em opacity-60">with a table index or an inventory</span>
        </dd>
      </dl>

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
          <tr v-for="type in units.info.types.leaders" :key="type.key">
            <td class="mono">
              <ObjectName :id="type.key" kind="unit" show-id />
            </td>
            <td class="text-right mono">
              {{ type.count.toLocaleString() }}
            </td>
            <td>
              <span
                class="bar"
                :style="{ width: `${(type.count / units.info.types.total * 100).toFixed(2)}%` }"
              />
            </td>
          </tr>
        </tbody>
      </table>
      <p
        v-if="units.info.types.leaders.length < units.info.types.distinct"
        class="m-0 mt-1.5 max-w-46rem text-0.8rem opacity-60"
      >
        The {{ units.info.types.leaders.length }} commonest of
        {{ units.info.types.distinct }} types — the same list
        <code class="text-0.9em font-mono">war3 map units</code> prints. The counts above
        are shares of all {{ units.info.types.total.toLocaleString() }} records, not of
        these {{ units.info.types.leaders.length }} rows.
      </p>

      <h2 class="section-head">
        by player
      </h2>
      <dl class="field-list">
        <template v-for="player in units.info.players" :key="player.key">
          <dt class="field-term">
            {{ player.key }}
          </dt>
          <dd class="field-value mono">
            {{ player.count.toLocaleString() }}
          </dd>
        </template>
      </dl>

      <h2 class="section-head">
        records
        <span class="ml-1.5 opacity-80">{{ units.info.shown.toLocaleString() }}</span>
      </h2>
      <p
        v-if="hidden > 0"
        class="m-0 mb-2 max-w-46rem border-l-3 border-amber-600 pl-3 text-0.85rem"
      >
        The first {{ units.info.shown.toLocaleString() }} of
        {{ units.info.records.toLocaleString() }} records, in file order;
        {{ hidden.toLocaleString() }} more are not listed. They are still counted in
        every figure above — only this list stops early.
      </p>
      <table class="data-table records-table">
        <thead>
          <tr>
            <th>type</th>
            <th>var</th>
            <th>position</th>
            <th>player</th>
            <th>hp</th>
            <th>mana</th>
            <th>level</th>
            <th>gold</th>
            <th>items</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(record, i) in units.records" :key="i">
            <!--
              The name, with the id on hover. `show-id` because this column is the table's first and a
              reader comparing two rows needs to tell units apart even when the game has no name for
              one of them — see `ObjectName`.
            -->
            <td class="mono">
              <ObjectName :id="record.kind" kind="unit" show-id />
            </td>
            <td class="mono">
              {{ record.variation }}
            </td>
            <td class="mono">
              {{ position(record) }}
            </td>
            <td class="mono">
              {{ record.player }}
            </td>
            <td class="mono">
              {{ orDefault(record.hitPoints) }}
            </td>
            <td class="mono">
              {{ orDefault(record.mana) }}
            </td>
            <td class="mono">
              {{ record.heroLevel }}
            </td>
            <td class="mono">
              {{ record.gold.toLocaleString() }}
            </td>
            <td class="mono">
              {{ itemFigure(record) }}
              <span
                v-if="record.itemTable !== null && record.itemTable >= 0"
                class="cursor-help opacity-60"
                title="also names a random item table"
              >*</span>
            </td>
          </tr>
        </tbody>
      </table>

      <template v-if="units.diagnostics.length">
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
