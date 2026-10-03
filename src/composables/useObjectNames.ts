import type { NameAnswer, NameQuery } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

/**
 * Object names, resolved by the core and remembered here.
 *
 * # What is cached, and why this is the right place
 *
 * ⚠️ The backend already keeps the expensive part — `names.rs` holds one `Resolver` for the life of
 * the window, so opening the game's archives happens once rather than per lookup (measured: 593 ms
 * to open, 0.37 µs to resolve). What remains is the **IPC round trip**, and that is what this cache
 * removes: a unit view renders 121 rows, and without this each row would ask the backend about its
 * own type.
 *
 * # Why the batches are coalesced
 *
 * A table's rows resolve in the same tick. Naively that is 121 calls; here the first call starts a
 * batch, the rest join it, and the whole screen costs **one** round trip. The pending map is keyed by
 * `kind:id` and holds the promise rather than the promise's value, so a second asker waits on the
 * first asker's request instead of starting its own.
 *
 * # Why a module-level cache
 *
 * Same argument `useOpenMap` makes for the open map: there is one of these per window, every consumer
 * is a component, and a store would add a provider to manage it. ⚠️ What it must not become is a
 * cache that outlives its inputs — the names depend on the game installation, and `forgetNames` is
 * called when the settings are saved. Nothing else can change them: the files are a game
 * installation's, and those do not change under a running editor.
 */

/** One resolved name. */
export interface ResolvedName {
  /** What to show. */
  name: string
  /** `map`, `game` or `id` — the core's own label for where it came from. */
  source: string
}

/** Names already resolved, by `kind:id`. */
const resolved = new Map<string, ResolvedName>()

/** Batches in flight, by `kind:id`, so a second asker joins the first one's request. */
const pending = new Map<string, Promise<void>>()

/**
 * A counter that changes when everything is forgotten.
 *
 * Components read it so that a `forgetNames` re-renders them: a `Map` is not reactive, and without
 * this a panel would keep showing the previous installation's names until something else re-rendered
 * it — which for a table nobody scrolls means "until the map is reopened".
 */
export const namesGeneration = ref(0)

/** The cache key for one object. */
function keyOf(query: NameQuery): string {
  return `${query.kind}:${query.id}`
}

/** The name for one ID, or `null` when it has not been resolved yet. */
export function peekName(kind: string, id: string): ResolvedName | null {
  return resolved.get(`${kind}:${id}`) ?? null
}

/**
 * Resolves everything asked for that is not already known.
 *
 * `onReady` is called once when every name in the batch is available. A component passes its own
 * re-render trigger, so a batch of 121 rows produces 121 independent waits on one request rather than
 * one component re-rendering for all of them.
 */
export async function resolveNames(queries: NameQuery[], onReady: () => void): Promise<void> {
  const wanted = queries.filter(query => !resolved.has(keyOf(query)))

  if (wanted.length === 0) {
    // Everything was already known, which is the common case for the second and later rows of one
    // type. Nothing to await, but the caller still has to be told to look again.
    onReady()
    return
  }

  // Join an in-flight batch for anything already being asked about, and start one for the rest.
  const awaited: Array<Promise<void>> = []
  const fresh: NameQuery[] = []
  for (const query of wanted) {
    const key = keyOf(query)
    const inFlight = pending.get(key)
    if (inFlight) {
      awaited.push(inFlight)
    }
    else {
      fresh.push(query)
    }
  }

  if (fresh.length > 0) {
    const request = invoke<NameAnswer[]>('resolve_names', { queries: fresh })
      .then((answers) => {
        for (const answer of answers)
          resolved.set(keyOf(answer), { name: answer.name, source: answer.source })
      })
      .catch(() => {
        // ⚠️ A failure is not fatal here, and deliberately so: a table must not fail to render
        // because a name could not be looked up. The ID stands in, which is what an object the game
        // has never heard of shows anyway — so the degradation is one the interface already has a
        // shape for rather than a new error path.
        for (const query of fresh)
          resolved.set(keyOf(query), { name: query.id, source: 'id' })
      })
      .finally(() => {
        for (const query of fresh)
          pending.delete(keyOf(query))
      })

    for (const query of fresh)
      pending.set(keyOf(query), request)
    awaited.push(request)
  }

  await Promise.all(awaited)
  onReady()
}

/**
 * The kinds a `techList` entry can be.
 *
 * ⚠️ A `techList` — `ureq` is one — holds ids whose **kind is not stated**: `R00M` is an upgrade,
 * `hfoo` would be a unit, `A000` an ability. The game resolves them by trying, and so must this. The
 * four below are the kinds the Object Editor's tech tree can reference; `buff`, `doodad` and
 * `destructable` cannot appear in one, which is why this is four queries per id and not seven.
 *
 * Kept beside `ElementKind` because the two are the same vocabulary read in different directions.
 */
export const TECH_KINDS: readonly string[] = ['unit', 'item', 'ability', 'upgrade']

/**
 * Forgets every name, for when the game installation changes.
 *
 * Called from the settings save path. In-flight batches are left to finish — they will write their
 * answers, and the generation bump means the next render asks again, so a stale answer is corrected
 * rather than displayed forever.
 */
export function forgetNames(): void {
  resolved.clear()
  namesGeneration.value += 1
}

/** How many names are cached, for a report or a test. */
export function cachedNameCount(): number {
  return resolved.size
}
