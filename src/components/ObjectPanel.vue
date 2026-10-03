<script setup lang="ts">
/**
 * The object view: the seven `war3map.w3*` files a map holds.
 *
 * Read-only, and the one view here whose data has a **name for every field that this
 * app cannot show**. `war3 map objects` turns `uhpm` into "Hit Points" only when it is
 * given `--game-dir`, because the metadata tables live in the game's archives rather
 * than the map. This app has no game-directory setting, so it prints ids and values —
 * exactly what the command line prints without that flag.
 *
 * ⚠️ That is why there is no name lookup, no `ObjectKind` list and no field table in
 * this file: any of those would be a second, worse copy of `war3-meta`, and a wrong
 * name is harder to notice than an id.
 */
import type { ObjectCategory, ObjectEntry, ObjectView } from '../types'
import { computed } from 'vue'

const props = defineProps<{ objects: ObjectView }>()

/** Problems only, worst first. */
const orderedDiagnostics = computed(() => {
  const rank: Record<string, number> = { error: 0, warn: 1, info: 2 }
  return [...props.objects.diagnostics].sort(
    (a, b) => (rank[a.severity] ?? 3) - (rank[b.severity] ?? 3)
  )
})

/** Every object across every category, for the header tally. */
const totals = computed(() => {
  let original = 0
  let custom = 0
  for (const category of props.objects.categories) {
    original += category.original
    custom += category.custom
  }
  return { original, custom, all: original + custom }
})

/** `id <- base`, or just the id for a modified original. */
function inheritance(object: ObjectEntry): string {
  return object.baseId ? `${object.id} <- ${object.baseId}` : object.id
}

/**
 * How many of an object's modifications the panel is not showing.
 *
 * `shown` is capped by the backend, and `modifications` is the real count, so the
 * difference is stated rather than left as a truncated list.
 */
function hiddenModifications(object: ObjectEntry): number {
  return Math.max(0, object.modifications - object.shown.length)
}

/** The list of objects one category prints: customs first, as the command line does. */
function listed(category: ObjectCategory): ObjectEntry[] {
  return [...category.customObjects, ...category.modifiedObjects]
}
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        Objects
      </h1>
      <p class="page-path">
        {{ objects.path }}
      </p>
    </header>

    <template v-if="objects.categories.length === 0">
      <p class="m-0 mb-2.5 border-l-3 border-slate-400 pl-3 text-0.9rem">
        <strong>This map has no object files.</strong>
      </p>
      <p class="m-0 max-w-46rem text-0.85rem opacity-70">
        None of the seven categories — units, items, destructables, doodads, abilities,
        buffs, upgrades — is present. That is what
        <code class="text-0.9em font-mono">war3 map objects</code> reports for this map
        too: an untouched Blizzard map carries no object data.
      </p>
    </template>

    <template v-else>
      <h2 class="section-head">
        summary
      </h2>
      <dl class="field-list">
        <dt class="field-term">
          categories
        </dt>
        <dd class="field-value">
          {{ objects.categories.length }}
          <span class="text-0.85em mono opacity-60">
            {{ objects.categories.map(c => c.mapFile).join(" ") }}
          </span>
        </dd>

        <dt class="field-term">
          objects
        </dt>
        <dd class="field-value">
          {{ totals.custom.toLocaleString() }} custom,
          {{ totals.original.toLocaleString() }} modified originals
        </dd>
      </dl>

      <p class="m-0 mt-3 max-w-46rem border-l-3 border-slate-400 pl-3 text-0.85rem">
        <strong>Field ids are shown as stored.</strong>
        Naming a field needs Blizzard's <code class="text-0.9em font-mono">*MetaData.slk</code>
        tables, which live in the game's archives rather than the map — that is what
        <code class="text-0.9em font-mono">war3 map objects --game-dir</code> is for.
        This app has no game directory, so what follows is the command line's own
        output without it, not a truncated version of the named one.
      </p>

      <section v-for="category in objects.categories" :key="category.mapFile">
        <h2 class="section-head">
          {{ category.mapFile }}
          <span class="ml-1.5 normal-case opacity-80">{{ category.kind }}</span>
        </h2>
        <dl class="field-list">
          <dt class="field-term">
            version
          </dt>
          <dd class="field-value mono">
            {{ category.version }}
          </dd>
          <dt class="field-term">
            objects
          </dt>
          <dd class="field-value">
            {{ category.original.toLocaleString() }} original,
            {{ category.custom.toLocaleString() }} custom
          </dd>
        </dl>

        <table class="data-table objects-table">
          <thead>
            <tr>
              <th>object</th>
              <th>field</th>
              <th>value</th>
              <th>level</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="object in listed(category)" :key="`${category.mapFile}:${object.id}`">
              <tr class="object-head">
                <td colspan="4">
                  <span class="font-600 mono">{{ inheritance(object) }}</span>
                  <span v-if="object.hero" class="ml-2 text-0.8em opacity-70">hero</span>
                  <span class="ml-2 text-0.8em opacity-60">
                    {{ object.modifications.toLocaleString() }}
                    field{{ object.modifications === 1 ? "" : "s" }}
                    <template v-if="hiddenModifications(object) > 0">
                      — showing {{ object.shown.length }}
                    </template>
                  </span>
                </td>
              </tr>
              <tr v-for="(m, i) in object.shown" :key="i">
                <td />
                <td class="mono">
                  {{ m.field }}
                </td>
                <td class="break-all">
                  {{ m.value }}
                </td>
                <td class="mono opacity-70">
                  <template v-if="m.level !== null">
                    #{{ m.level }}<span v-if="m.dataIndicator">/{{ m.dataIndicator }}</span>
                  </template>
                  <template v-else>
                    —
                  </template>
                </td>
              </tr>
              <tr v-if="object.shown.length === 0">
                <td />
                <td colspan="3" class="opacity-60">
                  no modifications
                </td>
              </tr>
            </template>
          </tbody>
        </table>
        <p
          v-if="category.shown < category.original + category.custom"
          class="m-0 mt-1.5 max-w-46rem text-0.8rem opacity-60"
        >
          Lists the first {{ category.customObjects.length }} custom and
          {{ category.modifiedObjects.length }} modified objects; the category holds
          {{ category.custom.toLocaleString() }} and
          {{ category.original.toLocaleString() }}.
        </p>
      </section>

      <template v-if="objects.diagnostics.length">
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

<style scoped>
/*
 * The object table's own column definition, kept in CSS for the same reason
 * `base.css` gives: the value column is the only one that needs a width, and the other
 * three size themselves. A `grid-template-columns` equivalent has no utility, and this
 * one is specific to this panel rather than shared with the record lists, so it stays
 * here instead of in the base layer.
 */
.objects-table {
  max-width: 62rem;
  table-layout: fixed;
}

.objects-table th:nth-child(1),
.objects-table td:nth-child(1) {
  width: 12rem;
}

.objects-table th:nth-child(2),
.objects-table td:nth-child(2) {
  width: 5rem;
}

.objects-table th:nth-child(4),
.objects-table td:nth-child(4) {
  width: 4rem;
  text-align: right;
}

/*
 * The per-object heading row. It spans the whole table, so it needs a rule *above* it
 * and no rule between it and its own fields — which is the opposite of what
 * `.data-table` gives every cell.
 */
.objects-table .object-head td {
  border-top: none;
  padding-top: 0.7rem;
  background: rgb(128 128 128 / 0.07);
}
</style>
