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
//! One distinction the string form has to carry explicitly is that **"this does not
//! apply to your map" is not "something went wrong"**. [`NOT_APPLICABLE`] marks it.

use crate::dto::{DiagnosticView, MapSummary, SavePreview, TerrainInfo, TerrainView};

/// Prefix marking a message that means "this does not apply here", not "something
/// went wrong".
///
/// # Why a marker rather than a typed error
///
/// Tauri serialises an `Err` from a command as whatever the error type serialises to,
/// and a plain string is the one shape the front end can show without knowing the
/// type. A typed enum would be cleaner in Rust and would force the front end to
/// destructure a tag it has no other use for.
///
/// What matters is not the mechanism but that the two cases stay distinguishable:
/// **the operation does not apply to this map** is not the user's problem to fix,
/// while **the file could not be read** is. A member whose key is derived from its
/// block offset cannot be relocated, so the core refuses to rebuild any archive
/// holding one — DotA's `(LISTFILE)` is such a member, and before this marker the panel
/// said "could not open that map", which was simply untrue.
pub const NOT_APPLICABLE: &str = "not-applicable: ";

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
/// A message for the interface to show. One starting with [`NOT_APPLICABLE`] means the
/// operation does not apply to this map rather than that anything failed.
#[tauri::command]
pub fn preview_save(path: String) -> Result<SavePreview, String> {
    let original = std::fs::read(&path).map_err(|e| e.to_string())?;
    let archive = war3_archive::Archive::open(&path).map_err(|e| e.to_string())?;

    // Rebuild every member verbatim, then compare. `add_raw` carries the stored
    // block through unchanged, so a difference here is a difference the builder
    // introduced rather than one the caller asked for.
    let mut builder = war3_archive::ArchiveBuilder::with_prefix(archive.prefix().to_vec())
        .map_err(|e| e.to_string())?;
    let mut member_count = 0usize;
    let mut names = archive.file_names();
    names.sort_unstable();
    for name in &names {
        let raw = archive.raw_member(name).map_err(|e| e.to_string())?;
        // The one failure that means "cannot", not "broken": a member that cannot be
        // relocated. Reported as a limitation of this operation on this map.
        builder
            .add_raw(raw)
            .map_err(|e| format!("{NOT_APPLICABLE}{e}"))?;
        member_count += 1;
    }
    let rebuilt = builder
        .to_bytes()
        .map_err(|e| format!("{NOT_APPLICABLE}{e}"))?;

    let first_difference = original
        .iter()
        .zip(rebuilt.iter())
        .position(|(a, b)| a != b)
        .map(|i| i as u64)
        .or_else(|| {
            // A common prefix with different lengths still differs, at the shorter
            // end. `None` here means byte-identical.
            (original.len() != rebuilt.len()).then_some(original.len().min(rebuilt.len()) as u64)
        });

    let note = match first_difference {
        None => "A rebuild of this map is byte identical, so saving would change only what you edit."
            .to_string(),
        Some(at) => format!(
            "A rebuild preserves every member's content but rewrites the archive: \
             {} bytes becomes {}, and the first difference is at offset {at}. Saving would \
             therefore write a new file rather than patch this one.",
            original.len(),
            rebuilt.len(),
        ),
    };

    Ok(SavePreview {
        path,
        original_bytes: original.len() as u64,
        rebuilt_bytes: rebuilt.len() as u64,
        member_count,
        first_difference,
        note,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A map that cannot be rebuilt must say so as a limitation, not as a failure.
    ///
    /// DotA's `(LISTFILE)` has `BLOCK_OFFSET_ADJUSTED_KEY`, so the core refuses to
    /// rebuild the archive. That is a correct refusal about the *operation*, and the
    /// panel must not report it the way it reports "the file could not be read".
    #[test]
    fn a_map_that_cannot_be_rebuilt_is_reported_as_not_applicable() {
        let Ok(path) = std::env::var("W3_TEST_MAP_UNCANNY") else {
            return;
        };
        match preview_save(path.clone()) {
            Ok(preview) => {
                // A rebuildable map is fine to point this at; then there is nothing to
                // check beyond the preview being coherent.
                assert!(preview.original_bytes > 0);
            }
            Err(message) => {
                assert!(
                    message.starts_with(NOT_APPLICABLE),
                    "a rebuild refusal must be marked as not-applicable, got: {message}"
                );
                assert!(
                    message.contains("offset"),
                    "the reason must name the cause, got: {message}"
                );
            }
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
}
