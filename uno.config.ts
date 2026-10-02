import {
  defineConfig,
  presetAttributify,
  presetWind3,
  transformerDirectives,
  transformerVariantGroup
} from 'unocss'

/**
 * UnoCSS for the editor's interface.
 *
 * # The rule this file serves
 *
 * Styling lives in the template as utilities. A `<style scoped>` block is for what
 * utilities cannot express — a `grid-template-columns` with fixed tracks, a
 * pseudo-element — not for grouping what would otherwise be a long class list. The
 * measure is not taste: before this change, style blocks were about half of all the
 * Vue code in the project.
 *
 * Shortcuts exist for patterns that repeat across components. A shortcut used once is
 * worse than the utilities it hides, because reading the template no longer says what
 * it does.
 */
export default defineConfig({
  theme: {
    /**
     * The desktop-app palette, replacing scattered literals.
     *
     * Three text weights and two rules cover everything the panels use. `ink-soft` and
     * `ink-faint` are used through `opacity` instead of as colours so they follow the
     * system light/dark scheme.
     */
    colors: {
      'ink': 'rgb(15 15 15)',
      'line-soft': 'rgb(128 128 128 / 0.18)',
      'line': 'rgb(128 128 128 / 0.4)'
    }
  },

  shortcuts: [
    // ── Page frame ─────────────────────────────────────────────────────────────
    ['page-body', 'flex flex-col max-w-60rem px-6 py-5'],
    ['page-title', 'm-0 text-1.5rem text-left'],
    ['page-path', 'm-0 mb-5 text-0.8rem opacity-60 break-all'],
    ['section-head', 'm-0 mt-6 mb-2 text-0.8rem uppercase tracking-0.06em opacity-60 text-left'],
    ['hint', 'mx-6 my-4 text-0.9rem opacity-70 max-w-40rem'],
    ['mono', 'font-mono'],

    // ── Definition list ────────────────────────────────────────────────────────
    ['field-list', 'grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1 m-0 mb-4 text-0.9rem'],
    ['field-term', 'opacity-60'],
    ['field-value', 'm-0'],

    // ── Toolbar ────────────────────────────────────────────────────────────────
    // The input is monospace because every value it takes is a path.
    ['toolbar-btn', 'px-3.5 py-1.5 text-0.85rem rounded-md b b-line bg-[transparent] text-[inherit] cursor-pointer disabled:opacity-50 disabled:cursor-default'],

    // ── Diagnostic rows ────────────────────────────────────────────────────────
    // Three fixed columns so the codes line up down the list; that alignment is what
    // makes a run of diagnostics readable.
    ['diag-list', 'list-none m-0 p-0 text-0.85rem'],
    ['diag-item', 'grid grid-cols-[3.5rem_12rem_1fr] gap-x-2 px-1 py-0.2 items-baseline'],
    ['diag-severity', 'opacity-80'],
    ['diag-code', 'text-0.8rem opacity-70'],
    ['diag-message', 'break-words'],
    // The severity colour on the row, applied to the whole row rather than the label, so
    // the marker reads as belonging to the diagnostic. Defined here because both panels
    // have a diagnostics list and duplicating the rule in two files is how the two drift
    // apart.
    ['diag-error', 'text-red-600'],
    ['diag-warn', 'text-amber-600'],

    // ── Call-outs ──────────────────────────────────────────────────────────────
    // The left rule is the whole visual idea, so it is a shortcut rather than the same
    // utility list at four call sites.
    ['callout', 'mx-6 mt-5 px-4 py-3.5 border-l-3 max-w-56rem'],
    ['callout-head', 'm-0 mb-2 text-0.8rem uppercase tracking-0.06em opacity-70 text-left'],
    ['callout-note', 'm-0 mb-3 text-0.9rem'],
    ['callout-because', 'm-0 text-0.8rem opacity-70 max-w-46rem'],
    ['callout-attention', 'callout border-amber-600 bg-amber-500/6'],
    // A scope limit is not a warning about the data, so it gets its own accent.
    ['callout-scope', 'callout border-slate-400 bg-slate-500/6'],

    // ── Flex helpers, matching the reference project's abbreviations ───────────
    ['fcc', 'flex justify-center items-center'],
    ['fccc', 'fcc flex-col'],
    ['fyc', 'flex items-center'],
    ['fbc', 'flex justify-between items-center'],
    ['fw', 'flex flex-wrap'],
    ['pr', 'relative'],
    ['pa', 'absolute'],
    ['ps', 'sticky']
  ],

  presets: [
    presetWind3(),
    presetAttributify()
  ],

  transformers: [
    transformerDirectives(),
    transformerVariantGroup()
  ]
})
