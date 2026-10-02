/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'

  // An erased single-file-component type. The lint config rejects `{}` (it accepts
  // anything non-nullish) and `any`, so the parameters are spelled out as
  // "no props" and "unknown props/exposed", which is what the scaffold meant.
  // `unknown` rather than `any` costs nothing here: nothing calls through it.
  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>
  export default component
}
