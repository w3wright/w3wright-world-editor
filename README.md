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

Phase 4 of the design set — the app shell and the read-only views. It opens a window,
calls Rust, and shows a map's name, author, description, members, diagnostics, its
terrain, its placed units and doodads, and its object data. No 3D viewport and no
WebGPU yet; those are Phase 5, and the WebGPU smoke test in ADR-0008 is the first task
of that phase, not this one.

What these panels do **not** do yet: edit anything, show a doodad or unit in 3D, or
resolve object field ids to names — the last needs the game's `*MetaData.slk` tables,
and there is no game-directory setting.

## The window draws its own chrome

`decorations: false` in `src-tauri/tauri.conf.json` removes the native title bar, so the
app draws it: the drag region, the window controls, and a menu bar whose categories **pop
out** over the window. Maps are opened through the system file picker or the installed-map
browser, not by typing a path — with no native menu there is nowhere for a stray text box to
live, and a dialog cannot produce a typo.

⚠️ Four things about this are not obvious and each cost real debugging time; they are
written down where the next person will look. The short version:

- `window.toggleMaximize()` needs `core:window:allow-toggle-maximize` — it is not
  `maximize` + `unmaximize`. See [`src-tauri/README.md`](src-tauri/README.md).
- The capability must use `"windows": ["*"]`; `["main"]` resolves to an empty window set and
  denies every command. Same file.
- UnoCSS's `dark:` variant compiles to a `.dark` **ancestor class**, which this app never
  sets — it follows the system scheme with `prefers-color-scheme`. So `dark:bg-…` matches
  nothing, and the menu popup was white-on-white in a dark window until its surface moved
  into [`base.css`](src/styles/base.css). Use that media query, not the variant.
- The popup's `left` is **measured from the category button that opened it**, not fixed in
  CSS. It was `left: 0.375rem` and looked right while `File` was the only category with
  entries — it happened to be the first button. The moment `Run` got a menu, its popup
  appeared under `File`, which reads as the wrong menu rather than a misplaced one.

The menu is one table in [`src/nav.ts`](src/nav.ts), and it is the **whole** of the app's
navigation — there is no sidebar, so `File ▸ Open Map…` and `Map ▸ Terrain` are the only way
from one screen to another. `Edit`, `Build` and `Help` are listed and disabled, each saying on
hover what it is waiting for, rather than hidden: a reader comparing the app against
`docs/03` §1.1 should be able to see the gap. A screen here is one panel of read-only
measurements, so a second pane had nothing to hold.

The app opens on `/welcome`: one sentence and one button. It is not in the menu and not
reachable from it — it is where the window starts and where `File ▸ Close Map` returns to, since
"nothing is open" both before and after are the same state. `FirstScreen`-style guidance belongs
in the place a user is stuck, and every screen shows a short empty state of its own when a map
is not open.

## Map text is decoded and drawn in the author's colours

A map name in the wild is not a name: it is `|cffffff00IMBA 3.83f AI|r`, or
`羊羊快跑4.34|CFF1FBF00最终正式版` with no reset at all — the colour runs to the end of the string.
Descriptions carry `|n` breaks and more of the same.

Those codes are a conclusion about what the bytes mean, so the decoder lives in the core —
`war3_map::plain` / `war3_map::spans` in `crates/war3-map/src/text.rs` — and `war3 map info`
prints with it too. The two views cannot disagree about what a map is called.

⚠️ Three things that module records and a re-implementation would get wrong:

- The byte order is `AARRGGBB`, not `RRGGBBAA`. The commonest code, `|cffffff00`, reads as the
  same colour either way, which is exactly why it is a trap.
- The alpha byte is `00` on most real text — the Sentinel's `|c00ff0303` — so honouring it would
  make the two commonest faction colours invisible. It is not sent to the interface at all.
- `|C` in upper case is real and the reset is optional. Measured across 168 local maps, only
  `|c` and `|r` occur; `|n` and `||` are handled so a map that uses them is not mangled, and are
  not claimed to have been seen.

The colours are **rendered**, not stripped: [`StyledText.vue`](src/components/StyledText.vue)
draws each run, and [`colour.ts`](src/colour.ts) keeps the author's hue and saturation while
solving the lightness for WCAG AA against the current surface. Drawing the stored RGB is not an
option — most map text is white, and white on this light background is invisible. A fixed
lightness band does not work either: measured, pure yellow is 2.49:1 at HSL lightness 32 on this
surface and a blue in the same band is 8:1, so the lightness has to be derived from each
colour's own contrast.

## The record lists are capped on purpose

`(4)LostTemple.w3m` holds **121 units and 5,317 doodads**. Serialising all of them
across the IPC bridge to render a table nobody scrolls is the wrong default, so the
detailed records stop at `DETAIL_LIMIT` (200) and the panel states in words how many it
is not showing. A silent truncation would make a partial list look complete.

The aggregates — file version, record count, distinct types, the ranked type histogram,
the per-player counts — are computed **in Rust**, and they are the same figures
`war3 map units` and `war3 map doodads` print. Two implementations of "most common
types" would be free to disagree about a map without anyone noticing which was right.

⚠️ `war3 map units` prints **twelve** type rows and `(4)LostTemple.w3m` has **22**
distinct unit types. The DTO carries the full histogram as well as the ranked head, so
the panel can say "the 12 commonest of 22" instead of presenting a head as the whole.

## Where the game is, and what that buys

`~/.w3wright/settings.json` holds one thing so far:

```json
{
  "war3Dir": "D:\\Warcraft3"
}
```

It is a plain file in the home directory rather than a plugin's private store, because two
readers want it and only one of them is the app: a user who wants to point the editor at another
installation should be able to edit it in a text editor, and a future command-line tool needs the
same answer without going through a webview.

Two features need that directory, and `File ▸ Settings` is where it is set:

- **`Run ▸ Play in Warcraft III`** starts the game on the open map, via
  `War3.exe -loadfile <map>` with the working directory set to the installation. ⚠️ The working
  directory matters: the game resolves `war3.mpq` and `War3Patch.mpq` relative to itself, so
  launching it with an inherited one is how "the game starts and cannot find its own data"
  happens. The entry is disabled until a map is open **and** a game is configured, and its
  tooltip says which of the two is missing — those send a user to two different places.
- **`File ▸ Browse Installed Maps…`** shows everything under `<war3Dir>/Maps` as a tree, so a map
  can be picked without a file dialog. The dialog opens wherever it was last used and leaves the
  user to remember whether a map is in `Maps`, `Maps\Download` or `Maps\FrozenThrone\Scenario`;
  this shows the whole structure at once. Each folder starts collapsed and shows how many maps are
  inside it.

⚠️ **Everything the app does is in the menu bar.** A `▶ Play` button lived in the title bar for
one increment and was moved here: that bar's job is the window — the drag region, the map's name,
whether a command is running, and the three controls — and starting another program is none of
those. Having it there also made the bar two things at once, chrome *and* the only command
outside the menu.

`Run` is its own category rather than an entry in `File`, because launching is the one action
whose effect is **outside this window**. Putting it among two ways of opening a file, a way of
closing one and a settings screen would have made `File` mean "everything".

⚠️ **The browser offers, it does not validate.** Its extension filter is the game's, and the core
decides what is really a map when one is opened — a file that is not a map is rejected there,
with the reason on screen. Deciding "this is a valid map" from a filename would be the second
implementation of a format rule that `docs/03` §1.1 forbids.

The installation is found in this order: what the user configured, then `InstallPath` in
`HKLM\SOFTWARE\WOW6432Node\Blizzard Entertainment\Warcraft III`, then a handful of conventional
directories. The three are reported as `configured`, `registry` or `common` rather than collapsed
into "found", because a path the user chose and a path the app guessed are not the same fact.

⚠️ A configured path is **never silently replaced** by a detected one. If the user's path does not
hold `War3.exe`, the panel says which of the three things is wrong with it — missing, a file, or a
directory without the game — instead of quietly using a different installation. The check runs on
**save**, not on read, so a wrong path is reported while the box it was typed into is still on
screen.

## Layout

```text
w3wright-world-editor/
├── Cargo.toml            # workspace of one (see the comment there — it is load-bearing)
├── .cargo/config.toml    # dev-only: patches war3-* to the local checkout
├── index.html            # Vite entry
├── pages/                # one file per screen; the route table is generated from it
├── src/                  # Vue 3 + TypeScript front end
│   ├── nav.ts            # the menu: the one table behind the menu bar, and the navigation
│   ├── types.ts          # mirrors src-tauri/src/dto.rs and the command DTOs by hand
│   ├── format.ts         # byte sizes, file names
│   ├── colour.ts         # map text colours, solved for WCAG AA against the current surface
│   ├── components/       # the shell (TitleBar, MenuBar) and one panel per view
│   └── composables/      # useOpenMap (the open map), useSettings (the game), useWindow
├── src-tauri/            # the Rust app: Tauri config, commands, DTOs (see its README)
│   ├── src/dto.rs        # the read-only views' shapes
│   ├── src/settings.rs   # ~/.w3wright/settings.json, and finding the game
│   └── src/game.rs       # the Maps tree, and starting the game
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
| `war3-core` | `0.0.5` | `0.0.5` | transitively |
| `war3-archive` | `0.0.5` | `0.0.5` | the real member list |
| `war3-terrain` | `0.0.5` | `0.0.5` | the terrain view |
| `war3-meta` | `0.0.5` | `0.0.5` | transitively, through `war3-object` |
| `war3-map` | `0.0.2` | `0.0.5` | the map model |
| `war3-game` | **not published** | `0.0.5` | the game's own data and the name resolver |
| `war3-object` | **not published** | `0.0.5` | the object view |
| `war3-project` | **not published** | `0.0.5` | member dispositions |

Only the crates actually used are declared. Seven went in when this app was
scaffolded and it used none of them, which is a compile cost and a claim about the
design that was not true.

**The blocker is the version, not the missing crates.** Cargo treats `0.0.x` as
exact, so a requirement of `"0.0.5"` is **not** satisfied by a published `0.0.2`.
Four of the eight are published at `0.0.5` and resolve from crates.io on their own;
`war3-map` is still two versions behind, and `war3-game`, `war3-object` and
`war3-project` have **nothing** published at all, so those features have no release
path whatsoever yet. Today only a build with the local patch works. See the release
checklist.

⚠️ A transitive crate has to be patched too, even when no code here names it.
`war3-object` depends on `war3-meta` by version, so leaving `war3-meta` out of
`.cargo/config.toml` fails the build on a crate that cannot be resolved at all — one
level further out than the version mismatch above, and with the same cause.

### Release checklist

1. In the `w3wright` checkout, bump `[workspace.package] version` and the
   `version` keys in `[workspace.dependencies]` together, then run
   `cargo package --workspace` from a **fresh** target directory to verify.
2. Publish the crates this app uses. `war3-project`, `war3-object` and `war3-meta` are
   **not on crates.io at all** — member dispositions and the object view depend on
   them, so those features have no release path until they are published. Publish them
   before claiming a release of this app builds.
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
