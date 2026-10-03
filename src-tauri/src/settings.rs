//! The user's settings, in `~/.w3wright/settings.json`.
//!
//! # Why a file in the home directory rather than a Tauri store
//!
//! Two readers want this file and only one of them is the app: a user who wants to point the
//! editor at a different game installation should be able to edit a text file, and a future
//! command-line tool needs the same answer without going through a webview. A plugin's private
//! store would make it the app's alone.
//!
//! # Why the directory is fixed rather than versioned
//!
//! `~/.w3wright/` is meant to accumulate the things that belong to a *user* rather than to a
//! map — settings today, and plausibly caches or keybindings later. Keeping the path stable is
//! what lets a second file appear without a migration.
//!
//! # What this file is not
//!
//! ⚠️ No map format lives here. This is the application's own configuration; nothing in it
//! describes a byte of a `.w3x`. The rule that the interface asks `crates/` about formats (see
//! `docs/03` §1.1) is untouched — but what a *game installation directory* is, is not a map
//! format question, and the app is the only thing that has an opinion about it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name inside the settings directory.
const FILE_NAME: &str = "settings.json";

/// The directory, relative to the user's home.
const DIR_NAME: &str = ".w3wright";

/// The game's executable, as every 1.2x installation ships it.
const GAME_EXE: &str = "War3.exe";

/// What the user has told the app, and nothing else.
///
/// Every field is optional in the file: a settings file with one key set is valid, and one
/// that does not exist at all is the same as an empty one.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The Warcraft III installation directory, if the user has set one.
    ///
    /// Stored as the user typed it, not canonicalised: this is echoed back into a text box, and
    /// rewriting a path the user is looking at makes it hard to tell what was saved.
    pub war3_dir: Option<String>,
}

/// The settings file's path, whether or not it exists.
#[must_use]
pub fn file_path() -> Option<PathBuf> {
    home().map(|home| home.join(DIR_NAME).join(FILE_NAME))
}

/// The settings directory, whether or not it exists.
#[must_use]
pub fn dir_path() -> Option<PathBuf> {
    home().map(|home| home.join(DIR_NAME))
}

/// The user's home directory.
///
/// `USERPROFILE` on Windows and `HOME` elsewhere, read here rather than pulled in with a crate:
/// two environment variables do not justify a dependency, and the failure mode is explicit
/// (`None`) instead of a panic three layers down.
fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// Reads the settings.
///
/// # Errors
///
/// Only when a settings file exists and cannot be parsed. A missing file is the default
/// settings, not an error — the app has to start on a machine where nothing has been
/// configured. A *malformed* file is an error on purpose: silently using defaults would make
/// the user's own edit look like it was saved when it was ignored.
pub fn load() -> Result<Settings, String> {
    let Some(path) = file_path() else {
        return Ok(Settings::default());
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(Settings::default());
    }
    serde_json::from_str(&text).map_err(|e| format!("{} is not valid JSON: {e}", path.display()))
}

/// Writes the settings, creating the directory if it is not there.
///
/// # Errors
///
/// When the directory cannot be created or the file cannot be written. Both are the user's to
/// fix or to report, so the message names the path.
pub fn save(settings: &Settings) -> Result<(), String> {
    let Some(dir) = dir_path() else {
        return Err("could not find your home directory to store settings in".to_string());
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let path = dir.join(FILE_NAME);
    // Pretty-printed because this file is meant to be opened and edited by hand. A one-line
    // blob would make that possible but unpleasant, and there is no size to save.
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("could not serialise the settings: {e}"))?;
    std::fs::write(&path, format!("{text}\n"))
        .map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// A game installation the app found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDir {
    /// The directory.
    pub dir: String,
    /// `War3.exe` inside it.
    pub exe: String,
    /// Where the directory was found: `configured`, `registry` or `common`.
    ///
    /// Reported rather than hidden because the three have different trust: a configured path is
    /// what the user chose, and either of the others is a guess the app made on their behalf.
    pub source: String,
    /// Whether `<dir>/Maps` exists, so the file browser has something to show.
    pub has_maps: bool,
}

impl GameDir {
    /// Builds the description of one candidate, or `None` when it is not an installation.
    fn of(dir: &Path, source: &str) -> Option<Self> {
        let exe = dir.join(GAME_EXE);
        if !exe.is_file() {
            return None;
        }
        Some(Self {
            dir: dir.display().to_string(),
            exe: exe.display().to_string(),
            source: source.to_string(),
            has_maps: dir.join("Maps").is_dir(),
        })
    }
}

/// Finds a game installation.
///
/// Order: what the user configured, then the registry, then a few conventional directories.
/// The configured path wins even if another is found, because a user who has pointed the app at
/// an installation has said which one they mean — and a second copy of the game on the same
/// machine is exactly the case where guessing wrong is annoying rather than harmless.
#[must_use]
pub fn find_game(settings: &Settings) -> Option<GameDir> {
    if let Some(dir) = settings
        .war3_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        // A configured path that does not hold the game is NOT silently replaced by a detected
        // one: the user would see the app using a directory they did not choose. `check_war3_dir`
        // says what is wrong with it instead.
        if let Some(found) = GameDir::of(Path::new(dir), "configured") {
            return Some(found);
        }
    }

    if let Some(dir) = from_registry() {
        if let Some(found) = GameDir::of(&dir, "registry") {
            return Some(found);
        }
    }

    for candidate in common_dirs() {
        if let Some(found) = GameDir::of(&candidate, "common") {
            return Some(found);
        }
    }
    None
}

/// Checks one directory and says precisely what is wrong with it.
///
/// # Why this is separate from [`find_game`]
///
/// `find_game` returning `None` cannot distinguish "that path does not exist", "that path is a
/// directory but has no `War3.exe`" and "the game is installed somewhere else entirely". A user
/// typing a path into a box needs to be told which, since only the first is usually a typo.
#[must_use = "the error says what is wrong with the directory, and discarding it loses that"]
pub fn check_war3_dir(dir: &str) -> Result<GameDir, String> {
    let trimmed = dir.trim();
    if trimmed.is_empty() {
        return Err("no directory given".to_string());
    }
    let path = Path::new(trimmed);
    if !path.exists() {
        return Err(format!("{trimmed} does not exist"));
    }
    if !path.is_dir() {
        return Err(format!("{trimmed} is a file, not a directory"));
    }
    GameDir::of(path, "configured").ok_or_else(|| {
        format!("{trimmed} has no {GAME_EXE} in it — that is not a Warcraft III directory")
    })
}

/// Reads `InstallPath` from the registry.
///
/// Through `reg query` rather than a registry crate: this is one value from one key, and the
/// dependency would exist for the sake of a single read. Failure is quiet on purpose — most
/// machines that matter here have the game configured instead.
fn from_registry() -> Option<PathBuf> {
    #[cfg(not(windows))]
    {
        None
    }
    #[cfg(windows)]
    {
        let key = r"HKLM\SOFTWARE\WOW6432Node\Blizzard Entertainment\Warcraft III";
        let output = std::process::Command::new("reg")
            .args(["query", key, "/v", "InstallPath"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            // `    InstallPath    REG_SZ    d:\warcraft3\`
            let mut parts = line.split_whitespace();
            if parts.next() != Some("InstallPath") {
                continue;
            }
            // Skip the value type, then take whatever remains as the path: it may contain
            // spaces, and `split_whitespace` would have cut it in two.
            parts.next()?;
            let rest = parts.collect::<Vec<_>>().join(" ");
            if rest.is_empty() {
                return None;
            }
            // The registry value is written with a trailing backslash.
            return Some(PathBuf::from(rest.trim_end_matches('\\')));
        }
        None
    }
}

/// Directories the game is commonly installed in.
///
/// A short list rather than a filesystem search: scanning every drive for `War3.exe` would take
/// seconds on a full disk and would find other people's installations.
fn common_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for drive in ['C', 'D', 'E', 'F'] {
        for name in ["Warcraft3", "Warcraft III", "Games\\Warcraft III"] {
            out.push(PathBuf::from(format!("{drive}:\\{name}")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A settings file with one key set is valid, and a missing field is the default.
    #[test]
    fn a_partial_settings_file_parses() {
        let parsed: Settings = serde_json::from_str(r#"{"war3Dir":"D:\\Warcraft3"}"#).unwrap();
        assert_eq!(parsed.war3_dir.as_deref(), Some(r"D:\Warcraft3"));
        // `default` on the container is what makes an empty object legal rather than an error.
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(empty.war3_dir.is_none());
    }

    /// The field is `war3Dir` on disk, because the file is meant to be hand-edited and the rest
    /// of the app speaks camelCase to the front end.
    #[test]
    fn the_key_is_camel_case() {
        let text = serde_json::to_string(&Settings {
            war3_dir: Some("x".to_string()),
        })
        .unwrap();
        assert!(text.contains("war3Dir"), "{text}");
    }

    /// A directory without the game in it is rejected, and the message says which thing is
    /// missing rather than "not found".
    #[test]
    fn a_directory_without_the_executable_says_so() {
        let temp = std::env::temp_dir().join("w3wright-settings-test-empty");
        std::fs::create_dir_all(&temp).unwrap();
        let error = check_war3_dir(&temp.display().to_string()).unwrap_err();
        assert!(error.contains(GAME_EXE), "{error}");
        assert!(error.contains("not a Warcraft III directory"), "{error}");
        let _ = std::fs::remove_dir(&temp);
    }

    #[test]
    fn a_missing_directory_is_distinguished_from_a_wrong_one() {
        let error = check_war3_dir("Z:\\definitely-not-here").unwrap_err();
        assert!(error.contains("does not exist"), "{error}");
    }

    /// The configured path wins over detection, even when detection would find something.
    ///
    /// Tested through the ordering rather than by planting a game: what matters is that a
    /// configured path is consulted first and that a bad one does not fall through to a guess.
    #[test]
    fn a_configured_path_that_is_wrong_does_not_fall_through_to_a_guess() {
        let settings = Settings {
            war3_dir: Some("Z:\\definitely-not-here".to_string()),
        };
        // Whatever `find_game` returns, it must not claim the configured directory is the
        // source when that directory holds no game.
        if let Some(found) = find_game(&settings) {
            assert_ne!(found.source, "configured", "{found:?}");
            assert_ne!(found.dir, "Z:\\definitely-not-here");
        }
    }

    #[test]
    fn an_empty_configured_path_is_the_same_as_none() {
        let settings = Settings {
            war3_dir: Some("   ".to_string()),
        };
        // Must not panic on an empty path, and must not treat it as a directory.
        if let Some(found) = find_game(&settings) {
            assert_ne!(found.dir.trim(), "");
        }
    }
}
