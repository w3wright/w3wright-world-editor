# w3wright-world-editor

Desktop studio for the W3wright Warcraft III map development platform.

This is its **own project in its own repository**. It is deliberately not a
member of the `w3wright` workspace one directory up: it consumes those crates the
way any other consumer does — by version — so that "builds here" and "builds from
crates.io" stay the same question.

> The design this implements lives in the `w3wright/docs` design set. The rule
> that matters here is its §1.1: **the UI never reimplements format logic.**
> Every question of the form "what do these bytes mean" is answered by `crates/`,
> including in its WASM form. A second MPQ reader in the front end is the thing
> to avoid.

## Status

Phase 4 of the design set — the app shell and the first real panel. It opens a
window, calls Rust, and shows a map's name, author, description, members and
diagnostics. No 3D viewport and no WebGPU yet; those are Phase 5, and the WebGPU
smoke test in ADR-0008 is the first task of that phase, not this one.

What the first panel does **not** do yet: pick a file through a dialog (the path is
typed), edit anything, or show object data. Editing is the increment that turns this
from a viewer into an editor, and it needs the save loop to be verified against the
engine first.

## Layout

```text
w3wright-world-editor/
├── Cargo.toml            # workspace of one (see the comment there — it is load-bearing)
├── .cargo/config.toml    # dev-only: patches war3-* to the local checkout
├── index.html            # Vite entry
├── src/                  # Vue 3 + TypeScript front end
│   ├── types.ts          # mirrors src-tauri/src/dto.rs by hand
│   ├── format.ts         # byte sizes, file names
│   └── components/
├── src-tauri/            # the Rust app: Tauri config, commands, DTOs
└── package.json
```

## ⚠️ `Map::files` is not the member list

The one trap worth knowing before adding a panel:

`war3_map::Map::files` looks like the archive's member list and is not. It is the
intersection of the archive with `war3_archive::KNOWN_MEMBER_NAMES`, a hardcoded
probe list used when an archive does not enumerate itself. Anything outside that
list — the minimap `war3mapMap.blp`, any imported asset — is **absent from it, with
no diagnostic**. `(4)LostTemple.w3m` has 16 members and `Map::files` reports 15.

`war3_archive::Archive::file_names()` is the real list. The summary command opens
the file with both crates for this reason; see the comment on `open_map`.

## Working on it

```bash
pnpm install
pnpm tauri dev      # dev build, hot-reloading front end
pnpm tauri build    # release build and bundle
```

The first Rust build compiles Tauri's dependency tree from scratch and takes
roughly half an hour on this machine. Later builds are incremental.

## The two dependency modes

This project has to build two ways, and the mechanism that makes both work is a
`[patch]` in `.cargo/config.toml`:

| Mode | `war3-*` comes from |
| --- | --- |
| **Development** | the local `../w3wright` checkout, via the patch |
| **Release** | crates.io, by the versions declared in `src-tauri/Cargo.toml` |

`src-tauri/Cargo.toml` always declares **published versions** — it never mentions
a path. The patch lives in config because config is never packaged, so a
published artefact cannot carry a filesystem path into someone else's build.

### ⚠️ Known blocker: a release build cannot resolve its dependencies yet

| Crate | crates.io | local | used here |
| --- | --- | --- | --- |
| `war3-map` | `0.0.2` | `0.0.3` | the map model |
| `war3-archive` | `0.0.2` | `0.0.3` | the real member list |
| `war3-core`, `war3-terrain` | `0.0.2` | `0.0.3` | transitively |
| `war3-project` | **not published** | `0.0.3` | not yet — the save loop |
| `war3-object` | **not published** | `0.0.3` | not yet — the object editor |

Only the crates actually used are declared. Seven went in when this app was
scaffolded and it used none of them, which is a compile cost and a claim about the
design that was not true.

**The blocker is the version, not the missing crates.** Cargo treats `0.0.x` as
exact, so a requirement of `"0.0.3"` is **not** satisfied by a published `0.0.2`.
Until `0.0.3` is published, only a build with the local patch works.

### Release checklist

1. In the `w3wright` checkout, bump `[workspace.package] version` and the
   `version` keys in `[workspace.dependencies]` together, then run
   `cargo package --workspace` from a **fresh** target directory to verify.
2. Publish the crates this app uses. `war3-project` and `war3-object` are not needed
   until the features that use them land — publish them with that change.
3. Update the version strings in `src-tauri/Cargo.toml` to the published version.
4. Build with the patch **disabled** to prove the release path works — nothing
   else proves it, because the patch hides the failure. Temporarily rename
   `.cargo/config.toml`, build, then rename it back.

Step 4 is the one that gets skipped and then breaks a release at the worst time.

## Why there is a workspace of one

`Cargo.toml` here has a `[workspace]` table with `members = ["src-tauri"]`. That
is not ceremony. Without it Cargo walks **up** the directory tree looking for a
workspace root, finds the `w3wright` checkout when it happens to be a sibling,
and refuses to build:

```text
error: current package believes it's in a workspace when it's not
```

So the table ends the search here, and the build must never depend on a
neighbouring checkout existing. See the comment in `Cargo.toml` before removing
it.

## IDE setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
