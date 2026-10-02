import { DEFAULT_PALETTE, PastelColor, pastelsFor } from 'nano-pastel'

/**
 * A colour per member disposition, from `nano-pastel`.
 *
 * # Why the library rather than four hand-picked colours
 *
 * The dispositions are an ordered list, and the thing that matters is that **neighbours
 * are far apart on the wheel** — `text` next to `binary` next to `raw`. Choosing four
 * colours by eye gets that wrong quietly: they end up distinguishable in isolation and
 * muddy next to each other. `nano-pastel`'s `order` strategy alternates warm and cool so
 * adjacent entries are ≥100° apart, which is exactly this problem.
 *
 * ⚠️ **`order`, not `hash`.** Hash strategy is stable per key but guarantees nothing
 * about neighbours: measured, it put `text` (150°) and `binary` (100°) 50° apart, which
 * for a colour-coded badge is not a difference anyone can read. Order is what buys the
 * separation, and the list order here is a constant in this file, so nothing shifts
 * under it.
 *
 * # Lightness is chosen for contrast, not for looks
 *
 * The default pastel (`lightness: 92`) is a **background** colour. Used as text it is
 * nearly invisible, so the badge takes a darkened variant for the text and the pastel for
 * the fill — the separation is by lightness, not hue.
 *
 * The text lightness is 30% on a light surface and 72% on a dark one. Both were measured
 * rather than guessed: the worst hue is 50° (yellow), which needs 32% to reach the 4.5:1
 * of WCAG AA on white, and the same reasoning runs the other way on `#2f2f2f`. A single
 * value cannot serve both — at 30% the dark surface gives contrast ratios of 1.0 to 2.6,
 * which is unreadable.
 */
export const DISPOSITION_KINDS = ['text', 'binary', 'raw', 'absent'] as const

export type DispositionKind = (typeof DISPOSITION_KINDS)[number]

const pastels = pastelsFor(DISPOSITION_KINDS, { strategy: 'order' })

/** The fallback for a disposition this build does not know. */
const NEUTRAL = new PastelColor({
  hue: DEFAULT_PALETTE.cool[0],
  saturation: 20,
  lightness: 45,
  alpha: 1
})

/**
 * CSS custom properties for one disposition, ready for `:style`.
 *
 * Hue and saturation only. The lightness and alpha live in the stylesheet, because they
 * are the half that changes with the colour scheme and a media query cannot be expressed
 * in an inline style. The split is therefore "what is constant in JS" versus "what the
 * theme decides".
 */
export function badgeVars(kind: string): Record<string, string> {
  const colour = isKnownDisposition(kind) ? pastels[kind] : NEUTRAL

  return {
    '--badge-h': String(colour.hue),
    // Raised from the pastel default of 45%: at text lightness the default reads as grey
    // rather than as a colour.
    '--badge-s': '63%'
  }
}

/** Whether a disposition is one this build knows about. */
export function isKnownDisposition(kind: string): kind is DispositionKind {
  return (DISPOSITION_KINDS as readonly string[]).includes(kind)
}
