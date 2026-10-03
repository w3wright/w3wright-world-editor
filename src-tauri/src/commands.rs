//! Tauri commands: the interface's only way into the core.
//!
//! # Shape
//!
//! One command per question, and each returns everything that question needs. That is
//! deliberate rather than convenient:
//!
//! - The core parses a map in **one pass** (`war3_map::Map::from_source`), so splitting
//!   a result across several commands would mean re-reading the file per question or
//!   caching a `Map` between calls.
//! - A cached `Map` is what makes an editor show stale data after a save. Keeping the
//!   app stateless is cheaper than remembering to invalidate, and this app has no
//!   reason to be stateful yet.
//!
//! Terrain is its own command rather than part of the map summary, because it is the
//! largest thing a map holds and most screens never look at it. Paying for that parse
//! on every open would be the wrong default.
//!
//! # Errors
//!
//! Every command returns `Result<_, String>`. The interface shows the message verbatim,
//! so it has to be the sentence a user can act on — not a debug rendering. The core's
//! errors already read that way, so they are passed through rather than reworded;
//! rewording them here would mean two vocabularies for the same failure.
//!
//! ⚠️ **This file no longer marks "does not apply" with a string prefix.** It used to: the
//! operation that can be inapplicable is the rebuild preview, and the fact was in the error type
//! all along — `MpqError::MemberNotWritable`, which `add_raw` returns for a member whose key is
//! derived from its block offset (DotA's `(LISTFILE)` is one). Flattening that to
//! `"not-applicable: …"` meant the front end had to parse the marker back out to tell a limitation
//! from a fault. `MpqError::is_not_applicable` answers that in the core now, and the marker is
//! gone rather than kept beside it.

use crate::dto::{
    doodad_record, doodad_summary, object_category, unit_record, unit_summary, DiagnosticView,
    DoodadView, MapSummary, ObjectView, SavePreview, TerrainInfo, TerrainView, UnitView,
    DETAIL_LIMIT,
};
use war3_map::MapSource;

/// Flattens a core diagnostics collection for the interface.
///
/// One conversion rather than one per command: the core's `Display` forms are what the
/// command-line tool prints, so keeping them identical here is what lets a diagnostic
/// be compared by eye between the two views.
fn diagnostics_of(source: &war3_core::diag::Diagnostics) -> Vec<DiagnosticView> {
    source
        .items()
        .iter()
        .map(|d| DiagnosticView {
            severity: d.severity.to_string(),
            code: d.code.to_string(),
            message: d.message.clone(),
        })
        .collect()
}

/// Opens a map and returns everything the interface displays about it.
///
/// # Why this opens the file twice
///
/// Two questions are being asked, and one crate answers each:
///
/// - `war3-archive` enumerates the **real** members. Its `file_names` comes from the
///   archive's own name list, so it includes imports and the minimap.
/// - `war3-map` parses the map's **contents** — metadata, string table, terrain.
///
/// ⚠️ The obvious single answer is wrong: `Map::files` looks like a member list but is
/// the intersection of the archive with `KNOWN_MEMBER_NAMES`, a hardcoded probe list.
/// `(4)LostTemple.w3m` has 16 members and `Map::files` reports 15 — the minimap is
/// simply missing, with no diagnostic, because its name is not on the list. Showing 15
/// files where a map has 16 is worse than an error, so the members come from the
/// archive.
///
/// The cost is two opens of the same file. That is negligible next to parsing it, and
/// it keeps each crate doing the job it is built for rather than teaching `war3-map`
/// about member enumeration.
///
/// # Errors
///
/// The core's own message when the file cannot be read or is not a map.
#[tauri::command]
pub fn open_map(path: String) -> Result<MapSummary, String> {
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;
    let map = war3_map::Map::from_source(&archive).map_err(|e| e.to_string())?;
    Ok(MapSummary::from_parse(&path, &archive, &map))
}

/// Rebuilds a map in memory and reports exactly what saving it would change.
///
/// Writes nothing. See [`SavePreview`] for why the interface shows this instead of
/// offering a save button: a rebuild preserves every member's content but rewrites
/// the archive's structure, so a save is a whole-file operation and the user is
/// entitled to see that before it happens.
///
/// # Errors
///
/// A fault: the file is not an archive, a table is malformed, the disk failed. A map the *operation*
/// does not apply to is **not** an error — see [`SavePreview::not_applicable`], which is the shape
/// every other view command uses for the same distinction (`TerrainView::ok` and so on).
#[tauri::command]
pub fn preview_save(path: String) -> Result<SavePreview, String> {
    // ⚠️ The operation is the core's, and deliberately so: the command line's `war3 map rebuild`
    // performs the same rebuild and must describe it the same way. Doing it here meant two
    // implementations of "would this change the file", which are free to disagree about a map
    // without anyone noticing which was right — and the sentence shown to the user was written in
    // this file.
    match war3_archive::RebuildPreview::of_file(&path) {
        Ok(preview) => Ok(SavePreview {
            path,
            ok: true,
            not_applicable: None,
            original_bytes: preview.original_bytes,
            rebuilt_bytes: preview.rebuilt_bytes,
            member_count: preview.member_count,
            first_difference: preview.first_difference,
            note: preview.note(),
        }),
        // ⚠️ The distinction is read from the core's error type, not from its text. A member whose
        // key is derived from its block offset cannot be relocated, so *no* rebuild of this archive
        // could be written — that is a limit of the operation, and the panel says so. Everything
        // else is a fault and travels the error path.
        Err(e) if e.is_not_applicable() => Ok(SavePreview {
            path,
            ok: false,
            not_applicable: Some(e.to_string()),
            original_bytes: 0,
            rebuilt_bytes: 0,
            member_count: 0,
            first_difference: None,
            note: String::new(),
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Reads a map's terrain and returns it for display.
///
/// # Why "no terrain" is not an error
///
/// A map without `war3map.w3e` is a valid archive; the core models terrain as optional
/// for exactly that reason. Reporting it as a failed command would put "this map has no
/// terrain" and "this file is not a map" on the same footing, so it comes back as
/// `ok: false` with the reason.
///
/// # Errors
///
/// Only when the file cannot be read as an archive at all.
#[tauri::command]
pub fn read_terrain(path: String) -> Result<TerrainView, String> {
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;

    let bytes = match archive.read_file("war3map.w3e") {
        Ok(bytes) => bytes,
        Err(e) => {
            return Ok(TerrainView {
                path,
                ok: false,
                error: Some(format!("no readable war3map.w3e in this map: {e}")),
                info: None,
                diagnostics: Vec::new(),
            })
        }
    };

    let terrain = match war3_terrain::Terrain::parse(&bytes) {
        Ok(terrain) => terrain,
        Err(e) => {
            return Ok(TerrainView {
                path,
                ok: false,
                error: Some(e.to_string()),
                info: None,
                diagnostics: Vec::new(),
            })
        }
    };

    let (origin_x, origin_y) = terrain.world_origin();
    let (min_h, max_h) = terrain.world_height_range();
    let flags = terrain.flag_counts();
    let max_textures = terrain.version.max_textures() as usize;

    let info = TerrainInfo {
        version: terrain.version.number(),
        record_size: terrain.version.record_size() as u32,
        texture_bits: terrain.version.texture_bits(),
        max_textures: terrain.version.max_textures(),
        width: terrain.width,
        height: terrain.height,
        tile_width: terrain.tile_width(),
        tile_height: terrain.tile_height(),
        tile_count: terrain.tiles.len(),
        tileset: terrain.tileset.to_string(),
        custom_tileset: terrain.uses_custom_tileset,
        center_offset_x: terrain.center_offset_x,
        center_offset_y: terrain.center_offset_y,
        world_origin_x: origin_x,
        world_origin_y: origin_y,
        min_world_height: min_h,
        max_world_height: max_h,
        ground_texture_count: terrain.ground_textures.len(),
        cliff_texture_count: terrain.cliff_textures.len(),
        used_ground_textures: terrain.used_ground_textures(),
        // Only the addressable ones are listed: the file may carry more entries than
        // the version's index width can name, and showing those would suggest they are
        // selectable when the format cannot reference them.
        ground_texture_ids: terrain
            .ground_textures
            .iter()
            .take(max_textures)
            .map(ToString::to_string)
            .collect(),
        cliff_texture_ids: terrain
            .cliff_textures
            .iter()
            .take(max_textures)
            .map(ToString::to_string)
            .collect(),
        ramp_points: flags.ramp,
        blight_points: flags.blight,
        water_points: flags.water,
        boundary_points: flags.boundary,
        layer_histogram: terrain.layer_histogram(),
    };

    Ok(TerrainView {
        path,
        ok: true,
        error: None,
        info: Some(info),
        diagnostics: diagnostics_of(&terrain.diagnostics),
    })
}

/// Reads a map's placed units and items, aggregated the way `war3 map units` prints
/// them.
///
/// # Why this goes through `Map::from_source` rather than the raw member
///
/// The terrain view reads `war3map.w3e` itself, because terrain is a leaf format with
/// nothing in the map model around it. Units are not: `Map::from_source` already
/// parses `war3mapUnits.doo` and records *why* it could not, and reading the member
/// here instead would mean either a second copy of that reasoning or a panel that
/// says "no units" where the core has a sentence about a desynchronised record
/// layout. So this asks the map model and passes its answer on, which is also what
/// makes the numbers below the same numbers the command-line tool prints.
///
/// # Why "no units" is not an error
///
/// A map with no `war3mapUnits.doo` is a valid archive — `(4)LostTemple.w3m` and
/// `DotA_IMBA_3.83.w3x` are both real examples of the two cases, and they differ:
/// one has no such member at all, the other's member is there and readable but the
/// file is simply absent from that archive. Reporting either as a failed command
/// would put it on the same footing as "this file is not a map", so they come back
/// as `ok: false` with the core's reason.
///
/// # Errors
///
/// Only when the file cannot be read as an archive at all.
#[tauri::command]
pub fn read_units(path: String) -> Result<UnitView, String> {
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;
    let map = war3_map::Map::from_source(&archive).map_err(|e| e.to_string())?;

    let diagnostics = diagnostics_of(&map.diagnostics);
    let Some(units) = &map.units else {
        return Ok(UnitView {
            path,
            ok: false,
            reason: Some(missing_reason(
                &diagnostics,
                "war3mapUnits.doo",
                "the map has no war3mapUnits.doo",
            )),
            info: None,
            records: Vec::new(),
            diagnostics,
        });
    };

    Ok(UnitView {
        path,
        ok: true,
        reason: None,
        info: Some(unit_summary(units)),
        records: units
            .units
            .iter()
            .take(DETAIL_LIMIT)
            .map(unit_record)
            .collect(),
        diagnostics,
    })
}

/// Reads a map's placed doodads, aggregated the way `war3 map doodads` prints them.
///
/// See [`read_units`] for why this asks the map model instead of the raw member.
///
/// # Errors
///
/// Only when the file cannot be read as an archive at all.
#[tauri::command]
pub fn read_doodads(path: String) -> Result<DoodadView, String> {
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;
    let map = war3_map::Map::from_source(&archive).map_err(|e| e.to_string())?;

    let diagnostics = diagnostics_of(&map.diagnostics);
    let Some(doodads) = &map.doodads else {
        return Ok(DoodadView {
            path,
            ok: false,
            reason: Some(missing_reason(
                &diagnostics,
                "war3map.doo",
                "the map has no war3map.doo",
            )),
            info: None,
            records: Vec::new(),
            diagnostics,
        });
    };

    Ok(DoodadView {
        path,
        ok: true,
        reason: None,
        info: Some(doodad_summary(doodads)),
        records: doodads
            .doodads
            .iter()
            .take(DETAIL_LIMIT)
            .map(doodad_record)
            .collect(),
        diagnostics,
    })
}

/// Reads a map's object data: the seven `war3map.w3*` files it holds.
///
/// # Why this does not go through `Map::from_source`
///
/// The map model does not read object files — they are outside it, as the crate's own
/// module list shows. `war3 map objects` reads each member from the archive and parses
/// it with `war3_object`, and this does the same thing rather than teaching the map
/// model about a format it does not currently carry.
///
/// # Why the object crate is needed at all, and what it costs
///
/// ⚠ `war3-object` is **not published to crates.io**. This line only resolves because
/// `.cargo/config.toml` patches it (and its `war3-meta` dependency) at the local
/// checkout, so the app still cannot be built for release until the crate is
/// published. That is a real limit of the current dependency mode, not of the view:
/// the code is correct and the numbers are checked against the command line, and the
/// README's release checklist is where the fix is written down.
///
/// # Why field ids are not resolved
///
/// `war3 map objects` names a field's meaning only with `--game-dir`, because the
/// metadata tables live in the game's archives rather than the map. This app has no
/// game-directory setting, so ids and values are shown as stored — the same output
/// the command line produces without that flag, not a degraded version of it.
///
/// # Errors
///
/// Only when the file cannot be read as an archive at all. A member that fails to
/// parse becomes a diagnostic and the other categories still come back.
#[tauri::command]
pub fn read_objects(path: String) -> Result<ObjectView, String> {
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;

    let mut categories = Vec::new();
    let mut diagnostics = Vec::new();
    for kind in war3_object::ObjectKind::ALL {
        let Some(bytes) = archive.get(kind.map_file()) else {
            continue;
        };
        match war3_object::ObjectFile::parse(kind, &bytes) {
            Ok(file) => {
                diagnostics.extend(diagnostics_of(&file.diagnostics));
                categories.push(object_category(&file, &kind.to_string()));
            }
            // One unreadable category is not the whole view failing: the other six
            // are still worth showing, and the message names the file.
            Err(e) => diagnostics.push(DiagnosticView {
                severity: "error".to_string(),
                code: "object.file".to_string(),
                message: format!("{} failed to parse: {e}", kind.map_file()),
            }),
        }
    }

    Ok(ObjectView {
        path,
        categories,
        diagnostics,
    })
}

/// Why a `.doo` file's data is not available, preferring the core's own sentence.
///
/// The core records "war3mapUnits.doo failed to parse; units skipped: ..." when the
/// member exists but the record layout is one this build does not know, and that
/// sentence is the difference between a user checking their map and a user assuming
/// the feature is broken. Matching it by the file name is how `war3 map units` and
/// `war3 map doodads` find it, and this reports the same sentence they do — the
/// alternative, a second lookup, would let the panel and the command line give
/// different reasons for one absence.
///
/// ⚠ The match is on a substring of a display message, which is a weak coupling. It
/// is the same coupling the command line has, so the two agree; making it strong
/// would mean a diagnostic that names its subject as a field rather than in prose,
/// which is a change in the core. Writing that down here so the next reader knows the
/// weakness was seen rather than missed.
fn missing_reason(diagnostics: &[DiagnosticView], member: &str, fallback: &str) -> String {
    diagnostics
        .iter()
        .find(|d| d.message.contains(member))
        .map_or_else(|| fallback.to_string(), |d| d.message.clone())
}

/// Reports the backend's own version, as a bridge smoke test.
///
/// A greeting would only prove the handler runs. Returning the version means the
/// interface can show something checkable: if it disagrees with `package.json`,
/// the frontend and backend are not the pair that was built together.
///
/// It is deliberately **not** called `core_version`. This is the app's version;
/// the `war3-map` version the app linked with is not visible from here, and a
/// function whose name claims otherwise would be the kind of small lie that costs
/// an afternoon later. To check which core was linked, look at `Cargo.lock`: a
/// patched crate has no `source` field.
#[tauri::command]
pub fn backend_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Where the settings file lives and what is in it.
///
/// The path is returned even when nothing has been saved yet, because a user who wants the app
/// to remember a game directory should be able to see *which file* would hold it — the whole
/// point of keeping settings in `~/.w3wright` rather than a plugin's private store.
///
/// # Errors
///
/// When a settings file exists but cannot be parsed. A missing one is the defaults, and is not an
/// error: the app has to start on a machine nothing has been configured on.
#[tauri::command]
pub fn settings_get() -> Result<SettingsView, String> {
    let settings = crate::settings::load()?;
    Ok(settings_view(&settings))
}

/// Writes the settings, after checking the game directory if one was given.
///
/// # Errors
///
/// When `war3_dir` is given and is not a Warcraft III installation, or when the file cannot be
/// written. ⚠️ The check happens **here** rather than on read, so that a wrong path is reported
/// while the user is looking at the box they typed it in. Checking on read instead would let the
/// bad value be saved and only complain later, on a screen with no field to fix.
#[tauri::command]
pub fn settings_save(settings: SettingsView) -> Result<SettingsView, String> {
    let mut stored = crate::settings::Settings::default();
    let candidate = settings.war3_dir.map(|dir| dir.trim().to_string());
    match candidate.as_deref() {
        Some("") | None => {}
        Some(dir) => {
            crate::settings::check_war3_dir(dir)?;
            stored.war3_dir = Some(dir.to_string());
        }
    }
    crate::settings::save(&stored)?;
    Ok(settings_view(&stored))
}

/// The game the app would use, and where that answer came from.
///
/// `game` is `None` when nothing was found — a state the interface must show as "set one", not as
/// an error, since a machine without the game installed is a legitimate way to use a map editor.
///
/// # Errors
///
/// Only when the settings file itself is unreadable.
#[tauri::command]
pub fn war3_status() -> Result<StatusView, String> {
    let settings = crate::settings::load()?;
    Ok(status_view(&settings))
}

/// Checks one directory the user typed, without saving it.
///
/// Separate from [`settings_save`] so the interface can answer "is this the right folder?" as
/// they type, rather than only on submit. It returns the same [`GameView`] a successful save
/// would, so the panel shows one shape either way.
///
/// # Errors
///
/// When the directory is missing, is a file, or holds no `War3.exe`. The message names which.
#[tauri::command]
pub fn war3_check(dir: String) -> Result<GameView, String> {
    let found = crate::settings::check_war3_dir(&dir)?;
    Ok(game_view(&found))
}

/// Lists the maps installed under the game's `Maps` directory.
///
/// ⚠️ This reports what a **directory listing** offers, not what is a valid map. The extension
/// filter is the game's, and the core has the final say when one is opened — deciding "this is a
/// map" from a filename is exactly the second implementation of a format rule that `docs/03` §1.1
/// forbids.
///
/// # Errors
///
/// When no installation can be found, or when `Maps` cannot be read.
#[tauri::command]
pub fn maps_list() -> Result<crate::game::MapBrowser, String> {
    crate::game::list_maps()
}

/// Starts the game on one map.
///
/// Returns the process id rather than nothing, so the interface can say what happened. A command
/// that returns `Ok(())` after spawning leaves the panel with no way to distinguish "started" from
/// "the click did nothing".
///
/// # Errors
///
/// When no installation is found, when the map is gone, or when the game cannot be started.
#[tauri::command]
pub fn game_launch(map: String) -> Result<crate::game::Launch, String> {
    crate::game::launch(&map)
}

/// The settings as the interface sees them.
fn settings_view(settings: &crate::settings::Settings) -> SettingsView {
    SettingsView {
        war3_dir: settings.war3_dir.clone(),
        path: crate::settings::file_path().map(|p| p.display().to_string()),
    }
}

/// A game installation as the interface sees it.
fn game_view(found: &crate::settings::GameDir) -> GameView {
    GameView {
        dir: found.dir.clone(),
        exe: found.exe.clone(),
        // The core's own words for where it came from, passed through rather than reworded: the
        // three sources mean different things to a reader, and inventing a friendlier synonym
        // here would be a second vocabulary for the same fact.
        source: found.source.clone(),
        has_maps: found.has_maps,
    }
}

/// The game's state as the interface sees it.
fn status_view(settings: &crate::settings::Settings) -> StatusView {
    StatusView {
        game: crate::settings::find_game(settings).as_ref().map(game_view),
        settings_path: crate::settings::file_path().map(|p| p.display().to_string()),
    }
}

/// What `settings_get` and `settings_save` return.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// The configured game directory, if there is one.
    pub war3_dir: Option<String>,
    /// Where the settings are stored; `None` when the home directory could not be found.
    pub path: Option<String>,
}

/// What `war3_status` returns.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusView {
    /// The installation the app would use, if any was found.
    pub game: Option<GameView>,
    /// Where the settings are stored.
    pub settings_path: Option<String>,
}

/// One game installation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    /// The installation directory.
    pub dir: String,
    /// The executable inside it.
    pub exe: String,
    /// `configured`, `registry` or `common` — where the directory was found.
    pub source: String,
    /// Whether it has a `Maps` directory for the browser to show.
    pub has_maps: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    // The two caps are asserted against the constants the panel prints, so a
    // test that drifts from the code it checks fails rather than agreeing with
    // itself.
    use crate::dto::{LEADER_COUNT, MODIFICATION_LIMIT};

    /// Reads a map named by an environment variable, or `None` when it is unset.
    ///
    /// Every test that needs a real map goes through this, so none of them hardcodes an
    /// absolute path and all of them skip silently on a machine without the maps. The
    /// core has its own fixture-based tests; what these add is a check that the **DTO
    /// layer** carries the parse through — that the interface is not being handed empty
    /// strings and plausible-looking zeros.
    fn map_from_env(name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|path| !path.is_empty())
    }

    /// `preview_save` must report a real difference, not a reassuring zero.
    ///
    /// This is the test that keeps the panel truthful. If a rebuild ever becomes
    /// byte identical, this fails, and the correct response is to simplify the
    /// panel — not to relax the assertion. Conversely, a `first_difference` of
    /// `None` for a map that does not rebuild identically would mean the comparison
    /// is broken, which is the failure mode that matters: it would tell a user their
    /// file is untouched when it is about to be rewritten.
    ///
    /// Pointed at a real map through `W3_TEST_MAP`, because the property under test
    /// is a property of the real format.
    #[test]
    fn a_rebuild_of_a_real_map_is_reported_as_different() {
        let Ok(path) = std::env::var("W3_TEST_MAP") else {
            return;
        };
        let preview = preview_save(path.clone()).expect("the map named by W3_TEST_MAP");

        assert_eq!(preview.path, path);
        assert!(preview.member_count > 0, "a map has members");
        assert!(preview.original_bytes > 0);

        // The finding this whole panel exists for: a rebuild is not a no-op.
        assert!(
            preview.first_difference.is_some(),
            "a rebuild of {path} came back byte identical, which contradicts the \
             measurement this panel is built on — simplify the panel if that is real"
        );
        assert!(
            preview.note.contains("rewrites the archive"),
            "the note must say what the numbers mean: {}",
            preview.note
        );
    }

    /// A map whose rebuild does not apply must say so as a limitation, not as a failure.
    ///
    /// `W3_TEST_MAP_UNCANNY` is DotA, whose `(LISTFILE)` has `BLOCK_OFFSET_ADJUSTED_KEY`, so the
    /// core refuses to relocate it and *no* rebuild of that archive can be written. That is a
    /// correct refusal about the operation.
    ///
    /// ⚠️ **The command points this map at itself**, which is the case worth pinning: it does not
    /// matter whether the variable names a map that can be rebuilt, because both branches are
    /// checked and the classification itself is asserted through the core's own
    /// `MpqError::is_not_applicable` rather than through the message text.
    #[test]
    fn a_rebuild_that_does_not_apply_is_a_limitation_rather_than_a_failure() {
        let Ok(path) = std::env::var("W3_TEST_MAP_UNCANNY") else {
            return;
        };
        let preview = preview_save(path.clone()).expect("a limitation is not an error");
        if preview.ok {
            assert!(preview.original_bytes > 0);
            assert!(!preview.note.is_empty());
            assert!(preview.not_applicable.is_none());
        } else {
            let why = preview
                .not_applicable
                .expect("the reason travels with the flag");
            assert!(
                why.contains("offset"),
                "the reason must name the cause, got: {why}"
            );
            // And the same failure classified through the core, which is where the answer lives.
            let direct = war3_archive::RebuildPreview::of_file(&path).unwrap_err();
            assert!(
                direct.is_not_applicable(),
                "a rebuild refusal must classify as not-applicable, got: {direct}"
            );
        }
    }

    /// The terrain view must agree with `war3 map terrain` on the same map.
    ///
    /// The numbers below are that command's output for `(4)LostTemple.w3m`, copied
    /// deliberately: the point of this layer is that it reports what the core computed,
    /// so a disagreement means the DTO is deriving something on its own.
    ///
    /// Point `W3_TEST_MAP` at that map or one with the same layout; it is skipped when
    /// unset, like the other real-map tests here.
    #[test]
    fn the_terrain_view_reports_what_the_core_computed() {
        let Ok(path) = std::env::var("W3_TEST_MAP") else {
            return;
        };
        let view = read_terrain(path).expect("a readable map");

        assert!(view.ok, "terrain should be readable: {:?}", view.error);
        let info = view.info.expect("info when ok");

        // `war3 map terrain` on LostTemple prints exactly these.
        assert_eq!(info.version, 11);
        assert_eq!(info.record_size, 7);
        assert_eq!(info.texture_bits, 4);
        assert_eq!(info.max_textures, 16);
        assert_eq!(info.width, 161);
        assert_eq!(info.height, 161);
        assert_eq!(info.tile_width, 160);
        assert_eq!(info.tile_height, 160);
        assert_eq!(info.tile_count, 25921);
        assert!(info.custom_tileset);

        // The counts have to be internally consistent, which catches a field wired to
        // the wrong source even when the individual numbers look plausible.
        assert_eq!(
            info.ground_texture_count,
            info.ground_texture_ids.len(),
            "the addressable list must be the whole list for this version"
        );
        assert!(info.min_world_height <= info.max_world_height);
        assert_eq!(
            info.layer_histogram.iter().sum::<usize>(),
            info.tile_count,
            "every tile point lands in exactly one layer bucket"
        );
        assert!(
            info.used_ground_textures.len() <= info.ground_texture_count,
            "referenced textures cannot outnumber the ones the file lists"
        );
        // Ramp, blight, water and boundary are subsets of the same tile points.
        for count in [
            info.ramp_points,
            info.blight_points,
            info.water_points,
            info.boundary_points,
        ] {
            assert!(count <= info.tile_count, "{count} > {}", info.tile_count);
        }
    }

    /// The unit view must agree with `war3 map units` on the same map.
    ///
    /// The numbers are that command's output for `(4)LostTemple.w3m`, copied
    /// deliberately. This layer's whole job is to report what the core computed, so a
    /// disagreement here means the DTO is deriving a figure of its own — which is the
    /// line `docs/03` §1.1 draws and the reason the histogram lives in `dto.rs` rather
    /// than in the panel.
    ///
    /// Point `W3_TEST_MAP` at that map, or one with the same contents. Skipped when it
    /// is unset, like the other real-map tests here.
    #[test]
    fn the_unit_view_reports_what_the_core_computed() {
        let Some(path) = map_from_env("W3_TEST_MAP") else {
            return;
        };
        let view = read_units(path.clone()).expect("a readable map");

        assert_eq!(view.path, path);
        assert!(view.ok, "LostTemple has units: {:?}", view.reason);
        let info = view.info.as_ref().expect("info when ok");

        // `war3 map units` on LostTemple prints exactly these.
        assert_eq!(info.version, 7);
        assert_eq!(info.subversion, 9);
        assert_eq!(info.records, 121);
        assert_eq!(info.levelled, 0);
        assert_eq!(info.placed_items, 0);

        // ⚠ 22 distinct types, and the command line prints **12 rows** — the head it
        // shows is capped, so `most common types` looks like the whole histogram and is
        // not. That difference is exactly why the DTO carries both the head and the full
        // list: a panel built on the command's twelve rows would report 12.
        assert_eq!(info.types.distinct, 22);
        assert_eq!(info.types.total, 121);
        assert_eq!(info.types.leaders.len(), LEADER_COUNT);
        assert_eq!(info.types.leaders[0].key, "nftb");
        assert_eq!(info.types.leaders[0].count, 28);
        assert_eq!(info.types.leaders[1].key, "ngna");
        assert_eq!(info.types.leaders[1].count, 12);
        // A second type has the same count, which is what pins the tie-break: with the
        // keys unnamed, a `HashMap` order would put these two either way round.
        assert_eq!(info.types.leaders[2].key, "ngol");
        assert_eq!(info.types.leaders[2].count, 12);

        assert_eq!(
            info.players
                .iter()
                .map(|p| (p.key.as_str(), p.count))
                .collect::<Vec<_>>(),
            vec![
                ("player 0", 1),
                ("player 1", 1),
                ("player 2", 1),
                ("player 3", 1),
                ("player 12", 100),
                ("player 15", 17),
            ]
        );

        // The detail list is the whole file here, and says so: 121 records against a cap
        // of 200, so nothing is missing and the panel must not claim otherwise. The
        // doodad tests are where the cap actually bites.
        assert_eq!(info.shown, info.records);
        assert_eq!(view.records.len(), 121);
        assert!(
            usize::try_from(info.records).expect("a count") < DETAIL_LIMIT,
            "this test is only meaningful while the whole file fits under the cap"
        );
        // `war3 map units`'s first record line, field for field where it prints them.
        assert_eq!(view.records[0].kind, "sloc");
        assert_eq!(view.records[0].player, 0);
        assert_eq!(view.records[0].hit_points, 0);
        assert_eq!(view.records[0].mana, 0);
        assert_eq!(view.records[0].hero_level, 0);
        // Version 7 has no item-table field at all, so this must be `None` rather than a
        // `-1` this build invented to fill the gap.
        assert_eq!(view.records[0].item_table, None);

        consistent(&view);
        // LostTemple's `.doo` files close exactly, so any diagnostic here is news.
        assert!(
            view.diagnostics.is_empty(),
            "LostTemple's units parse cleanly: {:?}",
            view.diagnostics
        );
    }

    /// The doodad view must agree with `war3 map doodads` on the same map.
    ///
    /// ⚠ The cap in this test earns its place: **5,317 doodads against 121 units** on
    /// the same map is the measurement the whole aggregation design rests on. It is also
    /// the assertion that would have caught the obvious first implementation, which was
    /// to serialise the record list.
    #[test]
    fn the_doodad_view_reports_what_the_core_computed() {
        let Some(path) = map_from_env("W3_TEST_MAP_DOODADS") else {
            return;
        };
        let view = read_doodads(path.clone()).expect("a readable map");

        assert_eq!(view.path, path);
        assert!(view.ok, "expects a map with doodads: {:?}", view.reason);
        let info = view.info.as_ref().expect("info when ok");

        // `war3 map doodads` on DotA_IMBA_3.83.w3x prints exactly these.
        assert_eq!(info.version, 7);
        assert_eq!(info.subversion, 11);
        assert_eq!(info.records, 5273);
        assert_eq!(info.types.distinct, 58);
        // No sample here carries the special block, and the panel has to say zero rather
        // than showing an empty section that reads as unread.
        assert_eq!(info.special, 0);
        assert_eq!(info.special_version, 0);

        // ⚠ Nothing beyond the head is pinned for this map: 58 distinct types against
        // a 12-entry head means the tail below is not fixed by the command's output. What
        // must hold is that the head is the top of the *full* histogram, which
        // `consistent_doodads` checks — and that is the assertion that catches a head
        // sorted by the wrong key.
        assert_eq!(info.types.leaders[0].key, "ATtr");
        assert_eq!(info.types.leaders[0].count, 2107);
        assert_eq!(info.types.leaders[1].key, "NTtw");
        assert_eq!(info.types.leaders[1].count, 1626);

        // The cap, on a map where it is the difference between a list and a payload:
        // 5,273 records become 200. `war3 map doodads`'s own first line for this map,
        // field for field where it prints them.
        assert!(info.records > u32::try_from(DETAIL_LIMIT).expect("a small constant"));
        assert_eq!(
            info.shown,
            u32::try_from(DETAIL_LIMIT).expect("a small constant")
        );
        assert_eq!(view.records.len(), DETAIL_LIMIT);
        assert_eq!(view.records[0].kind, "ATtr");
        assert_eq!(view.records[0].variation, 2);
        assert_eq!(view.records[0].flags, 2);
        // `flags_name` comes from the core, so a match in this layer would show up as an
        // empty or wrong name rather than as a type error.
        assert_eq!(view.records[0].flags_name, "visible solid");
        assert_eq!(view.records[0].life, 100);
        // A 255% life record, which is past a `u8` percentage of a percentage: the field
        // is not clamped anywhere, and clamping it here would be a claim about the format.
        assert_eq!(view.records[2].kind, "LOsk");
        assert_eq!(view.records[2].life, 255);

        consistent_doodads(&view);
    }

    /// The two doodad fields that are easy to get wrong are both worth a case.
    ///
    /// `life` is a stored byte, not a percentage this layer computes, and the flags byte
    /// has a value — 3 — that the core deliberately names `unknown`. Both are visible in
    /// `war3 map doodads`'s first page for `(4)LostTemple.w3m`, and both would look
    /// plausible if the record layout slipped by a field.
    #[test]
    fn the_doodad_view_carries_life_and_flags_as_stored() {
        let Some(path) = map_from_env("W3_TEST_MAP") else {
            return;
        };
        let view = read_doodads(path).expect("a readable map");
        let info = view.info.as_ref().expect("a map with doodads");

        assert_eq!(info.version, 7);
        assert_eq!(info.subversion, 9);
        assert_eq!(info.records, 5317);
        assert_eq!(info.types.distinct, 20);
        assert_eq!(info.types.leaders[0].key, "LTlt");
        assert_eq!(info.types.leaders[0].count, 4318);

        // Record 12 prints `0%` on the command line and record 13 prints `unknown`.
        assert_eq!(view.records[12].kind, "LTlt");
        assert_eq!(view.records[12].life, 0);
        assert_eq!(view.records[12].flags, 2);
        assert_eq!(view.records[12].flags_name, "visible solid");
        assert_eq!(view.records[13].flags, 3);
        assert_eq!(view.records[13].flags_name, "unknown");

        consistent_doodads(&view);
    }

    /// The doodad view must match on a map that is three versions away in layout.
    ///
    /// `(4)LostTemple.w3m` is v7/sub9 and `DotA_IMBA_3.83.w3x` is v7/sub11; this one is
    /// v8/sub11, whose records carry the item fields. Counting its type histogram is the
    /// check that the version gate did not quietly change what is counted.
    #[test]
    fn the_doodad_view_agrees_with_the_command_line_on_a_version_8_map() {
        let Some(path) = map_from_env("W3_TEST_MAP_OBJECTS") else {
            return;
        };
        let view = read_doodads(path).expect("a readable map");
        let info = view.info.as_ref().expect("a map with doodads");

        assert_eq!(info.version, 8);
        assert_eq!(info.subversion, 11);
        // `war3 map doodads` prints 4,616 and 23 for this map.
        assert_eq!(info.records, 4616);
        assert_eq!(info.types.distinct, 23);
        assert_eq!(info.types.leaders[0].key, "KTtw");
        assert_eq!(info.types.leaders[0].count, 1950);

        consistent_doodads(&view);
    }

    /// The unit view must match on a map whose records carry the hero fields.
    ///
    /// Version 8 adds the item-table pointer and the hero block, so this is the test that
    /// would notice the version gate being read from the wrong field: the count and the
    /// histograms would still look plausible, and the record walk would not.
    #[test]
    fn the_unit_view_agrees_with_the_command_line_on_a_version_8_map() {
        let Some(path) = map_from_env("W3_TEST_MAP_OBJECTS") else {
            return;
        };
        let view = read_units(path).expect("a readable map");
        let info = view.info.as_ref().expect("a map with units");

        assert_eq!(info.version, 8);
        assert_eq!(info.subversion, 11);
        // `war3 map units` prints exactly these for `(6)BlizzardTD.w3x`.
        assert_eq!(info.records, 207);
        assert_eq!(info.types.total, 207);
        assert_eq!(info.types.leaders[0].key, "o01G");
        assert_eq!(info.types.leaders[0].count, 66);
        assert_eq!(info.types.leaders[1].key, "sloc");
        assert_eq!(info.types.leaders[1].count, 8);
        assert_eq!(info.levelled, 0);
        // ⚠ Not "every version 8 unit has an item table": the field is always present
        // at v8, so counting *presence* would report 207 here. The command line reports 0,
        // and so must this.
        assert_eq!(info.placed_items, 0);
        // The version 8 path is the one that reaches `item_table`, and the first record is
        // a v8 hero unit whose pointer is unwritten. `war3 map units`'s first line for
        // this map is `hC06 3 (-320.4, -4481.7, 0.0) def def 1`.
        assert_eq!(view.records[0].kind, "hC06");
        assert_eq!(view.records[0].player, 3);
        assert_eq!(view.records[0].hit_points, -1);
        assert_eq!(view.records[0].item_table, Some(-1));
        // ⚠ `distinct` is 86 while the command line prints 12 rows — see the note in
        // the LostTemple test. It is the number that proves the head is a head.
        assert_eq!(info.types.distinct, 86);

        assert_eq!(
            info.players
                .iter()
                .map(|p| (p.key.as_str(), p.count))
                .collect::<Vec<_>>(),
            vec![
                ("player 0", 6),
                ("player 1", 6),
                ("player 2", 6),
                ("player 3", 6),
                ("player 4", 6),
                ("player 5", 6),
                ("player 8", 1),
                ("player 9", 96),
                ("player 15", 74),
            ]
        );

        consistent(&view);
    }

    /// A map with no `war3mapUnits.doo` must say so as a state, not as a failure.
    ///
    /// DotA_IMBA_3.83.w3x has no units member at all, and the core records that as an
    /// absence rather than an error. The panel distinguishes "this map has none" from
    /// "this file could not be read", and this pins which side of that line an absent
    /// member falls on.
    #[test]
    fn a_map_without_units_reports_the_absence_rather_than_failing() {
        let Some(path) = map_from_env("W3_TEST_MAP_UNCANNY") else {
            return;
        };
        let view = read_units(path).expect("the archive is still readable");

        assert!(!view.ok);
        assert!(view.info.is_none());
        assert!(view.records.is_empty());
        let reason = view.reason.expect("an absence has a reason");
        // Either sentence is correct and they say different things: the member is not in
        // the archive, or it is there and its layout is one this build refuses to guess
        // at. Both are reported by the same command line, in the same words.
        assert!(
            reason == "the map has no war3mapUnits.doo" || reason.contains("failed to parse"),
            "the reason must be the core's own sentence, got: {reason}"
        );
    }

    /// The object view must agree with `war3 map objects` on the same map.
    ///
    /// ⚠ This test is the reason `war3-object` is a dependency at all. It also pins the
    /// one thing the view cannot do: `war3 map objects` resolves field names when it is
    /// given `--game-dir`, and this app has no game directory, so the ids are shown as
    /// stored. The numbers below are that command's output **without** the flag — the
    /// same output a user without an installation sees.
    #[test]
    fn the_object_view_reports_what_the_core_computed() {
        let Some(path) = map_from_env("W3_TEST_MAP_OBJECTS") else {
            return;
        };
        let view = read_objects(path.clone()).expect("a readable map");

        assert_eq!(view.path, path);
        // All seven categories, in the core's own order, for this map.
        let counted: Vec<(String, String, u32, u32)> = view
            .categories
            .iter()
            .map(|c| (c.map_file.clone(), c.kind.clone(), c.original, c.custom))
            .collect();
        assert_eq!(
            counted,
            vec![
                ("war3map.w3u".to_string(), "unit".to_string(), 30, 187),
                ("war3map.w3t".to_string(), "item".to_string(), 0, 57),
                ("war3map.w3b".to_string(), "destructable".to_string(), 10, 1),
                ("war3map.w3d".to_string(), "doodad".to_string(), 1, 1),
                ("war3map.w3a".to_string(), "ability".to_string(), 13, 96),
                ("war3map.w3h".to_string(), "buff".to_string(), 2, 6),
                ("war3map.w3q".to_string(), "upgrade".to_string(), 0, 24),
            ]
        );
        for category in &view.categories {
            assert_eq!(category.version, 2, "{}", category.map_file);
        }

        // The two tables are printed separately by the command line, so the caps are per
        // table and a map with more than 200 of one still lists the other.
        let units = &view.categories[0];
        assert_eq!(units.custom_objects.len(), 187);
        assert_eq!(units.modified_objects.len(), 30);
        assert_eq!(units.shown, 217);
        for entry in units.custom_objects.iter().chain(&units.modified_objects) {
            assert_eq!(
                entry.shown.len(),
                usize::try_from(entry.modifications)
                    .expect("a count")
                    .min(MODIFICATION_LIMIT),
                "{} shows the head of its modifications",
                entry.id
            );
            for m in &entry.shown {
                assert_eq!(m.field.len(), 4, "a field id is four characters: {m:?}");
            }
        }
        // A modified original's base is the object itself, so it has none to name.
        assert!(units.custom_objects.iter().all(|o| !o.base_id.is_empty()));
        assert!(units.modified_objects.iter().all(|o| o.base_id.is_empty()));
        assert_eq!(view.categories.len(), 7);
    }

    /// A map with no object files reports that, rather than seven empty sections.
    #[test]
    fn a_map_without_object_files_reports_an_empty_category_list() {
        let Some(path) = map_from_env("W3_TEST_MAP") else {
            return;
        };
        let view = read_objects(path).expect("a readable map");

        assert!(
            view.categories.is_empty(),
            "(4)LostTemple.w3m has no object files, got {:?}",
            view.categories
                .iter()
                .map(|c| c.map_file.clone())
                .collect::<Vec<_>>()
        );
    }

    /// The wire names must be the ones `src/types.ts` declares.
    ///
    /// # Why this is worth a test of its own
    ///
    /// `types.ts` mirrors these DTOs **by hand** — see its module comment for why it is
    /// not generated. The only other feedback loop is the app at runtime, where a renamed
    /// field arrives as `undefined` and renders as an empty cell rather than as an error,
    /// and the busiest screens are the ones nobody loads by hand.
    ///
    /// Two spellings are deliberate and both are asserted here: the top-level and record
    /// fields are `camelCase`, from `#[serde(rename_all = "camelCase")]`, while
    /// [`crate::dto::TypeCount`] and [`crate::dto::TypeSummary`] have no `rename_all` —
    /// `key`/`count` staying as they are is not an oversight, they are the field names the
    /// format, the command line and the panel all use.
    ///
    /// Pointed at a map with all three kinds of data, so the records and the histograms
    /// are both non-empty. Skipped when the variable is unset.
    #[test]
    fn the_wire_names_are_the_ones_the_interface_declares() {
        let Some(path) = map_from_env("W3_TEST_MAP_OBJECTS") else {
            return;
        };

        let view = read_units(path.clone()).expect("a readable map");
        let json = serde_json::to_value(&view).expect("a serialisable view");
        assert_keys(
            &json,
            &["path", "ok", "reason", "info", "records", "diagnostics"],
        );
        let info = &json["info"];
        assert_keys(
            info,
            &[
                "version",
                "subversion",
                "records",
                "levelled",
                "placedItems",
                "shown",
                "types",
                "players",
            ],
        );
        assert_keys(&info["types"], &["distinct", "total", "counts", "leaders"]);
        assert_keys(&info["types"]["leaders"][0], &["key", "count"]);
        assert_keys(&info["players"][0], &["key", "count"]);
        assert_keys(
            &json["records"][0],
            &[
                "kind",
                "variation",
                "x",
                "y",
                "z",
                "rotation",
                "player",
                "hitPoints",
                "mana",
                "itemTable",
                "dropSets",
                "inventory",
                "abilities",
                "gold",
                "targetAcquisition",
                "heroLevel",
                "creationNumber",
            ],
        );

        let view = read_doodads(path.clone()).expect("a readable map");
        let json = serde_json::to_value(&view).expect("a serialisable view");
        assert_keys(
            &json,
            &["path", "ok", "reason", "info", "records", "diagnostics"],
        );
        assert_keys(
            &json["info"],
            &[
                "version",
                "subversion",
                "specialVersion",
                "records",
                "special",
                "specialKinds",
                "shown",
                "types",
            ],
        );
        assert_keys(
            &json["records"][0],
            &[
                "kind",
                "variation",
                "x",
                "y",
                "z",
                "rotation",
                "flags",
                "flagsName",
                "life",
                "itemTable",
                "itemSets",
                "editorId",
            ],
        );

        let view = read_objects(path).expect("a readable map");
        let json = serde_json::to_value(&view).expect("a serialisable view");
        assert_keys(&json, &["path", "categories", "diagnostics"]);
        let category = &json["categories"][0];
        assert_keys(
            category,
            &[
                "mapFile",
                "kind",
                "version",
                "original",
                "custom",
                "shown",
                "customObjects",
                "modifiedObjects",
            ],
        );
        let entry = &category["customObjects"][0];
        assert_keys(entry, &["id", "baseId", "hero", "modifications", "shown"]);
        assert_keys(
            &entry["shown"][0],
            &["field", "value", "level", "dataIndicator"],
        );
    }

    /// Asserts that an object's keys are exactly the expected set.
    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let object = value.as_object().unwrap_or_else(|| {
            panic!("expected a JSON object, got {value}");
        });
        let mut actual: Vec<&str> = object.keys().map(String::as_str).collect();
        actual.sort_unstable();
        let mut expected: Vec<&str> = expected.to_vec();
        expected.sort_unstable();
        assert_eq!(
            actual, expected,
            "the wire names differ from `src/types.ts`; update both together"
        );
    }

    /// Counts that all come from one parse must agree with each other.
    ///
    /// An assertion of this shape is worth more than another pinned number: it is what
    /// catches a field wired to the wrong source, where every individual value looks
    /// plausible. The sums here are the ones the command line's own output implies.
    fn consistent(view: &UnitView) {
        let info = view.info.as_ref().expect("info when consistent");
        let shown = u32::try_from(DETAIL_LIMIT).expect("a small constant");

        // The histogram covers every record, and the head is its own top.
        assert_eq!(
            info.types.total, info.records,
            "types must cover every record"
        );
        assert_eq!(
            info.types.distinct,
            u32::try_from(info.types.counts.len()).expect("a count")
        );
        assert_eq!(
            info.players.iter().map(|p| p.count).sum::<u32>(),
            info.records,
            "every record belongs to exactly one player"
        );
        assert!(
            info.types.leaders.len() <= LEADER_COUNT,
            "the head is capped"
        );
        assert!(
            info.types
                .leaders
                .iter()
                .zip(&info.types.counts)
                .all(|(a, b)| a.key == b.key && a.count == b.count),
            "the head must be a prefix of the ranked list"
        );
        for pair in info.types.counts.windows(2) {
            assert!(
                pair[0].count > pair[1].count
                    || (pair[0].count == pair[1].count && pair[0].key < pair[1].key),
                "the ranking is descending, ties by key: {:?}",
                pair
            );
        }

        // A detail list longer than the cap, or shorter than the cap on a map that has
        // more records, would mean the truncation is somewhere else than it claims.
        assert_eq!(info.shown, info.records.min(shown));
        assert_eq!(
            u32::try_from(view.records.len()).expect("a count"),
            info.shown
        );
        assert!(info.levelled <= info.records);
        assert!(info.placed_items <= info.records);
    }

    /// The doodad equivalent of [`consistent`].
    fn consistent_doodads(view: &DoodadView) {
        let info = view.info.as_ref().expect("info when consistent");
        let shown = u32::try_from(DETAIL_LIMIT).expect("a small constant");

        assert_eq!(
            info.types.total, info.records,
            "types must cover every record"
        );
        assert_eq!(
            info.types.distinct,
            u32::try_from(info.types.counts.len()).expect("a count")
        );
        assert_eq!(
            info.special_kinds.iter().map(|k| k.count).sum::<u32>(),
            info.special,
            "the special histogram is the special block"
        );
        assert_eq!(info.shown, info.records.min(shown));
        assert_eq!(
            u32::try_from(view.records.len()).expect("a count"),
            info.shown
        );
        // Every doodad record's flag name has to be one the core's table produces. This is
        // the assertion that fails if `flags_name` is ever replaced by a match in this
        // layer that misses a value.
        for record in &view.records {
            assert!(
                [
                    "invisible non-solid",
                    "visible non-solid",
                    "visible solid",
                    "unknown"
                ]
                .contains(&record.flags_name.as_str()),
                "flags {} produced {:?}, which is not one of the core's names",
                record.flags,
                record.flags_name
            );
        }
    }
}
