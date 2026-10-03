<script setup lang="ts">
/**
 * The object view: the seven `war3map.w3*` files a map holds.
 *
 * Read-only. Both the objects and their fields are shown by **name**, resolved from the game
 * installation: `hC06` is 守卫 (基本建造者), `unam` is 名字, and the ids inside a value — `uabi`'s six
 * abilities — are resolved too. ⚠️ Every one of those comes from the core's tables rather than from a
 * mapping here, because the text lives in the installation's own language files.
 */
import type { ObjectCategory, ObjectEntry, ObjectView } from '../types'
import { computed } from 'vue'
import FieldValue from './FieldValue.vue'
import ObjectName from './ObjectName.vue'

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
        <strong>Field names and ids inside values come from the installed game.</strong>
        Both need Blizzard's <code class="text-0.9em font-mono">*MetaData.slk</code> tables, which live
        in the game's archives rather than the map — the same tables
        <code class="text-0.9em font-mono">war3 map objects --game-dir</code> uses. Without a game
        directory configured, a field shows its id and a value its stored form, which is what the
        command line prints without that flag.
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
                  <span class="font-600">
                    <!--
                      The object's own name, then what it inherits from. ⚠️ The category's `kind` is
                      the core's spelling of the *table* (`unit`, `item`, …), so it is passed through
                      rather than mapped here: a second table of kind names in the interface is exactly
                      the divergence `docs/03` §1.1 forbids.
                    -->
                    <ObjectName :id="object.id" :kind="category.kind" />
                    <template v-if="object.baseId">
                      <span class="mx-1 opacity-50">←</span>
                      <ObjectName :id="object.baseId" :kind="category.kind" />
                    </template>
                  </span>
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
                <!--
                  The field's name, with its id on hover. ⚠️ The id is still shown when the metadata
                  does not describe the field — an object written by a tool that invented one — because
                  `m.field` is what is really in the file and a blank cell would hide that.
                -->
                <td class="mono" :title="m.field">
                  {{ m.fieldLabel ?? m.field }}
                </td>
                <td class="break-all">
                  <FieldValue :modification="m" />
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
