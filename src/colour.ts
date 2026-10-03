import type { ComputedRef } from 'vue'
import type { RichText } from './types'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

/**
 * The author's colour for a run of map text, in a form the stylesheet can draw.
 *
 * # The problem
 *
 * Warcraft III map text carries the author's colour, and much of it is **white** —
 * `|cffffff00` is the commonest code in the corpus. Drawing the author's RGB unchanged puts
 * white text on this window's light background: the colours would be "rendered" in the sense
 * that the browser was told about them, and lost in the sense the reader cares about.
 *
 * # What is kept, and what is solved for
 *
 * Hue and saturation are the author's. Lightness is **solved for legibility**: starting from the
 * author's own lightness, it moves away from the background until the text reaches WCAG AA
 * (4.5:1) against the surface it is drawn on.
 *
 * Two simpler rules were tried and both are wrong:
 *
 * - *Draw the colour as stored.* White on white, and the Sentinel's near-black red on the dark
 *   scheme, are invisible.
 * - *Clamp lightness to a band.* Measured, this does not work: pure yellow is 2.49:1 at HSL
 *   lightness 32 on this surface, and 1.76:1 at 38, while the same band gives a blue 8:1. A
 *   band is one number for every hue, and hues do not have the same contrast at the same
 *   lightness — yellow is much brighter than blue at lightness 32. So the lightness has to be
 *   derived* from the colour's own contrast, which is what this does.
 *
 * The result is that a colour the author picked is recognisable rather than exact, and never
 * unreadable. That trade is made deliberately: a panel of unreadable text is worse than text
 * whose yellow is a little darker than the game's.
 */

/** WCAG AA for body text. */
const TARGET_CONTRAST = 4.5

/**
 * The surface the text is drawn on.
 *
 * ⚠️ Kept in step with `src/styles/base.css` by hand. There is no way to read it at runtime that
 * would survive a stylesheet change either, and a wrong value here would silently mis-solve
 * every colour — so the two literals are named together here as the pair they are.
 */
const SURFACE = {
  light: [0xF6, 0xF6, 0xF6] as const,
  dark: [0x2F, 0x2F, 0x2F] as const
}

/**
 * Whether the window is in a dark scheme.
 *
 * In a module ref rather than left to a CSS media query because the lightness is solved in
 * TypeScript: a `@media` rule can set a value, but it cannot solve an equation per colour.
 * `src/styles/base.css`'s own query is the visual source of truth; this one follows it for the
 * arithmetic.
 */
export const isDark = ref(false)

/** Starts following the system scheme. Called by the one component that draws a colour. */
export function watchColorScheme(): void {
  if (typeof window === 'undefined' || !window.matchMedia)
    return
  const query = window.matchMedia('(prefers-color-scheme: dark)')
  const apply = (matches: boolean): void => {
    isDark.value = matches
  }
  apply(query.matches)
  const onChange = (event: MediaQueryListEvent): void => apply(event.matches)
  // Registered on mount rather than at module scope: `onBeforeUnmount` outside a component is a
  // warning, and a listener nobody removes outlives the window it was watching.
  onMounted(() => query.addEventListener('change', onChange))
  onBeforeUnmount(() => query.removeEventListener('change', onChange))
}

/** A stored colour as `hsl()` arguments plus the numbers behind them. */
export interface RenderedColour {
  /** Ready for a `color:` declaration. */
  css: string
  /** Hue, 0–360 — the author's. */
  hue: number
  /** Saturation, 0–100 — the author's. */
  saturation: number
  /** The lightness that was solved for. */
  lightness: number
  /** The contrast ratio the result has against the surface, for a caller that wants to check. */
  contrast: number
}

/**
 * Converts an author's RGB to a colour that can be read on this background.
 *
 * Returns `null` in one case: the colour is essentially white. WC3 writes `|cffffff00` on most
 * map text, and drawn honestly that is white text on this light surface — invisible. Lightening
 * it further cannot help and darkening it would turn "no colour in particular" into a dark grey
 * the author never chose, so white is drawn in the surface's own text colour and the caller is
 * told there is no colour here.
 *
 * The threshold is on the channels rather than on saturation, because what matters is whether
 * the colour is *light enough to be white for this purpose*. `|c00ff0303` — the Sentinel's red —
 * is maximally saturated in red and also nearly black in blue, and is nowhere near this.
 */
export function renderColour(red: number, green: number, blue: number): RenderedColour | null {
  if (red >= 230 && green >= 230 && blue >= 230)
    return null

  const [hue, saturation, sourceLightness] = toHsl(red, green, blue)
  const surface = isDark.value ? SURFACE.dark : SURFACE.light
  return solve(hue, saturation, sourceLightness, surface)
}

/**
 * Moves lightness away from the background until the contrast reaches the target.
 *
 * One direction only: dark text gets darker on a light surface, and light text gets lighter on a
 * dark one. Reversing mid-way would cross the surface's own lightness and produce text that
 * fades out before it comes back.
 *
 * The search ends at the extreme (0 or 100) rather than stopping short, so the last candidate is
 * always the most contrast the hue can give. Returning `None` therefore means "this hue cannot be
 * read at all here", not "the search gave up".
 */
function solve(
  hue: number,
  saturation: number,
  from: number,
  surface: readonly [number, number, number]
): RenderedColour | null {
  const darken = relativeLuminance(surface) > 0.18
  const extreme = darken ? 0 : 100

  let lightness = from
  let reached = 0
  for (let step = 0; step <= 50; step++) {
    // Clamped to the extreme so the loop cannot walk past it and come back from the far side.
    lightness = darken ? Math.max(extreme, from - step * 2) : Math.min(extreme, from + step * 2)
    reached = contrast(hslToRgb(hue, saturation, lightness), surface)
    if (reached >= TARGET_CONTRAST || lightness === extreme)
      break
  }

  if (reached < TARGET_CONTRAST)
    return null

  return {
    css: `hsl(${hue} ${saturation}% ${lightness}%)`,
    hue,
    saturation,
    lightness,
    contrast: Math.round(reached * 100) / 100
  }
}

/** Relative luminance, the WCAG definition. */
function relativeLuminance(rgb: readonly [number, number, number]): number {
  const [r, g, b] = [rgb[0], rgb[1], rgb[2]].map((value) => {
    const channel = value / 255
    return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
  }) as [number, number, number]
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/** The WCAG contrast ratio between two colours. */
function contrast(a: readonly [number, number, number], b: readonly [number, number, number]): number {
  const [high, low] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x) as [number, number]
  return (high + 0.05) / (low + 0.05)
}

/** RGB to HSL, hue in degrees and the other two as percentages. */
function toHsl(red: number, green: number, blue: number): [number, number, number] {
  const r = red / 255
  const g = green / 255
  const b = blue / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const span = max - min
  const lightness = (max + min) / 2

  if (span === 0)
    return [0, 0, Math.round(lightness * 100)]

  const saturation = span / (1 - Math.abs(2 * lightness - 1))
  let hue: number
  if (max === r)
    hue = ((g - b) / span) % 6
  else if (max === g)
    hue = (b - r) / span + 2
  else
    hue = (r - g) / span + 4

  hue *= 60
  if (hue < 0)
    hue += 360

  return [Math.round(hue), Math.round(saturation * 100), Math.round(lightness * 100)]
}

/** HSL back to 0–255 RGB. */
function hslToRgb(hue: number, saturation: number, lightness: number): [number, number, number] {
  const s = saturation / 100
  const l = lightness / 100
  const c = (1 - Math.abs(2 * l - 1)) * s
  const h = hue / 60
  const x = c * (1 - Math.abs((h % 2) - 1))
  let rgb: [number, number, number]
  if (h < 1)
    rgb = [c, x, 0]
  else if (h < 2)
    rgb = [x, c, 0]
  else if (h < 3)
    rgb = [0, c, x]
  else if (h < 4)
    rgb = [0, x, c]
  else if (h < 5)
    rgb = [x, 0, c]
  else
    rgb = [c, 0, x]

  const m = l - c / 2
  return rgb.map(channel => Math.round((channel + m) * 255)) as [number, number, number]
}

/** A colour per segment of a string, in order. */
export function segmentColours(text: RichText): Array<string | undefined> {
  return text.segments.map((segment) => {
    if (!segment.colour)
      return undefined
    return renderColour(segment.colour.red, segment.colour.green, segment.colour.blue)?.css
  })
}

/**
 * Colours for one string, recomputed when the scheme changes.
 *
 * The reactive read of `isDark` lives here rather than inside `segmentColours`, which is a pure
 * function: hiding a dependency in one is how a value ends up stale after a theme change.
 */
export function useSegmentColours(text: () => RichText): ComputedRef<Array<string | undefined>> {
  return computed(() => {
    void isDark.value
    return segmentColours(text())
  })
}
