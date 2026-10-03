<script setup lang="ts">
/**
 * Map text, drawn in the colours its author chose.
 *
 * # Why this is a component
 *
 * Warcraft III puts inline colour codes in map names, authors and descriptions, and the panel
 * has four such fields. Rendering them is a `v-for` over segments with a computed colour each,
 * which is three lines of template that would otherwise be copied four times, twice over — the
 * name is drawn both as a heading and in the flat field list.
 *
 * # Where the decision about lightness lives
 *
 * In `src/colour.ts`, and the reason is in its module comment: the author's hue and saturation
 * are kept, and lightness is chosen for the current scheme, because most map text is white and
 * white on this light background is invisible. This component only applies what it is given.
 */
import type { RichText } from '../types'
import { useSegmentColours, watchColorScheme } from '../colour'

const props = withDefaults(defineProps<{
  /** The runs to draw. */
  text: RichText
  /** What to show when the whole field is empty. */
  fallback?: string
  /** Whether to keep the author's line breaks. */
  multiline?: boolean
}>(), {
  fallback: '—',
  multiline: false
})

// Once per app is enough, and only a component that draws a colour needs it.
watchColorScheme()

const colours = useSegmentColours(() => props.text)

/** Whether every segment is empty, in which case the fallback is what to show. */
const blank = (): boolean => props.text.segments.every(segment => segment.text === '')
</script>

<template>
  <span v-if="blank()" class="opacity-60">{{ fallback }}</span>
  <span
    v-else
    :class="multiline ? 'whitespace-pre-wrap' : undefined"
  ><template
    v-for="(segment, i) in text.segments"
    :key="i"
  ><span :style="colours[i] ? { color: colours[i] } : undefined">{{ segment.text }}</span></template></span>
</template>
