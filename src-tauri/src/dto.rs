//! The serialisable boundary between the core crates and the interface.
//!
//! # Why this layer exists
//!
//! The `war3-*` crates are deliberately dependency-free: no `serde`, no
//! serialisation formats, nothing that would stop them reaching `wasm32` or that
//! would have to be carried by every consumer. That rule is what keeps the core
//! usable from a browser runtime later.
//!
//! So the interface cannot send a `Map` across the Tauri bridge. It sends these
//! types instead, and **this is the only place where the two vocabularies meet**.
//!
//! # What belongs here, and what does not
//!
//! A field belongs here when the interface actually renders it. Everything else
//! stays in the core: a DTO that mirrors a whole `MapInfo` would be a second copy
//! of the format to keep in step, and the copies drift. When the interface needs
//! another field, it is added here in the same change that needs it.
//!
//! No format logic lives here either. Deciding what a byte means, resolving a
//! `TRIGSTR_` reference, working out a size — all of that is the core's job, and
//! the DTO only carries the answer. See `docs/03` §1.1: the interface must not
//! reimplement format logic.

use serde::Serialize;
use war3_core::diag::Diagnostics;


/// Everything the interface shows about one open map.
///
/// One command returns all of it rather than the interface asking for pieces. The
/// data comes from a single parse, so splitting it into several commands would
/// mean either re-reading the file per question or keeping the `Map` alive in
/// state — and the second is what makes an editor's caches go stale.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSummary {
    /// The path that was opened, as given.
    pub path: String,
    /// Map name, with any `TRIGSTR_` reference already resolved by the core.
    pub name: String,
    /// Author, likewise resolved.
    pub author: String,
    /// Description, likewise resolved.
    pub description: String,
    /// Recommended players, `None` when the format version has no such field.
    pub recommended_players: Option<String>,
    /// `.w3i` format version.
    pub format_version: i32,
    /// Tileset as a name, e.g. `Lordaeron Summer`.
    pub tileset: Option<String>,
    /// Playable size in tiles.
    pub playable_width: i32,
    /// Playable size in tiles.
    pub playable_height: i32,
    /// The raw flags word; the interface shows it, it does not interpret it.
    pub flags: u32,
    /// How many members the archive holds.
    pub member_count: usize,
    /// Total size of the members in bytes.
    pub total_member_bytes: u64,
    /// How many entries the string table has, after resolution.
    pub string_count: usize,
    /// Members, sorted by name so the list is stable between runs.
    pub members: Vec<Member>,
    /// Whether `extract` would accept this map, and why not when it would not.
    ///
    /// A map can be perfectly readable and still not extractable: if the archive holds
    /// more blocks than it has names for, extracting would silently drop members, and
    /// the core refuses rather than doing that. The interface says so instead of
    /// offering an action that cannot work.
    pub extractable: Extractability,
    /// Everything the parse could not read, in the order it was found.
    pub diagnostics: Vec<DiagnosticView>,
}

/// Whether the map can be turned into a source project.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Extractability {
    /// `true` when the core would accept it.
    pub ok: bool,
    /// Blocks the archive holds that are in use.
    pub used_blocks: usize,
    /// Blocks with a recovered name.
    pub named_blocks: usize,
    /// Why it would be refused, in the core's own words, or `None` when it would not.
    ///
    /// Passed through rather than reworded: it is the same sentence the command-line
    /// tool prints for the same condition, and the workaround (`--drop-unnamed`) is
    /// named in it.
    pub reason: Option<String>,
}

/// One archive member, as the interface lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    /// Member name.
    pub name: String,
    /// Size in bytes.
    pub size: u32,
    /// Where this member would go if the map were extracted.
    ///
    /// `text`, `binary` or `raw`. Taken from [`war3_project::disposition`], which is
    /// the same answer `extract` acts on — so the list cannot claim a member will be
    /// textified while extraction writes it as binary.
    pub disposition: String,
    /// Why, when the disposition is not `text`. Empty otherwise.
    pub disposition_reason: String,
}

/// What a save **would** do, without doing it.
///
/// # Why this exists instead of a save button
///
/// Saving an edited map means rebuilding the archive, and a rebuild is not a
/// minimal change. Measured on `(4)LostTemple.w3m`: the archive goes from 245,236
/// bytes to 244,201 and diverges from offset 520 onward — the hash slots and block
/// table are laid out again.
///
/// So "change the map name and save" would rewrite the user's whole file. That is
/// not something to do quietly, and until there is an in-place patch path in the
/// core, the honest interface is one that **shows what would happen**. See
/// `docs/04` Q20's appended finding.
///
/// ⚠️ Everything here is a **measurement of the two byte strings**, not an
/// assertion about member content. An earlier version of this type carried
/// `members_preserved: true` and an `identical_members` count, both of which were
/// copies of the loop counter — they said nothing and would have read as verified
/// facts. Member-level equality is checked by the core's own rebuild tests, which
/// run the comparison; this type reports only what it can see.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePreview {
    /// The map that was examined.
    pub path: String,
    /// Size of the original file in bytes.
    pub original_bytes: u64,
    /// Size of the rebuilt archive in bytes.
    pub rebuilt_bytes: u64,
    /// How many members were carried into the rebuild.
    pub member_count: usize,
    /// Byte offset of the first difference between the original and the rebuild, or
    /// `None` when the two are byte identical.
    pub first_difference: Option<u64>,
    /// What the interface should tell the user, in one sentence.
    ///
    /// Composed here rather than in the front end because it states a fact about the
    /// *format*, and the interface is not allowed to know those. See `docs/03` §1.1.
    pub note: String,
}

/// A map's terrain, in the form the interface shows it.
///
/// Everything here is a value the core already computed, including the derived ones:
/// `minWorldHeight` comes from `Terrain::world_height_range`, not from walking the
/// tiles here. Deriving anything in this layer would be a second implementation of the
/// format, which is the line `docs/03` §1.1 draws.
///
/// Terrain is optional on a map, and "this map has no terrain" is not an error — an
/// archive with no `.w3e` is still a valid map. So this carries `ok` and `error`
/// instead of the command failing, and the panel shows the reason.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainView {
    /// The map this was read from.
    pub path: String,
    /// `false` when the map has no readable terrain; `error` then says why.
    pub ok: bool,
    /// Why the terrain could not be read, when it could not.
    pub error: Option<String>,
    /// The numbers, present only when `ok`.
    pub info: Option<TerrainInfo>,
    /// Diagnostics from the terrain parse.
    pub diagnostics: Vec<DiagnosticView>,
}

/// Terrain measurements, all as the core reports them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainInfo {
    /// On-disk `.w3e` version.
    ///
    /// `i32`, matching the core's own type. Format versions are signed there so an
    /// unrecognised one can be reported rather than wrapped into a plausible small
    /// number, and narrowing it here would undo that.
    pub version: i32,
    /// Bytes per tile point for that version.
    pub record_size: u32,
    /// Ground texture index width in bits.
    pub texture_bits: u32,
    /// How many textures that width can address.
    pub max_textures: u32,
    /// Tile points along X.
    pub width: u32,
    /// Tile points along Y.
    pub height: u32,
    /// Playable tiles along X, which is tile points minus one.
    pub tile_width: u32,
    /// Playable tiles along Y.
    pub tile_height: u32,
    /// How many tile points parsed.
    pub tile_count: usize,
    /// Main tileset, as the core's own name.
    pub tileset: String,
    /// Whether a custom or mixed tileset is in use.
    pub custom_tileset: bool,
    /// Centre offset along X, in world units.
    pub center_offset_x: f32,
    /// Centre offset along Y, in world units.
    pub center_offset_y: f32,
    /// South-west corner of the map in world units, X.
    pub world_origin_x: f32,
    /// South-west corner of the map in world units, Y.
    pub world_origin_y: f32,
    /// Lowest terrain height in world units.
    pub min_world_height: f32,
    /// Highest terrain height in world units.
    pub max_world_height: f32,
    /// Ground texture count.
    pub ground_texture_count: usize,
    /// Cliff texture count.
    pub cliff_texture_count: usize,
    /// The ground texture indices the map actually references.
    pub used_ground_textures: Vec<u8>,
    /// The addressable ground textures, in file order, as four-character codes.
    pub ground_texture_ids: Vec<String>,
    /// The addressable cliff textures, likewise.
    pub cliff_texture_ids: Vec<String>,
    /// Tile points with the ramp flag.
    pub ramp_points: usize,
    /// Tile points with the blight flag.
    pub blight_points: usize,
    /// Tile points with the water plane enabled.
    pub water_points: usize,
    /// Tile points with the camera boundary flag.
    pub boundary_points: usize,
    /// How many tile points use each ground texture index, 0 through 15.
    pub layer_histogram: [usize; 16],
}
/// One diagnostic, flattened for display.
///
/// `severity` and `code` are strings rather than the core's enums: they are shown,
/// not matched on, and the core's own `Display` forms are what the command-line
/// tool prints, so the two stay comparable by eye.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticView {
    /// `info`, `warning` or `error`, lowercased.
    pub severity: String,
    /// Stable code, e.g. `wts.missing-key`.
    pub code: String,
    /// Human-readable message, including the numbers involved.
    pub message: String,
}

impl MapSummary {
    /// Builds the summary from a parsed map **and** the archive it came from.
    ///
    /// Both are needed, and the archive is not redundant. See
    /// [`crate::commands::open_map`]: `Map::files` is not the member list.
    ///
    /// Members are sorted here rather than in the core: the core's order is the
    /// archive's, which is meaningful to it and not to a list view.
    #[must_use]
    pub fn from_parse(
        path: &str,
        archive: &war3_archive::Archive,
        map: &war3_map::Map,
    ) -> Self {
        let mut members: Vec<Member> = archive
            .file_names()
            .into_iter()
            .filter_map(|name| {
                // The size has to come from the block entry: a member can be listed
                // but unreadable (an unsupported codec, for instance), and dropping
                // it from the list would hide that it exists.
                let block = archive
                    .block_index_for_name(name)
                    .and_then(|i| archive.block_entry(i as usize).copied())?;
                // The disposition comes from the core, and its diagnostics are
                // swallowed here on purpose: this is a *preview* of what extraction
                // would decide, and `extract` records the same reasons when it really
                // runs. Collecting them twice would double every warning in the panel.
                let mut scratch = Diagnostics::new();
                let (disposition, reason) = match war3_project::disposition::of(
                    archive,
                    name,
                    &mut scratch,
                ) {
                    Ok(war3_project::Disposition::Textified { .. }) => ("text".to_string(), String::new()),
                    Ok(war3_project::Disposition::KeptBinary { reason }) => {
                        ("binary".to_string(), reason)
                    }
                    Ok(war3_project::Disposition::KeptRaw) => (
                        "raw".to_string(),
                        "this workspace cannot decode it, so its stored block is kept".to_string(),
                    ),
                    Ok(war3_project::Disposition::Absent) => ("absent".to_string(), String::new()),
                    // `of` only errors when the container is inconsistent, which the
                    // summary is not the place to fail over: the member still exists
                    // and still has a size.
                    Err(e) => ("binary".to_string(), e.to_string()),
                };
                Some(Member {
                    name: name.to_owned(),
                    size: block.uncompressed_size,
                    disposition,
                    disposition_reason: strip_member_prefix(&reason, name),
                })
            })
            .collect();
        members.sort_by(|a, b| a.name.cmp(&b.name));

        // Summed here because it is a display figure: the list shows individual
        // sizes and the header shows their total.
        let total_member_bytes: u64 = members.iter().map(|m| u64::from(m.size)).sum();

        // The same condition `extract` refuses on, reported so the interface can say
        // so up front rather than offering an action that will fail.
        let used_blocks = archive.used_block_count();
        let named_blocks = archive.file_count();
        let extractable = Extractability {
            ok: named_blocks >= used_blocks,
            used_blocks,
            named_blocks,
            reason: (named_blocks < used_blocks).then(|| {
                format!(
                    "{used_blocks} blocks are in use but only {named_blocks} names could be \
                     recovered, so extracting would drop {} member(s)",
                    used_blocks - named_blocks
                )
            }),
        };

        // Two diagnostic streams, both worth showing: the archive's ("the MPQ header
        // is not at offset 0", "N blocks have no name") and the map's (missing
        // `TRIGSTR_` keys, unreadable terrain). Concatenated in that order because
        // the archive's are about the container and come logically first.
        let diagnostics = archive
            .diagnostics()
            .items()
            .iter()
            .chain(map.diagnostics.items().iter())
            .map(|d| DiagnosticView {
                // The core's own forms, not a second spelling: the command-line tool
                // prints these, so keeping them identical is what lets a diagnostic
                // be compared by eye between the two.
                severity: d.severity.to_string(),
                code: d.code.to_string(),
                message: d.message.clone(),
            })
            .collect();

        Self {
            path: path.to_owned(),
            name: map.metadata.name.clone(),
            author: map.metadata.author.clone(),
            description: map.metadata.description.clone(),
            recommended_players: map.metadata.recommended_players.clone(),
            format_version: map.metadata.format_version,
            // The core's `Display`, not `name()`: the command-line tool prints this
            // form, so the two screens agree character for character. Using `name()`
            // was a small divergence — "Lordaeron Summer" against the CLI's
            // "Lordaeron Summer (L)" — and a field that reads differently in two
            // places invites someone to "fix" one of them.
            tileset: map.metadata.tileset.map(|t| t.to_string()),
            playable_width: map.metadata.playable_width,
            playable_height: map.metadata.playable_height,
            flags: map.metadata.flags.0,
            member_count: members.len(),
            total_member_bytes,
            string_count: map.strings.len(),
            members,
            extractable,
            diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    /// Checks the DTO against a real map, when one is pointed at.
    ///
    /// `W3_TEST_MAP` names a `.w3x`/`.w3m` outside this repository, so there is no
    /// absolute path in the source and no fixture checked in. Skipped silently when
    /// it is unset, which is the honest behaviour: the core already has its own map
    /// tests, and what this one adds is a check that the **DTO** carries the parse
    /// through — that the interface is not being handed empty strings.
    ///
    /// It is worth running after touching this file:
    ///
    /// ```text
    /// W3_TEST_MAP='D:\Warcraft3\Maps\(4)LostTemple.w3m' cargo test -p w3wright-world-editor
    /// ```
    #[test]
    fn a_real_map_round_trips_through_the_summary() {
        let Ok(path) = std::env::var("W3_TEST_MAP") else {
            return;
        };
        let archive = war3_archive::Archive::open(&path).expect("the map named by W3_TEST_MAP");
        let map = war3_map::Map::from_source(&archive).expect("a parseable map");
        let summary = MapSummary::from_parse(&path, &archive, &map);

        // The resolved name is the point of this DTO: a map whose name is a
        // TRIGSTR_ reference must arrive already resolved, because the interface has
        // no way to resolve it and must not learn one.
        assert!(
            !summary.name.is_empty(),
            "the name must survive the conversion"
        );
        assert!(
            !summary.name.starts_with("TRIGSTR_"),
            "the core resolves references, so the DTO must carry the resolved text"
        );
        assert_eq!(summary.member_count, summary.members.len());

        // Every member must carry a disposition, and it must come from the core rather
        // than default to empty: the list is the Explorer's whole answer to "where does
        // this member go", so a blank one is a silent hole.
        for member in &summary.members {
            assert!(
                ["text", "binary", "raw", "absent"].contains(&member.disposition.as_str()),
                "member {} has disposition {:?}",
                member.name,
                member.disposition
            );
            assert_eq!(
                member.disposition == "text",
                member.disposition_reason.is_empty(),
                "only a textified member has no reason to give: {}",
                member.name
            );
        }
        // ⚠️ No assertion that some member is textified. That was tried and is wrong:
        // a map with a stripped `(listfile)` exposes only metadata members, none of
        // which has a text form, so "at least one text" fails on real maps while the
        // classifier is working correctly. What must hold is that every member was
        // classified at all, which the loop above checks.
        let textified = summary
            .members
            .iter()
            .filter(|m| m.disposition == "text")
            .count();
        eprintln!(
            "dispositions: {} of {} members textified",
            textified,
            summary.members.len()
        );

        // The extractability verdict is the same comparison `extract` refuses on.
        eprintln!(
            "EXTRACTABLE ok={} used={} named={} reason={:?}",
            summary.extractable.ok,
            summary.extractable.used_blocks,
            summary.extractable.named_blocks,
            summary.extractable.reason
        );
        assert_eq!(summary.extractable.named_blocks, archive.file_names().len());
        assert_eq!(summary.extractable.ok, summary.extractable.reason.is_none());
        assert_eq!(summary.path, path);

        // ⚠️ Not `map.files`: that is the intersection with `KNOWN_MEMBER_NAMES`, and
        // this assertion used to compare against it — which made the test agree with
        // the bug. The member list must come from the archive, and for a real map the
        // two differ (LostTemple: 16 members versus 15), so this is the assertion that
        // pins the fix.
        assert_eq!(summary.members.len(), archive.file_names().len());
        assert!(
            summary.members.len() >= map.files.len(),
            "the archive lists at least what the probe list found"
        );

        // Members are sorted for a stable list, and every one carries its size.
        let mut sorted = summary.members.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        assert!(
            summary.members.iter().map(|m| &m.name).eq(sorted.iter().map(|m| &m.name)),
            "members must be in a stable order"
        );
    }
}

/// Removes a leading `name: ` from a core diagnostic, for display beside the name.
///
/// The core's messages are written to stand alone on one line of `war3 map list`, and
/// that is exactly right there — the reader has no other column telling them which
/// member the line is about. In a table that *does* have a name column, the prefix is
/// noise: "(ATTRIBUTES)" printed twice in one row.
///
/// Only the exact `name: ` form is removed, and only from the front, so a message that
/// happens to contain the name elsewhere is left alone. The core's wording is not
/// otherwise touched: it is still the sentence the command-line tool prints, which is
/// what lets a user compare the two.
#[must_use]
fn strip_member_prefix(reason: &str, name: &str) -> String {
    reason
        .strip_prefix(name)
        .and_then(|rest| rest.strip_prefix(": "))
        .unwrap_or(reason)
        .to_owned()
}
    /// The name is removed from a diagnostic shown beside the name, and nothing else is.
    #[test]
    fn the_member_name_is_stripped_only_as_a_prefix() {
        assert_eq!(
            strip_member_prefix("WAR3MAP.WTS: no text form", "WAR3MAP.WTS"),
            "no text form"
        );
        // A name that merely appears later must be left alone: the sentence is the
        // core's, and rewriting the middle of it would make the two views disagree.
        assert_eq!(
            strip_member_prefix("kept as binary: WAR3MAP.WTS could not be parsed", "WAR3MAP.WTS"),
            "kept as binary: WAR3MAP.WTS could not be parsed"
        );
        // A colon that is not the separator.
        assert_eq!(strip_member_prefix("WAR3MAP.WTS:", "WAR3MAP.WTS"), "WAR3MAP.WTS:");
        // No name at all.
        assert_eq!(
            strip_member_prefix("this workspace cannot decode it", "WAR3MAP.WTS"),
            "this workspace cannot decode it"
        );
    }