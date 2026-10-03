<script setup lang="ts">
/**
 * An object ID, shown as its name, with the ID on hover.
 *
 * ```vue
 * <ObjectName kind="unit" id="hfoo" />        <!-- 步兵, title="hfoo" -->
 * <ObjectName kind="item" id="pghe" />        <!-- 大生命药水 -->
 * ```
 *
 * # Why this is one component rather than a rule per panel
 *
 * Every list in this app holds IDs: placed units, doodads, object data, and later abilities, items,
 * upgrades. Writing the ID in each of them and looking the name up separately is the same twenty
 * lines copied per table — and more to the point, each copy would decide for itself what to do when
 * the name is unknown, the game is missing, or the answer has not arrived yet.
 *
 * # Three states, and none of them is a blank cell
 *
 * | When | Shows |
 * | --- | --- |
 * | the name is known | the name |
 * | the game has no name for it | the ID, which is what the World Editor shows too |
 * | the answer has not arrived | the ID, immediately |
 *
 * ⚠️ **The ID is the placeholder**, not a spinner or an empty cell. A table of 121 rows that renders
 * 121 spinners and then swaps them is worse than one that shows IDs and improves them: the layout
 * does not move, a reader can start reading at once, and a failed lookup leaves the ID rather than a
 * hole. The swap is invisible because the ID is what would have been there before this component
 * existed.
 *
 * # Why `title` and not a hand-rolled tooltip
 *
 * The ID is a fact a reader occasionally needs and rarely wants: a four-character code beside every
 * human-readable name would be noise. `title` is the browser's own tooltip — no layout of ours, no
 * focus trap, positioned by the platform, and announced by screen readers. A custom tooltip would be
 * a second implementation of something the environment already does well.
 *
 * ⚠️ Markup is **not** stripped here. A name from the game can carry `|cffffff00…|r`, and decoding it
 * is `war3_map::plain`'s job in the core — one function, used by the command line and the panels
 * alike. A `strip` here would be a second answer to a question already answered once, and it is the
 * exact mistake `docs/03` §1.1 describes.
 */
import { computed, ref, watch } from 'vue'
import { namesGeneration, peekName, resolveNames } from '../composables/useObjectNames'

const props = withDefaults(defineProps<{
  /** Which table to consult: `unit`, `item`, `ability`, `destructable`, `doodad`, `buff`, `upgrade`. */
  kind: string | readonly string[]
  /** The object's four-character id. */
  id: string
  /** Whether to show the ID as a secondary line instead of only on hover. */
  showId?: boolean
}>(), {
  showId: false
})

/** The kinds to try, in order. One entry for almost everything. */
const kinds = computed<readonly string[]>(() =>
  typeof props.kind === 'string' ? [props.kind] : props.kind)

/** What to draw. Starts as the ID, so the first paint is already correct. */
const shown = ref(props.id)

/**
 * Re-reads the cache for this object.
 *
 * Called on mount, when the inputs change, and when the cache is forgotten. Reading from a plain
 * `Map` needs a nudge like this because a `Map` is not reactive — see `namesGeneration`.
 *
 * ⚠️ **Several kinds are tried in order and the first that actually named the object wins.** A
 * `techList` value (`ureq` = `R00M`) holds ids whose kind the file does not state — `R00M` is an
 * upgrade, `hfoo` a unit — so the only way to name one is to ask. The test is `source !== 'id'` and not
 * "the name differs from the id": an object may legitimately be called `hfoo`, and a kind that has no
 * entry answers with the id itself.
 */
function refresh(): void {
  for (const kind of kinds.value) {
    const known = peekName(kind, props.id)
    if (known && known.source !== 'id') {
      shown.value = known.name
      return
    }
  }
  // Nothing has named it yet, or nothing ever will. The id is what is stored, so it is what shows.
  shown.value = props.id
}

watch(
  () => [kinds.value.join(','), props.id, namesGeneration.value],
  () => {
    refresh()
    // ⚠️ Asked for even when the cache already had it, because `refresh` above is what reads the
    // cache and this is what fills it. When the answer is already known `resolveNames` returns
    // without a round trip, so the second call costs nothing.
    //
    // ⚠️ Every kind is asked about **in one call**, so the coalescing in `resolveNames` turns this into
    // one round trip however many kinds there are. Asking kind by kind would be several round trips for
    // one cell.
    void resolveNames(kinds.value.map(kind => ({ kind, id: props.id })), refresh)
  },
  { immediate: true }
)
</script>

<template>
  <!--
    ⚠️ A single `<span>` with a `title`, not a wrapper with a custom hover layer. `title` is the
    platform's own tooltip: it needs no layout, no z-index and no focus management, and it is
    announced. Inline because these sit inside table cells and list rows.
  -->
  <span class="inline-flex items-baseline gap-1.5" :title="id">
    <span>{{ shown }}</span>
    <!--
      The ID in text, for a context where a tooltip cannot reach — a printed or copied line, or a
      keyboard user who never hovers. Off by default because it doubles the width of every cell.
    -->
    <code v-if="showId && shown !== id" class="text-0.78em opacity-50">{{ id }}</code>
  </span>
</template>
