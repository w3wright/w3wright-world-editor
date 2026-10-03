//! The game an editor is for: the file browser over its `Maps` directory, and launching it.
//!
//! # Why the app has an opinion about a game installation
//!
//! Everything the editor shows comes from `crates/` (see `docs/03` §1.1), but three things are
//! not questions about a map's bytes and so have no home in the core:
//!
//! - **Where the game is.** A directory on this machine, told to us by the user or the registry.
//! - **What maps are installed.** A directory listing, not a format — the *file* is still parsed
//!   by the core once it is opened, and a file that is not a map is rejected there.
//! - **How to start the game on a map.** A command line.
//!
//! ⚠️ One thing that *looks* like the second is the core's: **which file names count as a map**.
//! That list used to live in this file. It is now `war3_map::is_map_file_name`, and the browser
//! still reports "offered" rather than "valid" — see [`list_maps`] and the core function's own
//! documentation for why those are different questions.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::settings::{self, GameDir};

/// How deep the browser will walk.
///
/// A guard rather than a feature: `Maps` is 2 levels deep on the machine this was built against,
/// and a directory symlink pointing at an ancestor would otherwise recurse until the stack ran
/// out. The limit is well above any real layout, so it never shows up as a truncated list — it
/// shows up as a directory that stops expanding, which is the safe half of that trade.
const MAX_DEPTH: usize = 8;

/// One entry in the browser: a directory or a map file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEntry {
    /// The file or directory name, as on disk.
    pub name: String,
    /// The full path, which is what opening it needs.
    pub path: String,
    /// `dir` or `file`.
    pub kind: String,
    /// For a file, its size in bytes; `null` for a directory.
    pub size: Option<u64>,
}

/// One directory and everything under it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapDir {
    /// The directory's name; empty for the root, which is labelled by the caller.
    pub name: String,
    /// The directory's full path.
    pub path: String,
    /// Subdirectories, by name.
    pub dirs: Vec<MapDir>,
    /// Map files directly in this directory, by name.
    pub files: Vec<MapEntry>,
}

/// The whole browser payload: the game the tree came from, and the tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBrowser {
    /// The game directory the tree was read from.
    pub game_dir: String,
    /// The `Maps` directory, or `null` when the installation has none.
    pub maps_dir: Option<String>,
    /// The tree, or `null` when there is no `Maps` directory to walk.
    pub tree: Option<MapDir>,
    /// How many map files were found in total.
    pub map_count: usize,
}

/// Lists the maps installed under `<war3Dir>/Maps`.
///
/// # Errors
///
/// When no game installation can be found at all, or when `Maps` cannot be read. Both are states
/// the user can act on — the first by configuring a directory — so the message names what to fix.
pub fn list_maps() -> Result<MapBrowser, String> {
    let settings = settings::load()?;
    let game = settings::find_game(&settings).ok_or_else(|| {
        "no Warcraft III installation was found — set one in Settings, or open a map from disk"
            .to_string()
    })?;
    let maps_dir = PathBuf::from(&game.dir).join("Maps");
    if !maps_dir.is_dir() {
        // A real installation always has one; a directory holding only `War3.exe` is a partial
        // copy, and saying so beats showing an empty browser.
        return Ok(MapBrowser {
            game_dir: game.dir,
            maps_dir: None,
            tree: None,
            map_count: 0,
        });
    }

    let mut count = 0;
    let tree = walk(&maps_dir, "", 0, &mut count)?;
    Ok(MapBrowser {
        game_dir: game.dir,
        maps_dir: Some(maps_dir.display().to_string()),
        tree: Some(tree),
        map_count: count,
    })
}

/// Walks one directory.
///
/// Directories first then files, each alphabetically, so the same installation always produces
/// the same list — a browser that reorders itself between opens is one nobody can use twice.
fn walk(dir: &Path, name: &str, depth: usize, count: &mut usize) -> Result<MapDir, String> {
    let mut node = MapDir {
        name: name.to_string(),
        path: dir.display().to_string(),
        dirs: Vec::new(),
        files: Vec::new(),
    };
    if depth >= MAX_DEPTH {
        return Ok(node);
    }

    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;

    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf, u64)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        // A directory that cannot be stat'ed is skipped rather than failing the whole walk: one
        // unreadable folder should not hide the rest of an installation.
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            dirs.push((file_name, path));
        } else if war3_map::is_map_file_name(&file_name) {
            files.push((file_name, path, meta.len()));
        }
    }

    // `sort_by_cached_key` rather than `sort_by_key`: the key allocates a lowercased copy, and a
    // cached sort computes it once per entry instead of once per comparison.
    dirs.sort_by_cached_key(|(name, _)| name.to_lowercase());
    files.sort_by_cached_key(|(name, _, _)| name.to_lowercase());

    for (child_name, child_path) in dirs {
        let child = walk(&child_path, &child_name, depth + 1, count)?;
        node.dirs.push(child);
    }
    for (file_name, file_path, size) in files {
        *count += 1;
        node.files.push(MapEntry {
            name: file_name,
            path: file_path.display().to_string(),
            kind: "file".to_string(),
            size: Some(size),
        });
    }
    Ok(node)
}

/// Starts the game on a map.
///
/// # Why `-loadfile` and not "open with"
///
/// Verified against this installation's `War3.exe`: the binary carries the switch table
/// `.d3d.opengl.swtnl.classic.datadir.loadfile.gametype.fullscreen.window.`, and starting
/// `War3.exe -loadfile <map>` produces a running game. That is a direct observation rather than a
/// convention copied from a forum.
///
/// # Why the working directory is set
///
/// The game resolves its archives (`war3.mpq`, `War3Patch.mpq`) relative to itself. Launching it
/// with an inherited working directory — which for a Tauri app is wherever the app was started
/// from — is how "the game starts and then cannot find its own data" happens.
///
/// # Errors
///
/// When no installation is found, when the map is not there, or when the game cannot be spawned.
pub fn launch(map: &str) -> Result<Launch, String> {
    let settings = settings::load()?;
    let game = settings::find_game(&settings).ok_or_else(|| {
        "no Warcraft III installation was found — set one in Settings".to_string()
    })?;
    launch_with(&game, map)
}

/// Starts the game from an installation the caller already has.
fn launch_with(game: &GameDir, map: &str) -> Result<Launch, String> {
    let map_path = Path::new(map);
    if !map_path.is_file() {
        return Err(format!("{map} is not there any more"));
    }
    let child = std::process::Command::new(&game.exe)
        .arg("-loadfile")
        .arg(map_path)
        .current_dir(&game.dir)
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", game.exe))?;

    Ok(Launch {
        pid: child.id(),
        exe: game.exe.clone(),
        map: map.to_string(),
    })
}

/// What a successful launch produced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Launch {
    /// The process id, so the interface can say something happened rather than nothing.
    pub pid: u32,
    /// The executable that was started.
    pub exe: String,
    /// The map it was started on.
    pub map: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ⚠️ The extension test moved to the core with the function it tests:
    // `war3_map::map::tests::map_file_names_are_recognised_regardless_of_case`. Testing it here
    // would be testing a copy, which is the thing moving it was meant to end.

    /// Walking a real directory has to be stable: the same input, the same order.
    #[test]
    fn the_walk_is_sorted_and_counts_files() {
        let root = std::env::temp_dir().join("w3wright-walk-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("zeta")).unwrap();
        std::fs::create_dir_all(root.join("alpha")).unwrap();
        std::fs::write(root.join("b.w3x"), b"x").unwrap();
        std::fs::write(root.join("a.w3m"), b"x").unwrap();
        std::fs::write(root.join("notes.txt"), b"x").unwrap();
        std::fs::write(root.join("alpha").join("deep.W3X"), b"x").unwrap();

        let mut count = 0;
        let tree = walk(&root, "", 0, &mut count).unwrap();

        assert_eq!(count, 3, "two at the root and one nested");
        assert_eq!(
            tree.dirs
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"],
            "directories sort case-insensitively"
        );
        assert_eq!(
            tree.files
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a.w3m", "b.w3x"],
            "only map files are listed, sorted"
        );
        assert_eq!(tree.dirs[0].files.len(), 1, "the nested map is under alpha");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The depth guard stops a walk rather than overflowing the stack.
    #[test]
    fn the_depth_guard_stops_rather_than_failing() {
        let root = std::env::temp_dir().join("w3wright-depth-test");
        let _ = std::fs::remove_dir_all(&root);
        let mut deep = root.clone();
        for i in 0..MAX_DEPTH + 3 {
            deep = deep.join(format!("d{i}"));
        }
        std::fs::create_dir_all(&deep).unwrap();

        let mut count = 0;
        // Bound but not inspected: the assertion is that the walk *returns* on a deep tree
        // instead of overflowing the stack. Pinning where it stopped would make the guard harder
        // to tune later without saying anything more about correctness.
        let _tree = walk(&root, "", 0, &mut count).expect("a deep tree is walked, not an error");
        assert_eq!(count, 0, "no map files were planted");
        let _ = std::fs::remove_dir_all(&root);
    }
}
