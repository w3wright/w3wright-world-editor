<script setup lang="ts">
/**
 * The status bar: what the application is doing, and how long it has been doing it.
 *
 * # Why a status bar rather than a spinner in the panel
 *
 * Opening a map takes seconds, most of it inside one call. A spinner where the user clicked says "this
 * panel is loading" and disappears the moment they navigate; this bar says what the **application** is
 * doing, keeps saying it while they look elsewhere, and has room for the fact that makes a wait
 * bearable — which step is taking the time.
 *
 * # The tooltip is the feature
 *
 * A one-line bar can only say "Opening a map… 3.4s". The tooltip carries the breakdown, and ⚠️ it
 * stays populated **after** the work finishes:
 *
 * ```text
 * Opening a map                        4.2s
 *   Opening the archive                3.1s
 *   Reading the map                    0.6s
 *   Reading object names               0.5s
 * ```
 *
 * "It took 4 s" is not actionable; "3.1 s of it was opening the archive" is. Clearing the steps on
 * success would make the one case worth investigating — a slow load — permanently unexplainable.
 *
 * # Why the spinner is CSS and not an image
 *
 * A rotation is the one animation a status indicator needs, and UnoCSS utilities cannot express a
 * keyframe. The `<style>` block is therefore deliberate and minimal: a rule that says why it is here,
 * not a place for layout that utilities can do.
 */
import { computed } from 'vue'
import {
  currentTask,
  isBusy,
  queuedCount
} from '../composables/useTasks'

/** The label on the bar, e.g. `Opening a map`. */
const label = computed(() => currentTask.value?.label ?? 'Ready')

/** Elapsed seconds, one decimal, as the bar shows them. */
const elapsed = computed(() => {
  const task = currentTask.value
  if (!task)
    return ''
  const seconds = (task.running ? task.elapsed : task.elapsed) / 1000
  return `${seconds.toFixed(1)}s`
})

/** The tooltip: the operation, then every step with its own timing. */
const tooltip = computed(() => {
  const task = currentTask.value
  if (!task)
    return ''
  const lines = [`${task.label} — ${(task.elapsed / 1000).toFixed(1)}s`]
  for (const step of task.steps) {
    // ⚠️ A step with no time is the one running now, and it is marked as such rather than shown as
    // `0.0s` — a zero would read as "instant" and hide which step the wait is in.
    const took = step.ms === null ? 'running…' : `${(step.ms / 1000).toFixed(1)}s`
    lines.push(`  ${step.label} — ${took}`)
  }
  return lines.join('\n')
})
</script>

<template>
  <footer
    class="status-bar"
    :title="tooltip"
    :class="isBusy ? 'status-bar-busy' : ''"
  >
    <!--
      ⚠️ The spinner is present when idle too, at rest and dimmed, rather than appearing and
      disappearing. A bar whose content shifts sideways every time work starts is one the eye has to
      re-find; a fixed icon that starts turning does not move anything.
    -->
    <span class="spinner" :class="isBusy ? 'spinner-on' : ''" aria-hidden="true" />
    <span class="status-label">{{ label }}</span>
    <span v-if="elapsed" class="status-time">{{ elapsed }}</span>
    <!--
      Other operations, counted rather than listed: a bar with two lines is a bar nobody reads, and the
      tooltip already names the one being shown.
    -->
    <span v-if="queuedCount > 0" class="status-queued">+{{ queuedCount }}</span>
    <span class="status-spacer" />
    <!--
      The hint is the bar's other job: it is the only always-visible place that can teach the tooltip
      without a tour. Dimmed so it does not compete with the state.
    -->
    <span class="status-hint">hover for details</span>
  </footer>
</template>

<style scoped>
/*
 * ⚠️ A `<style>` block, against the usual rule that utilities only. Two things here cannot be a
 * utility: a rotation needs a keyframe, and the bar has to be pinned to the bottom of a flex column
 * regardless of what the page does. Everything else — colours, spacing, the dimmed hint — is a class
 * from base.css or a utility.
 */
.status-bar {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  height: 1.6rem;
  padding: 0 0.6rem;
  flex: none;
  font-size: 0.78rem;
  color: rgb(0 0 0 / 62%);
  background: rgb(0 0 0 / 4%);
  border-top: 1px solid rgb(0 0 0 / 10%);
  user-select: none;
}

.status-bar-busy {
  color: rgb(0 0 0 / 82%);
}

@media (prefers-color-scheme: dark) {
  .status-bar {
    color: rgb(255 255 255 / 62%);
    background: rgb(255 255 255 / 6%);
    border-top-color: rgb(255 255 255 / 12%);
  }

  .status-bar-busy {
    color: rgb(255 255 255 / 88%);
  }
}

.status-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status-time,
.status-queued,
.status-hint {
  font-variant-numeric: tabular-nums;
  opacity: 70%;
}

.status-spacer {
  flex: 1;
}

/*
 * The spinner: a ring with one coloured quarter, rotating. `border-top-color` is the accent so it
 * reads as the same interface as the buttons; the rest of the ring is transparent, which is what
 * makes the rotation visible without a second element.
 */
.spinner {
  width: 0.72rem;
  height: 0.72rem;
  flex: none;
  border: 1.5px solid rgb(0 0 0 / 22%);
  border-top-color: rgb(0 0 0 / 22%);
  border-radius: 50%;
  opacity: 55%;
}

.spinner-on {
  border-color: rgb(0 0 0 / 18%);
  border-top-color: var(--accent);
  opacity: 100%;
  animation: status-spin 0.7s linear infinite;
}

@media (prefers-color-scheme: dark) {
  .spinner {
    border-color: rgb(255 255 255 / 26%);
    border-top-color: rgb(255 255 255 / 26%);
  }

  .spinner-on {
    border-color: rgb(255 255 255 / 20%);
    border-top-color: var(--accent);
  }
}

/*
 * ⚠️ `prefers-reduced-motion` is honoured rather than ignored: a continuously rotating element is
 * exactly what that setting exists for, and the label already says work is happening.
 */
@media (prefers-reduced-motion: reduce) {
  .spinner-on {
    animation: none;
    border-top-color: var(--accent);
  }
}

@keyframes status-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
