<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
/**
 * Settings: which Warcraft III installation the app should use.
 *
 * # Why this screen exists at all
 *
 * Two features need the installation directory and neither can be done without it: starting the
 * game on a map, and browsing the maps already installed. The path is stored in
 * `~/.w3wright/settings.json`, which this panel names rather than hides — a user who wants to
 * change it with a text editor should not have to find out where it is by guessing.
 *
 * # Why the directory is validated on save but never corrected
 *
 * The backend checks for `War3.exe` and says which of the three things is wrong: the path does
 * not exist, it is a file, or it is a directory without the game in it. It does **not** then
 * substitute a directory it found on its own — a user who typed one path and watches the app use
 * another has been overruled by their own tool. Detection is offered as a suggestion instead.
 */
import { computed, onMounted, ref } from 'vue'
import {
  busy,
  checkWar3Dir,
  ensureStatus,
  error,
  game,
  notice,
  refreshStatus,
  saveWar3Dir,
  settings,
  status
} from '../composables/useSettings'
import EmptyState from './EmptyState.vue'

/** The directory being edited, seeded from what is stored. */
const draft = ref('')

/** What the last check said, so "and it works" is visible before saving. */
const checked = ref<{ ok: boolean, message: string } | null>(null)

onMounted(async () => {
  await ensureStatus()
  draft.value = settings.value?.war3Dir ?? ''
})

/**
 * Where the current installation came from, in the interface's words.
 *
 * The three cases are not interchangeable and the panel must not present them as one: a path the
 * user configured is a decision, and a path found in the registry is the app's guess on their
 * behalf. The backend sends its own names for them; this is the sentence a reader needs.
 */
const sourceNote = computed(() => {
  switch (game.value?.source) {
    case 'configured':
      return 'from your settings'
    case 'registry':
      return 'found in the Windows registry — not yet saved to your settings'
    case 'common':
      return 'found in a directory the game is commonly installed in — not yet saved'
    default:
      return ''
  }
})

/** Whether the draft differs from what is stored. */
const dirty = computed(() => draft.value.trim() !== (settings.value?.war3Dir ?? ''))

/** Checks the draft without saving it. */
async function onCheck(): Promise<void> {
  checked.value = null
  const found = await checkWar3Dir(draft.value)
  if (found)
    checked.value = { ok: true, message: `That is a Warcraft III directory (${found.exe})` }
}

/** Saves the draft. */
async function onSave(): Promise<void> {
  checked.value = null
  if (await saveWar3Dir(draft.value))
    draft.value = settings.value?.war3Dir ?? ''
}

/** Clears the stored directory without touching the draft. */
async function onClear(): Promise<void> {
  checked.value = null
  if (await saveWar3Dir(''))
    draft.value = ''
}

/** Asks the system for a directory and puts it in the draft. */
async function onPick(): Promise<void> {
  checked.value = null
  try {
    const chosen = await openDialog({
      directory: true,
      multiple: false,
      title: 'Select the Warcraft III directory'
    })
    // `null` is a cancelled dialog, which is not an error and not a change.
    if (typeof chosen === 'string')
      draft.value = chosen
  }
  catch (e) {
    error.value = `the folder dialog could not be opened: ${String(e)}`
  }
}

/** Re-runs detection, for a user who has just installed the game. */
async function onDetect(): Promise<void> {
  checked.value = null
  await refreshStatus()
}
</script>

<template>
  <section class="page-body">
    <header>
      <h1 class="page-title">
        Settings
      </h1>
      <p class="page-note">
        Where the game is installed. Used to start the game on a map, and to browse the maps you
        already have.
      </p>
    </header>

    <EmptyState
      v-if="status === null && !busy"
      what="the app settings"
    />

    <template v-else>
      <dl class="field-list">
        <dt class="field-term">
          in use
        </dt>
        <dd class="field-value flex flex-col items-start gap-0.5">
          <template v-if="game">
            <span class="mono">{{ game.dir }}</span>
            <span class="text-0.8rem opacity-70">{{ sourceNote }}</span>
            <span
              v-if="!game.hasMaps"
              class="text-0.8rem opacity-70"
            >
              ⚠️ no <span class="mono">Maps</span> directory in it, so the map browser has nothing
              to show
            </span>
          </template>
          <span v-else class="opacity-60">
            none found — start the game and browse installed maps need one
          </span>
        </dd>

        <dt class="field-term">
          settings file
        </dt>
        <dd class="field-value text-0.85rem mono">
          {{ status?.settingsPath ?? "—" }}
        </dd>
      </dl>

      <h2 class="section-head">
        game directory
      </h2>
      <div class="max-w-40rem flex flex-col items-start gap-2">
        <input
          v-model="draft"
          type="text"
          spellcheck="false"
          placeholder="D:\Warcraft3"
          class="settings-input mono"
          aria-label="Warcraft III directory"
          @keyup.enter="onSave"
        >
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="settings-btn"
            :disabled="busy"
            @click="onPick"
          >
            Browse…
          </button>
          <button
            type="button"
            class="settings-btn"
            :disabled="busy || !draft.trim()"
            @click="onCheck"
          >
            Check
          </button>
          <button
            type="button"
            class="settings-btn primary"
            :disabled="busy || !dirty"
            @click="onSave"
          >
            Save
          </button>
          <button
            type="button"
            class="settings-btn"
            :disabled="busy || !settings?.war3Dir"
            @click="onClear"
          >
            Clear
          </button>
          <button
            type="button"
            class="settings-btn"
            :disabled="busy"
            @click="onDetect"
          >
            Detect
          </button>
        </div>

        <!--
          The check and the save are separate answers: one says the directory is right, the other
          says it was remembered. Showing them in one place is what stops "Check" reading as a
          successful save.
        -->
        <p
          v-if="checked"
          class="m-0 text-0.85rem"
          :class="checked.ok ? 'text-green-700' : 'text-red-700'"
        >
          {{ checked.message }}
        </p>
        <p v-if="notice" class="m-0 text-0.85rem text-green-700">
          {{ notice }}
        </p>
        <p v-if="error" class="m-0 break-all text-0.85rem text-red-700">
          {{ error }}
        </p>
        <p class="m-0 max-w-36rem text-0.8rem opacity-70">
          The check looks for <span class="mono">War3.exe</span> in the directory. Detection reads
          the Windows registry and a few usual locations; nothing is saved until you press Save.
        </p>
      </div>
    </template>
  </section>
</template>

<style scoped>
/*
 * The input and buttons are scoped rules rather than utility classes because both express things
 * utilities cannot: the input's width has to follow the container (a path is long and a fixed
 * width either clips it or leaves a gap), and the buttons need `disabled` styling, which UnoCSS
 * would have to express as a variant on every one of the five.
 */
.settings-input {
  width: 100%;
  padding: 0.4rem 0.6rem;
  font-size: 0.85rem;
  color: inherit;
  background: #fff;
  border: 1px solid rgb(0 0 0 / 18%);
  border-radius: 4px;
}

.settings-input:focus {
  border-color: var(--accent);
  outline: 2px solid rgb(47 111 235 / 25%);
}

.settings-btn {
  padding: 0.35rem 0.8rem;
  font-size: 0.85rem;
  color: inherit;
  cursor: pointer;
  background: #fff;
  border: 1px solid rgb(0 0 0 / 18%);
  border-radius: 4px;
}

.settings-btn:hover:not(:disabled) {
  background: rgb(0 0 0 / 4%);
}

.settings-btn:disabled {
  cursor: default;
  opacity: 45%;
}

.settings-btn.primary {
  color: var(--on-accent);
  background: var(--accent);
  border-color: var(--accent);
}

.settings-btn.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

@media (prefers-color-scheme: dark) {
  .settings-input,
  .settings-btn {
    background: #2b2b2b;
    border-color: rgb(255 255 255 / 22%);
  }

  .settings-btn:hover:not(:disabled) {
    background: rgb(255 255 255 / 8%);
  }

  .settings-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
  }
}
</style>
