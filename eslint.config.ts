import antfu from '@antfu/eslint-config'

export default antfu({
  unocss: true,

  // Build output, generated schemas, and the Rust side.
  //
  // `src-tauri/gen/schemas` is Tauri's own generated JSON; it is gitignored, so linting
  // it would report on files nobody reviews and nobody commits.
  //
  // ⚠️ TOML is **not** ignored, so this config also formats `Cargo.toml` and
  // `pnpm-workspace.yaml` — the default antfu ruleset includes a TOML plugin. That is
  // deliberate rather than overlooked: it means `pnpm lint` owns TOML style here, and
  // the visible consequence is that it expands single-line arrays such as
  // `crate-type = ["staticlib", "cdylib", "rlib"]` onto several lines. `cargo fmt`
  // does not touch manifests, so nothing else competes for that file. Run `pnpm lint`
  // after hand-editing a manifest, or the next run reformats it.
  ignores: [
    'dist/**',
    'src-tauri/gen/**',
    'src-tauri/target/**'
  ],

  // The reference project disables markdown linting; this one has no markdown sources
  // to lint beyond the README, which the default would try to reformat.
  markdown: false,

  rules: {
    'style/comma-dangle': ['warn', 'never']
  },

  vue: {
    overrides: {
      'vue/comma-dangle': ['warn', 'never']
    }
  },

  jsonc: {
    overrides: {
      'jsonc/comma-dangle': ['warn', 'never']
    }
  }
})
