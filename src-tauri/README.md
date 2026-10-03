# The Tauri side

Two things here are not obvious and both cost real debugging time. They are written down
because neither is discoverable from the code that depends on them.

## The window draws its own title bar

`tauri.conf.json` sets `"decorations": false`, so there is no native title bar, no system
menu, and no platform close button. Everything a title bar does is reimplemented:

| What | Where | Permission it needs |
| --- | --- | --- |
| Dragging the window | `data-tauri-drag-region` on the bar and its parts | `core:window:allow-start-dragging` |
| Double-click to maximize | Tauri's own drag-region handling | `core:window:allow-internal-toggle-maximize` (in `core:window:default`) |
| Minimize | `TitleBar.vue` ▸ `useWindow.ts` | `core:window:allow-minimize` |
| Maximize / restore | `TitleBar.vue` ▸ `useWindow.ts` | `core:window:allow-toggle-maximize` |
| Close | `TitleBar.vue` ▸ `useWindow.ts` | `core:window:allow-close` |
| Reading the maximized state | `refreshMaximized`, on every resize | `core:window:allow-is-maximized` (in `core:window:default`) |

### ⚠️ `toggle_maximize` is its own permission

`window.toggleMaximize()` is **not** `maximize` + `unmaximize`. Granting those two leaves it
denied, and the failure is a rejected promise that `useWindow.ts` logs as one line — "could
not toggle maximize the window: window.toggle_maximize not allowed. Permissions associated
with this command: core:window:allow-toggle-maximize". The button then does nothing, which
reads as a broken button rather than as a missing permission. If a window control ever stops
working, read that log line before touching the component.

## ⚠️ `"windows": ["*"]`, not `["main"]`

The window's label **is** `main` — it is set explicitly in `tauri.conf.json`, and the
capability is the only one in the project. `"windows": ["main"]` nevertheless resolves to a
command whose window set is empty, and every command is then denied with:

```text
… not allowed on window "main", webview "main", URL: local
allowed on: [URL: local]
referenced by: capability: default, permission: allow-open
```

`[URL: local]` with no window list is the tell: the origin matched and the window did not.
Measured on Tauri 2.12.1 / tauri-utils 2.10.1 by running both forms against the same
binary — with `["main"]` both the file dialog and `minimize` failed, with `["*"]` both
worked. The mechanism inside `tauri-utils`' ACL resolution was not found; what is recorded
here is the measurement, so the next person does not repeat it.

This app has exactly one window, so `"*"` grants nothing that `"main"` would not. If a
second window is ever added, revisit this rather than assuming the wildcard is still
harmless.
