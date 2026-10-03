<script setup lang="ts">
/**
 * One field's value, with the ids inside it turned into names.
 *
 * # Why the value needs a component at all
 *
 * A value is not one string. Measured against a real map's object data, it is four different things,
 * and the screen used to show all of them as the file stores them:
 *
 * | Field | Stored | Shown |
 * | --- | --- | --- |
 * | `uabi` | `A04M,A03Y,Ahrp,A005` | six ability **names**, each with its id on hover |
 * | `ubui` | `h002,h001,n000` | unit names |
 * | `unam` | `TRIGSTR_4227` | the map's own text, resolved by the backend |
 * | `ua1t` | `hero` | 英雄 — a word, resolved by the backend |
 *
 * ⚠️ **Why the ids are resolved here and not in the backend.** The interface already has a batched,
 * coalescing name resolver (`useObjectNames`): 121 rows asking about the same six abilities make **one**
 * round trip. Resolving a list in the backend would either be one call per row or a second batching
 * mechanism, and the answer is already cached by the first. The backend hands over the raw string and
 * the type word that says its parts are ids; the splitting happens once, here.
 *
 * # Why the split is not a guess
 *
 * `valueType` comes from the game's metadata (`abilityList`, `unitList`, `upgradeList`) and is what
 * says the parts are ids — see `war3_game::list_element_kind`, which is the same function the backend
 * used to decide `valueLabel` is absent. The prefix on the field id (`u` = unit) is *not* used: it is a
 * convention that happens to hold, not a fact the game states.
 */
import type { ObjectModification } from '../types'
import { computed } from 'vue'
import { TECH_KINDS } from '../composables/useObjectNames'
import ObjectName from './ObjectName.vue'

const props = defineProps<{
  /** The modification, as the backend flattened it. */
  modification: ObjectModification
}>()

/**
 * The kind — or kinds — the value's parts belong to, or `null` when the value is not a list of ids.
 *
 * ⚠️ A `techList` (`ureq` = `R00M`) maps to **four** kinds rather than one, because its ids carry no
 * kind: `R00M` is an upgrade and `hfoo` would be a unit, and the value does not say which. `ObjectName`
 * tries them in order and takes the first that names the object — one batched request either way.
 * Mapping it to a single kind would query the wrong table and show the id, which reads as missing data.
 *
 * ⚠️ Kept in step with `war3_game::fields::list_element_kind` and `value_shape`; the compiler cannot
 * check that, hence this note. A type word added there needs adding here.
 */
const elementKind = computed<string | readonly string[] | null>(() => {
  switch (props.modification.valueType) {
    case 'abilityList':
      return 'ability'
    case 'unitList':
      return 'unit'
    case 'upgradeList':
      return 'upgrade'
    case 'techList':
      return TECH_KINDS
    default:
      return null
  }
})

/** The ids in the value, in the order the file lists them. */
const ids = computed<string[]>(() => {
  if (elementKind.value === null)
    return []
  return props.modification.value
    .split(',')
    .map(part => part.trim())
    // ⚠️ An empty value is common (`uabi` with nothing set) and must not produce one empty id: a
    // lookup for `''` would be a round trip for an answer nobody can use.
    .filter(part => part.length > 0)
})
</script>

<template>
  <!--
    A comma-separated list of ids, each shown as its name with the id on hover.
  -->
  <span v-if="elementKind !== null && ids.length > 0" class="inline-flex flex-wrap items-baseline gap-x-1">
    <template v-for="(id, i) in ids" :key="`${id}-${i}`">
      <span v-if="i > 0" class="opacity-40">,</span>
      <ObjectName :id="id" :kind="elementKind" />
    </template>
  </span>
  <!--
    ⚠️ `valueLabel` is what the backend computed for every value that is **not** a list: a `TRIGSTR_`
    reference resolved through the map's table, a word like `hero` turned into 英雄, or a number that
    was already its own text. `??` and not `||` so an empty label stays empty rather than falling
    through to the raw value.
  -->
  <span v-else-if="modification.valueLabel !== null">{{ modification.valueLabel }}</span>
  <!--
    The raw value as a last resort — a list type this build does not know, an empty list, or a map
    whose string table could not be read. Showing the stored form is the same degradation everywhere
    else in this workspace: a reader sees what is there rather than a blank.
  -->
  <span v-else>{{ modification.value }}</span>
</template>
