import { computed, ref } from 'vue'

/**
 * What the application is busy with, for the status bar.
 *
 * # Why this exists rather than a spinner per screen
 *
 * Opening a map takes seconds, and most of that is one call that cannot report anything until it is
 * done. A spinner in the middle of a panel says "this panel is loading"; a **status bar** says what
 * the application* is doing, keeps saying it if the user navigates away mid-operation, and has room
 * for the one fact that makes a wait tolerable — what step it is on.
 *
 * # Why the details are kept after it finishes
 *
 * ⚠️ The last operation's steps stay in the tooltip once the work is done, and that is the point of
 * the tooltip rather than a leftover. "It took 4 s" is not actionable; "it took 4 s, and 3.1 s of that
 * was opening the archive" is. Discarding the steps on success would keep the only interesting case —
 * a slow load — permanently unexplainable.
 *
 * # Why the newest operation wins
 *
 * Two operations can overlap (a map open and a name batch), and a bar with two lines is a bar nobody
 * reads. The newest becomes the label and the rest are counted, so the display stays one line and
 * still says that something else is happening.
 */

/** One step of an operation. */
export interface TaskStep {
  /** What this step is doing, as shown to the user. */
  label: string
  /** How long it took, in milliseconds. `null` while it is still running. */
  ms: number | null
  /**
   * When it started, on the same clock as everything else here.
   *
   * ⚠️ Carried on the step rather than in a side table. A `Map` of start times keyed by task and index
   * was the first attempt, and it needed an entry created, updated and deleted in four places —
   * exactly the bookkeeping that goes wrong silently, leaving a step that is never closed and a
   * tooltip claiming work is still happening.
   */
  startedAt: number
}

/** One operation that is running or has just finished. */
export interface TaskState {
  /** Stable id, so a caller can update its own task without holding a reference. */
  id: number
  /** What the operation is doing overall, e.g. `Opening a map`. */
  label: string
  /** The steps so far, oldest first. */
  steps: TaskStep[]
  /** How long ago it started, in milliseconds, refreshed by the ticker while it runs. */
  elapsed: number
  /** Whether it is still running. */
  running: boolean
}

/** A fresh id per task, so two operations never collide. */
let nextId = 1

/** Everything currently running, oldest first. */
const running = ref<TaskState[]>([])

/** The last operation that finished, kept for the tooltip. */
const last = ref<TaskState | null>(null)

/**
 * When each running task started, by id.
 *
 * ⚠️ `performance.now()` and not `Date.now()`: the former is monotonic, so a system clock adjustment
 * during a multi-second load cannot produce a negative or wildly wrong elapsed time.
 */
const starts = new Map<number, number>()

/** The elapsed-time ticker, so a second task does not install a second one. */
let ticker: ReturnType<typeof setInterval> | null = null

/**
 * Whether the application is busy.
 *
 * Read by the status bar. A `computed` rather than a plain getter so a component re-renders on the
 * transition and not on every elapsed-time tick.
 */
export const isBusy = computed(() => running.value.length > 0)

/** Every operation in flight. */
export const activeTasks = computed(() => running.value)

/**
 * The operation the bar shows: the newest running one, or the last finished one.
 *
 * ⚠️ Newest rather than first, because the newest is the one the user just asked for. A long name
 * load still finishing under a map the user has already closed should not hold the bar.
 */
export const currentTask = computed<TaskState | null>(() => {
  const tasks = running.value
  return tasks.length > 0 ? tasks[tasks.length - 1] : last.value
})

/**
 * How many operations are running besides the one displayed.
 *
 * Shown as `+1` rather than as a second line: two concurrent operations should be visible without
 * making the bar tall.
 */
export const queuedCount = computed(() => Math.max(0, running.value.length - 1))

/**
 * Starts an operation, with its first step already open.
 *
 * Returns its id, which is how the caller reports steps and finishes it. Ids are numbers rather than
 * the returned object so that a caller cannot keep a stale reference alive — the array is replaced on
 * every change, so a held object would not be updated.
 */
export function startTask(label: string): number {
  const now = performance.now()
  const id = nextId++
  const task: TaskState = {
    id,
    label,
    steps: [{ label, ms: null, startedAt: now }],
    elapsed: 0,
    running: true
  }
  starts.set(id, now)
  running.value = [...running.value, task]
  startTicker()
  return id
}

/**
 * Adds a step to a running operation, closing the previous one.
 *
 * Closing against the clock is what makes the tooltip a sequence of **timings** rather than a list of
 * labels: "opening the archive — 3.1 s" is the fact that explains a slow load, and it cannot be
 * recovered afterwards.
 */
export function addStep(id: number, label: string): void {
  const now = performance.now()
  running.value = running.value.map((task) => {
    if (task.id !== id)
      return task
    const steps = task.steps.map((step, i) =>
      i === task.steps.length - 1 && step.ms === null ? { ...step, ms: now - step.startedAt } : step)
    return { ...task, steps: [...steps, { label, ms: null, startedAt: now }] }
  })
}

/** Updates only the overall label of a running operation. */
export function setTaskLabel(id: number, label: string): void {
  running.value = running.value.map(task => (task.id === id ? { ...task, label } : task))
}

/**
 * Finishes an operation.
 *
 * The last step is closed and the whole thing moves to `last`, so the tooltip keeps its timings. A
 * task that is not running is ignored rather than an error: two code paths can both finish a task
 * when one of them failed, and a duplicate finish is not worth a thrown exception.
 */
export function finishTask(id: number): void {
  const task = running.value.find(t => t.id === id)
  if (!task)
    return
  const now = performance.now()
  const steps = task.steps.map((step, i) =>
    i === task.steps.length - 1 && step.ms === null ? { ...step, ms: now - step.startedAt } : step)
  const done: TaskState = {
    ...task,
    steps,
    elapsed: now - (starts.get(id) ?? now),
    running: false
  }
  running.value = running.value.filter(t => t.id !== id)
  last.value = done
  starts.delete(id)
  // The ticker stops itself; this just avoids waiting up to 100 ms for it when nothing is left.
  if (running.value.length === 0)
    stopTicker()
}

/**
 * Forgets the last finished operation.
 *
 * Called when the user starts something new of their own accord, so a stale "opened" line does not
 * sit in the bar looking like current work.
 */
export function clearLastTask(): void {
  last.value = null
}

/**
 * Runs `body` as one step of an operation.
 *
 * ⚠️ The step is added **before** `body` runs and not after, so the label appears while the work is
 * happening — which is the entire point. A step recorded on completion would describe the past.
 */
export async function step<T>(id: number, label: string, body: () => Promise<T>): Promise<T> {
  addStep(id, label)
  return await body()
}

// --- timing -----------------------------------------------------------------------------------
//
// The state itself (`starts`, `ticker`) is declared at the top of the file with the other module
// state, because every function here touches it and `ts/no-use-before-define` is right to object to
// the alternative.

/** Starts the elapsed-time ticker if it is not already running. */
function startTicker(): void {
  if (ticker !== null)
    return
  ticker = setInterval(() => {
    if (running.value.length === 0) {
      stopTicker()
      return
    }
    const now = performance.now()
    running.value = running.value.map(task => ({ ...task, elapsed: now - (starts.get(task.id) ?? now) }))
  }, 100)
}

/** Stops the ticker. */
function stopTicker(): void {
  if (ticker !== null) {
    clearInterval(ticker)
    ticker = null
  }
}
