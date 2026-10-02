<script setup lang="ts">
/**
 * The map overview: identity, size, members and the source-project verdict.
 *
 * Everything shown here is a value the core computed. Nothing is derived in this file
 * — resolving a `TRIGSTR_`, sizing a member list, deciding a disposition all happen in
 * `crates/`. See `docs/03` §1.1.
 */
import type { MapSummary } from '../types'
import { computed } from 'vue'
import { badgeVars } from '../badges'
import { formatBytes, formatFlags } from '../format'

const props = defineProps<{ summary: MapSummary }>()

/** Problems only, for the count in the header. */
const problems = computed(() =>
  props.summary.diagnostics.filter(d => d.severity !== 'info')
)

/** Diagnostics worst-first, so an error is not below a warning. */
const orderedDiagnostics = computed(() => {
  const rank: Record<string, number> = { error: 0, warn: 1, info: 2 }
  return [...props.summary.diagnostics].sort(
    (a, b) => (rank[a.severity] ?? 3) - (rank[b.severity] ?? 3)
  )
})

/** Each disposition, with how many members land there. */
const dispositionCounts = computed(() => {
  const counts = new Map<string, number>()
  for (const member of props.summary.members)
    counts.set(member.disposition, (counts.get(member.disposition) ?? 0) + 1)
  return counts
})

/*
 * Badge colours come from `nano-pastel` through `badgeVars`, which yields the hue and
 * saturation as custom properties.
 *
 * ⚠️ **An earlier version used UnoCSS shortcuts with a name built at runtime**, as in
 * `` `tag-${disposition}` ``. That silently produced no styles: UnoCSS extracts class
 * names **statically from the source text**, so a name assembled at runtime appears
 * nowhere and no rule is generated for it. Inline custom properties have no such
 * constraint — the value is data, not a class name.
 */
function badgeStyle(disposition: string): Record<string, string> {
  return badgeVars(disposition)
}
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        {{ summary.name || "(unnamed map)" }}
      </h1>
      <p class="page-path">
        {{ summary.path }}
      </p>
    </header>

    <dl class="field-list">
      <dt class="field-term">
        author
      </dt>
      <dd class="field-value">
        {{ summary.author || "—" }}
      </dd>

      <dt class="field-term">
        players
      </dt>
      <dd class="field-value">
        {{ summary.recommendedPlayers || "—" }}
      </dd>

      <dt class="field-term">
        size
      </dt>
      <dd class="field-value">
        {{ summary.playableWidth }} × {{ summary.playableHeight }} tiles
      </dd>

      <dt class="field-term">
        tileset
      </dt>
      <dd class="field-value">
        {{ summary.tileset || "—" }}
      </dd>

      <dt class="field-term">
        format
      </dt>
      <dd class="field-value">
        version {{ summary.formatVersion }}
      </dd>

      <dt class="field-term">
        members
      </dt>
      <dd class="field-value">
        {{ summary.memberCount }} files, {{ formatBytes(summary.totalMemberBytes) }}
      </dd>

      <dt class="field-term">
        strings
      </dt>
      <dd class="field-value">
        {{ summary.stringCount }} entries
      </dd>

      <dt class="field-term">
        flags
      </dt>
      <dd class="field-value mono">
        {{ formatFlags(summary.flags) }}
      </dd>
    </dl>

    <template v-if="summary.description">
      <h2 class="section-head">
        description
      </h2>
      <p class="m-0 whitespace-pre-wrap text-0.9rem">
        {{ summary.description }}
      </p>
    </template>

    <h2 class="section-head">
      members
      <span class="ml-1.5 opacity-80">{{ summary.members.length }}</span>
      <span class="ml-2.5 inline-flex gap-1">
        <span
          v-for="[kind, n] in dispositionCounts"
          :key="kind"
          class="badge"
          :style="badgeStyle(kind)"
        >
          {{ kind }} {{ n }}
        </span>
      </span>
    </h2>
    <ul class="m-0 list-none p-0 text-0.85rem">
      <li
        v-for="member in summary.members"
        :key="member.name"
        class="flex items-baseline gap-2 border-b border-b-line-soft px-1 py-0.2"
      >
        <span class="badge badge-fixed" :style="badgeStyle(member.disposition)">
          {{ member.disposition }}
        </span>
        <span class="overflow-hidden text-ellipsis whitespace-nowrap mono">
          {{ member.name }}
        </span>
        <span class="min-w-20 flex-none text-right opacity-60">
          {{ formatBytes(member.size) }}
        </span>
        <span
          v-if="member.dispositionReason"
          class="min-w-0 flex-1 cursor-help overflow-hidden text-ellipsis whitespace-nowrap text-0.75rem opacity-60"
          :title="member.dispositionReason"
        >
          {{ member.dispositionReason }}
        </span>
      </li>
    </ul>

    <h2 class="section-head">
      source project
    </h2>
    <p v-if="summary.extractable.ok" class="m-0 max-w-46rem text-0.85rem">
      This map can be extracted: {{ summary.extractable.namedBlocks }} names recovered
      for {{ summary.extractable.usedBlocks }} blocks in use.
    </p>
    <p v-else class="m-0 max-w-46rem border-l-3 border-amber-600 pl-3 text-0.85rem">
      <strong>The core would refuse to extract this map.</strong>
      {{ summary.extractable.reason }}. It refuses rather than silently dropping members;
      exporting anyway (<code class="text-0.8rem">--drop-unnamed</code>) is a
      command-line decision, not one this panel makes for you.
    </p>

    <h2 class="section-head">
      diagnostics
      <span
        class="ml-1.5 opacity-80"
        :class="problems.length > 0 ? 'text-red-600' : ''"
      >
        {{ summary.diagnostics.length }}
      </span>
    </h2>
    <p v-if="summary.diagnostics.length === 0" class="m-0 text-0.85rem opacity-60">
      none — the whole map parsed
    </p>
    <ul v-else class="diag-list">
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
  </section>
</template>
