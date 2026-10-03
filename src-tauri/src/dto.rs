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
//!
//! # The one thing this layer does compute
//!
//! Histograms: how many units of each type, which types are commonest, how many records
//! each player owns. Those are aggregations over values the core already parsed, not
//! readings of bytes, and they are computed here rather than in the interface on purpose
//! \u2014 `war3 map units` prints the same lists, and two implementations of "most common
//! types" would be free to disagree about a map without anyone noticing which was right.
//!

use serde::Serialize;
use war3_core::diag::Diagnostics;

/// One run of text with an optional colour.
///
/// # Why the colour is three numbers and not a CSS string
///
/// The panel cannot draw the author's colour unchanged: `|cffffff00` is white, and white text
/// on this window's light background is invisible. The fix is the one `src/badges.ts` already
/// uses for the same problem — keep the hue and saturation, and let the stylesheet choose a
/// lightness that works in the current colour scheme. That needs the *components*, so sending
/// `#ffffff` would have thrown away the numbers the renderer needs.
///
/// ⚠️ There is no alpha. WC3 writes one and it is `0` on most real text, which is fully
/// transparent; the game ignores it, and `war3_map::Markup::colour` deliberately does not
/// return it so that no renderer can honour it by mistake.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RichSegment {
    /// The text of this run.
    pub text: String,
    /// The author's colour for it, or `None` for text left at the default.
    pub colour: Option<SegmentColour>,
}

/// A colour as stored in the map, before the renderer decides how light to draw it.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentColour {
    /// Red, 0–255.
    pub red: u8,
    /// Green, 0–255.
    pub green: u8,
    /// Blue, 0–255.
    pub blue: u8,
}

/// A map text field, split into the runs it is made of.
///
/// Always at least one segment: a field with no markup is one segment with no colour, so the
/// panel renders both cases through one path rather than branching on "is there any colour in
/// this string". An empty field is one empty segment, for the same reason.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RichText {
    /// The runs, in order.
    pub segments: Vec<RichSegment>,
}

impl RichText {
    /// Splits a core string field into the runs the author coloured.
    ///
    /// The decoding is `war3_map::spans`, which is the same reader `war3 map info` prints with —
    /// nothing here decides what a code means, it only reshapes what the core decided.
    #[must_use]
    pub fn from_core(s: &str) -> Self {
        let segments: Vec<RichSegment> = war3_map::spans(s)
            .into_iter()
            .map(|run| RichSegment {
                colour: run
                    .colour()
                    .map(|(red, green, blue)| SegmentColour { red, green, blue }),
                text: run.text().to_owned(),
            })
            .collect();

        // `spans` returns nothing for an empty string; the panel would then have no node to
        // render. One empty segment keeps "the field is empty" and "the field is missing" from
        // being the same shape in the template.
        if segments.is_empty() {
            return Self {
                segments: vec![RichSegment {
                    text: String::new(),
                    colour: None,
                }],
            };
        }
        Self { segments }
    }
}

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
    /// Map name with any `TRIGSTR_` reference already resolved by the core, and with the
    /// markup codes removed.
    ///
    /// This is the plain form, for the places that cannot show a colour — a window title, a
    /// `title` tooltip, a list of maps. [`MapSummary::name_rich`] carries the same text with
    /// the colours the author chose, and is what the panel draws.
    pub name: String,
    /// The author's name, plain.
    pub author: String,
    /// The description, plain.
    pub description: String,
    /// Recommended players, `None` when the format version has no such field.
    pub recommended_players: Option<String>,
    /// The map name split into the runs the author coloured.
    ///
    /// # Why both forms
    ///
    /// Warcraft III map titles carry inline colour codes, and `羊羊快跑4.34|CFF1FBF00最终正式版`
    /// is a real one: green for the second half, with no reset at the end. Showing the code is
    /// wrong, and showing the text with the code *removed* throws away the only part of the
    /// title the author deliberately marked. So both are sent: the plain string for labels, and
    /// this for the panel, which renders each run in its colour.
    pub name_rich: RichText,
    /// The author's name, split the same way.
    pub author_rich: RichText,
    /// The description, split the same way.
    pub description_rich: RichText,
    /// Recommended players, split the same way; `None` when there is no such field.
    pub recommended_players_rich: Option<RichText>,
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

/// One ID to look up.
///
/// `kind` uses the core's own spelling (`unit`, `item`, `ability`, …) so that a caller reads it from
/// the same vocabulary the panels and the command line use.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameQuery {
    /// Which table to consult.
    pub kind: String,
    /// The object's four-character id.
    pub id: String,
}

/// What one ID is called.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameAnswer {
    /// Echoed back, so a caller can match answers to a batch without relying on order.
    pub kind: String,
    /// Echoed back, for the same reason.
    pub id: String,
    /// The name, which is the id itself when nothing knows it.
    ///
    /// ⚠️ Markup is **not** stripped here: a name may carry `|cffffff00…|r`, and decoding that is
    /// `war3_map::plain`'s job — one function, used by the command line and the panels alike.
    pub name: String,
    /// `map`, `game` or `id` — the core's own label for where the name came from.
    pub source: String,
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
    /// Whether the question could be answered at all.
    ///
    /// ⚠️ `false` is **not** a failure: it means rebuilding does not apply to this archive, which is
    /// a property of the map rather than a fault. The other view commands carry the same field for
    /// the same reason (`TerrainView::ok`), so the interface has one shape to handle rather than a
    /// convention per command.
    pub ok: bool,
    /// Why the question does not apply, when it does not. `None` when `ok`.
    pub not_applicable: Option<String>,
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
    /// Composed by the core rather than here, because it states a fact about the *format* and it is
    /// the same sentence `war3 map rebuild` gives. See `docs/03` §1.1.
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

/// One placed unit or item, as the list shows it.
///
/// # Why this is not the core's `Unit`
///
/// The core's record has twenty-one fields and four nested tables. Sending all of them
/// would be a second copy of the format to keep in step, and most of them — the
/// random-unit payload, the ability modifications, the drop sets — are not on this
/// screen. What is here is what is rendered; the rest stays in the core until an editing
/// screen needs it, exactly as `docs/03` §1.1 requires.
///
/// Default fillers are reported rather than pre-resolved: `-1` hit points means "the
/// type's default", and looking that default up needs the object data, which is a
/// different question for a different screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitRecord {
    /// Unit or item type, e.g. `hfoo`; `uDNR`/`iDNR` for a random one.
    pub kind: String,
    /// Which variant of the model to use.
    pub variation: i32,
    /// World position.
    pub x: f32,
    /// World position.
    pub y: f32,
    /// World position.
    pub z: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// Owning player, 0-based; 16 is neutral passive.
    pub player: i32,
    /// Hit points, or `-1` for the type's default.
    pub hit_points: i32,
    /// Mana: `-1` for the type's default, `0` for a unit with no mana.
    pub mana: i32,
    /// Index into the map's item tables, or `null` for version 7 files, which have no
    /// such field.
    pub item_table: Option<i32>,
    /// How many dropped-item sets the record carries.
    pub drop_sets: u32,
    /// How many inventory entries it carries.
    pub inventory: u32,
    /// How many ability modifications it carries.
    pub abilities: u32,
    /// Gold carried, default 12500.
    pub gold: i32,
    /// Target acquisition range; `-1` is normal and `-2` is "camp".
    pub target_acquisition: f32,
    /// Hero level, 1 for non-heroes.
    pub hero_level: i32,
    /// The World Editor's creation number.
    pub creation_number: i32,
}

/// One placed doodad, as the list shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoodadRecord {
    /// Doodad type, e.g. `LTlt` for a Lordaeron summer tree.
    pub kind: String,
    /// Which variant of the model to use.
    pub variation: i32,
    /// World position.
    pub x: f32,
    /// World position.
    pub y: f32,
    /// World position.
    pub z: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// The stored flags value.
    pub flags: u8,
    /// That value as the core names it.
    ///
    /// ⚠ Composed by `DoodadFile::flags_name`, **not** by a match in this layer. The
    /// byte is a small integer rather than independent bits, and which integers mean what
    /// is a format conclusion — the one thing the interface may not hold an opinion
    /// about.
    pub flags_name: String,
    /// Life as a percentage of the type's default.
    pub life: u8,
    /// Pointer to a random item table in `war3map.w3i`, or `-1` for none.
    pub item_table: i32,
    /// How many item sets the record carries.
    pub item_sets: u32,
    /// The World Editor's doodad number, unique per map.
    pub editor_id: i32,
}

/// One entry in a type or player histogram.
///
/// `key` and `count` are deliberately not renamed to camel case: `key` is the field name
/// a user sees, and a type key is a four-character code or a player number, not a
/// JavaScript identifier.
#[derive(Debug, Clone, Serialize)]
pub struct TypeCount {
    /// The type or player the count is for.
    pub key: String,
    /// How many records it covers.
    pub count: u32,
}

/// A type histogram, with the ranked head extracted.
///
/// # Why the head is computed here and not in the interface
///
/// The interface must not sort by count and take twelve: "most common types" is a
/// statement about the map's contents, and a second implementation of it is exactly the
/// divergence `docs/03` §1.1 exists to prevent. The full histogram is carried too, and
/// that is not redundancy — `war3 map units` prints **twelve rows** while
/// `(4)LostTemple.w3m` has **22** distinct unit types, so the head alone cannot say how
/// much it is a head *of*.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeSummary {
    /// How many distinct types there are.
    pub distinct: u32,
    /// How many records the histogram covers.
    pub total: u32,
    /// Every type and its count, most common first.
    pub counts: Vec<TypeCount>,
    /// The same list, truncated to the number the panel displays.
    pub leaders: Vec<TypeCount>,
}

/// How many types the ranked list keeps.
///
/// Twelve is `war3 map units` and `war3 map doodads`' own head, and matching it is the
/// point: a user comparing the panel against the command line must see the same list.
pub const LEADER_COUNT: usize = 12;

/// Builds a [`TypeSummary`] from a histogram the core counted.
///
/// # Why the whole histogram is carried as well as the head
///
/// `war3 map units` prints **twelve rows** while `(4)LostTemple.w3m` has **22** distinct unit types,
/// so a head alone cannot say how much it is a head *of*. The core returns the complete histogram
/// and the two views take what each needs — the cap is a display decision and lives at the display.
fn type_summary(counts: Vec<war3_map::CountEntry<String>>) -> TypeSummary {
    TypeSummary {
        distinct: count(counts.len()),
        total: counts.iter().map(|entry| entry.count).sum(),
        leaders: counts
            .iter()
            .take(LEADER_COUNT)
            .cloned()
            .map(entry)
            .collect(),
        counts: counts.into_iter().map(entry).collect(),
    }
}

/// One core count entry as the interface's shape.
fn entry(from: war3_map::CountEntry<String>) -> TypeCount {
    TypeCount {
        key: from.key,
        count: from.count,
    }
}

/// How many records the detail list keeps.
///
/// # Why the list is capped, and why the panel says so
///
/// `(4)LostTemple.w3m` holds 121 units but **5,317 doodads**. Serialising all of them
/// across the IPC bridge to render a table nobody scrolls is the wrong default, so the
/// detailed records stop at this many and the panel names the number and how many are
/// missing. A silent truncation would make a partial list look complete, which is the
/// failure this constant exists to avoid.
pub const DETAIL_LIMIT: usize = 200;

/// A map's placed units and items.
///
/// `ok` is `false` when the map's `war3mapUnits.doo` is absent or unreadable, and
/// `reason` then says which. That is not an error state: a map without units is a valid
/// map, and the core models the file as optional for exactly that reason. The two cases
/// line up with what `war3 map units` reports.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    /// The map this was read from.
    pub path: String,
    /// `false` when the map has no readable `war3mapUnits.doo`.
    pub ok: bool,
    /// Why, in the core's own words, when `ok` is `false`.
    pub reason: Option<String>,
    /// The header numbers and histograms, present only when `ok`.
    pub info: Option<UnitInfo>,
    /// The first [`DETAIL_LIMIT`] records, in file order.
    pub records: Vec<UnitRecord>,
    /// Diagnostics from the parse and from the map.
    pub diagnostics: Vec<DiagnosticView>,
}

/// `war3mapUnits.doo`'s header numbers and histograms.
///
/// Every field here is also printed by `war3 map units`. Matching that output is not
/// politeness: the moment the screen and the command line can disagree about how many
/// units a map has, one of them is lying and nobody can tell which.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitInfo {
    /// On-disk format version.
    pub version: i32,
    /// Sub-version. It does **not** select the record layout; the version does.
    pub subversion: i32,
    /// How many records parsed.
    pub records: u32,
    /// Units above hero level 1.
    pub levelled: u32,
    /// Units carrying a real item table index or a non-empty inventory.
    ///
    /// # Why this is not "how many have the field"
    ///
    /// Version 8 always carries the item-table pointer, so counting its presence would
    /// claim every unit in a version 8 map has item data. Only an index that is not `-1`,
    /// or an inventory with something in it, counts.
    pub placed_items: u32,
    /// How many records the detail list below this carries.
    pub shown: u32,
    /// Types by frequency.
    pub types: TypeSummary,
    /// Records per player, player order.
    pub players: Vec<TypeCount>,
}

/// A map's placed doodads.
///
/// The same shape as [`UnitView`], for the same reasons.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoodadView {
    /// The map this was read from.
    pub path: String,
    /// `false` when the map has no readable `war3map.doo`.
    pub ok: bool,
    /// Why, in the core's own words, when `ok` is `false`.
    pub reason: Option<String>,
    /// The header numbers and histogram, present only when `ok`.
    pub info: Option<DoodadInfo>,
    /// The first [`DETAIL_LIMIT`] records, in file order.
    pub records: Vec<DoodadRecord>,
    /// Diagnostics from the parse and from the map.
    pub diagnostics: Vec<DiagnosticView>,
}

/// `war3map.doo`'s header numbers and histogram.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoodadInfo {
    /// On-disk format version: 7 for the fixed record, 8 for the item-bearing one.
    pub version: i32,
    /// Sub-version. Observed values are 9 and 11, and it does not select the layout.
    pub subversion: i32,
    /// Version word of the special-doodad block.
    pub special_version: i32,
    /// How many records parsed.
    pub records: u32,
    /// How many special doodads follow them.
    pub special: u32,
    /// The special doodads' types, by frequency.
    ///
    /// ⚠ Not printed by `war3 map doodads`, which reports the count alone. It is carried
    /// because the block is otherwise invisible: every sample here has zero of them, and
    /// "0 special doodads" tells a reader nothing about whether the field was read at all.
    pub special_kinds: Vec<TypeCount>,
    /// How many records the detail list below this carries.
    pub shown: u32,
    /// Types by frequency.
    pub types: TypeSummary,
}

/// One field change on an object.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectModification {
    /// The field's four-character id, e.g. `unam` for a unit's name.
    ///
    /// ⚠️ Kept alongside `field_label` rather than replaced by it: the id is what the file stores, and
    /// a reader comparing the editor against the file — or against `war3 map objects` — needs it.
    pub field: String,
    /// What the game calls this field, e.g. 名字 for `unam`, or `None` when the metadata does not
    /// describe it.
    ///
    /// ⚠️ Resolved from the game's `*MetaData.slk` `displayName` column **and** the string table it
    /// points into — a three-hop chain that cannot be reproduced here, because the middle hop is a
    /// `WESTRING_*` key whose text is in another file, in the installation's language.
    pub field_label: Option<String>,
    /// The field's `type` word, e.g. `abilityList`.
    ///
    /// This is what tells the interface whether the value's comma-separated parts are **object ids**
    /// worth resolving to names. `None` for an int or a plain string, which is most fields.
    pub value_type: Option<String>,
    /// The value, as the core's own `Display` renders it.
    ///
    /// ⚠️ `FieldValue` distinguishes four storage types and this collapses them to text. The type that
    /// matters for display is [`ObjectModification::value_type`], which is the **field's** declared
    /// type: it says whether the parts of this string are ids. `uabi` holds
    /// `A04M,A03Y,Ahrp,A005,A001,A00M` — six ability ids — and resolving them is what turns a column
    /// of codes into a list of names.
    ///
    /// ⚠️ For a **list-valued** field this is the raw string and the interface splits and resolves it.
    /// For every other field it is still what the file stores, and [`Self::value_label`] is what to
    /// show — they differ whenever the stored form is a word or a reference.
    pub value: String,
    /// What to show for the value, when it is **not** a list of object ids; `None` when it is.
    ///
    /// Three cases, all measured against a real map:
    ///
    /// - `ua1t` = `hero` → 英雄: a **word**, whose text is a section of `UI\UnitEditorData.txt`
    /// - `unam` = `TRIGSTR_4227` → the map's own text: a reference into `war3map.wts`
    /// - `uhpm` = `100` → `100`: a number, already its own text
    ///
    /// `None` for a list is not "no label" — it is "the label is not computable here", because it
    /// needs one name lookup per comma-separated part.
    pub value_label: Option<String>,
    /// Which level the value applies to, or `null` when the kind is not levelled.
    ///
    /// 0 means "every level". Which kinds carry the block is the core's `is_levelled`, not
    /// a list here.
    pub level: Option<i32>,
    /// Which `DataA..DataI` column the value belongs to, when levelled.
    pub data_indicator: Option<i32>,
}

/// One object the map creates or modifies.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectEntry {
    /// The object's own id.
    pub id: String,
    /// What it inherits from; empty for a modified original, whose `id` *is* the base
    /// object.
    pub base_id: String,
    /// Whether the game treats it as a hero.
    ///
    /// From `ObjectKind::is_hero_id`, which knows the uppercase-first-letter rule and that
    /// the rule only means anything for units. Deciding it here would be a format
    /// conclusion.
    pub hero: bool,
    /// How many modifications the record holds in the file.
    pub modifications: u32,
    /// The modifications shown, at most [`MODIFICATION_LIMIT`] of them.
    pub shown: Vec<ObjectModification>,
}

/// How many modifications one object's entry shows.
///
/// The same reasoning as [`DETAIL_LIMIT`]: enough to see what the author changed, bounded
/// so one object with a thousand fields cannot dominate the payload. The count is carried
/// beside it so the panel can say what is missing.
pub const MODIFICATION_LIMIT: usize = 12;

/// One category of object data: the unit, item, ability and four others.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectCategory {
    /// The map member this came from, e.g. `war3map.w3u`.
    pub map_file: String,
    /// The category, as the core's own name for it.
    pub kind: String,
    /// On-disk format version.
    pub version: i32,
    /// How many Blizzard objects the map modifies.
    pub original: u32,
    /// How many objects the map creates.
    pub custom: u32,
    /// How many entries the lists below carry, across both tables.
    pub shown: u32,
    /// The objects created by the map, first.
    pub custom_objects: Vec<ObjectEntry>,
    /// The Blizzard objects it modifies.
    pub modified_objects: Vec<ObjectEntry>,
}

/// A map's object data, across every category it has.
///
/// There is no `ok` flag here, unlike the unit and doodad views. A map with no object
/// files is not a map with something unavailable — "no object files in this map" is the
/// whole answer, and `categories` being empty in both cases makes a second flag carry no
/// information.
///
/// ⚠ **Field ids are shown unresolved.** `war3 map objects` prints names when it is given
/// `--game-dir`, because the metadata tables that map `uhpm` to "Hit Points" live in the
/// game's `*MetaData.slk`, not in the map. This app has no game-directory setting yet, so
/// it shows ids and values — the same thing the command line shows without that flag, and
/// not an error state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectView {
    /// The map this was read from.
    pub path: String,
    /// The categories present, in the core's own category order.
    pub categories: Vec<ObjectCategory>,
    /// Parse failures and the core's own object diagnostics.
    pub diagnostics: Vec<DiagnosticView>,
}

/// A list length as the DTO's own count type.
///
/// `u32` rather than `usize`: the front end reads it as a number either way, and a length
/// that cannot be a `u32` is not a length any real file reports.
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Builds the unit header numbers and histogram.
///
/// ⚠️ The numbers are `war3_map::UnitSummary`'s, which is the **same** implementation
/// `war3 map units` prints from. They used to be counted here as well, with the same tie-break rule
/// written out in both places — a coincidence rather than a guarantee, and the symptom of it ending
/// would have been a panel and a command line that cannot be compared row by row.
pub fn unit_summary(units: &war3_map::UnitFile) -> UnitInfo {
    let summary = war3_map::UnitSummary::of(units);
    UnitInfo {
        version: summary.version,
        subversion: summary.subversion,
        records: summary.records,
        levelled: summary.levelled,
        placed_items: summary.placed_items,
        shown: count(units.units.len().min(DETAIL_LIMIT)),
        types: type_summary(summary.types.ranked()),
        players: summary
            .players
            .iter()
            .map(|(player, count)| TypeCount {
                key: format!("player {player}"),
                count: *count,
            })
            .collect(),
    }
}

/// Builds the doodad header numbers and histogram.
pub fn doodad_summary(doodads: &war3_map::DoodadFile) -> DoodadInfo {
    let summary = war3_map::DoodadSummary::of(doodads);
    DoodadInfo {
        version: summary.version,
        subversion: summary.subversion,
        special_version: summary.special_version,
        records: summary.records,
        special: summary.special,
        special_kinds: summary
            .special_types
            .ranked()
            .into_iter()
            .map(entry)
            .collect(),
        shown: count(doodads.doodads.len().min(DETAIL_LIMIT)),
        types: type_summary(summary.types.ranked()),
    }
}

/// Flattens one unit record for the list.
pub fn unit_record(unit: &war3_map::Unit) -> UnitRecord {
    UnitRecord {
        kind: unit.kind.to_string(),
        variation: unit.variation,
        x: unit.position.x,
        y: unit.position.y,
        z: unit.position.z,
        rotation: unit.rotation,
        player: unit.player,
        hit_points: unit.hit_points,
        mana: unit.mana,
        item_table: unit.item_table,
        drop_sets: count(unit.drop_sets.len()),
        inventory: count(unit.inventory.len()),
        abilities: count(unit.abilities.len()),
        gold: unit.gold,
        target_acquisition: unit.target_acquisition,
        hero_level: unit.hero_level,
        creation_number: unit.creation_number,
    }
}

/// Flattens one doodad record for the list.
pub fn doodad_record(doodad: &war3_map::Doodad) -> DoodadRecord {
    DoodadRecord {
        kind: doodad.kind.to_string(),
        variation: doodad.variation,
        x: doodad.position.x,
        y: doodad.position.y,
        z: doodad.position.z,
        rotation: doodad.rotation,
        flags: doodad.flags,
        flags_name: war3_map::DoodadFile::flags_name(doodad.flags).to_owned(),
        life: doodad.life,
        item_table: doodad.item_table,
        item_sets: count(doodad.item_sets.len()),
        editor_id: doodad.editor_id,
    }
}

/// Flattens one object, keeping at most [`MODIFICATION_LIMIT`] modifications.
///
/// `kind` is passed down for one reason: a field's label and type are looked up **per object kind**,
/// because the same four letters can mean different things in two tables. It comes from the file the
/// object was parsed out of rather than from a guess, so it cannot disagree with the ids inside.
fn object_entry(object: &war3_object::Object, kind: war3_object::ObjectKind) -> ObjectEntry {
    ObjectEntry {
        id: object.id.to_string(),
        base_id: if object.base_id.is_zero() {
            String::new()
        } else {
            object.base_id.to_string()
        },
        hero: war3_object::ObjectKind::is_hero_id(object.id),
        modifications: count(object.modifications.len()),
        shown: object
            .modifications
            .iter()
            .take(MODIFICATION_LIMIT)
            .map(|m| {
                let field = m.field.to_string();
                let value_type = crate::names::field_type(kind, &field);
                let raw = m.value.to_string();
                // ⚠️ A list-valued field is left **unresolved** here on purpose. Its parts are object
                // ids, and the interface already has a batched, coalescing resolver for exactly that
                // (`useObjectNames`) — resolving them here as well would be a second path to the same
                // answer, and it would put the splitting rule in two places.
                //
                // ⚠️ The test is the core's `value_shape` and not `list_element_kind`: `techList`
                // (`ureq` = `R00M,R00T`) is a list of ids whose **kind is not fixed** — they may be
                // units, items, abilities or upgrades — so `list_element_kind` answers `None` for it and
                // a check built on that would treat a list of ids as a single value.
                let is_list =
                    war3_game::value_shape(value_type.as_deref()) == war3_game::ValueShape::IdList;
                let value_label = if is_list {
                    None
                } else {
                    Some(crate::names::value_text(
                        &raw,
                        value_type.as_deref(),
                        crate::names::map_strings().as_ref(),
                    ))
                };

                ObjectModification {
                    // ⚠️ Resolved from the game's metadata, not from a table in this file. Before
                    // this, the screen showed `unam`, `uabi`, `uahp` — the ids the file stores.
                    field_label: crate::names::field_label(kind, &field),
                    value_type,
                    field,
                    value: raw,
                    value_label,
                    level: m.level.map(|l| l.level),
                    data_indicator: m.level.map(|l| l.data_indicator),
                }
            })
            .collect(),
    }
}

/// Builds one category's view from a parsed object file.
///
/// The entries are truncated per table rather than across both, because the two are
/// printed as separate lists — the same split `war3 map objects` uses.
pub fn object_category(file: &war3_object::ObjectFile, kind_name: &str) -> ObjectCategory {
    let kind = file.kind;
    let custom_objects: Vec<ObjectEntry> = file
        .table
        .custom
        .iter()
        .take(DETAIL_LIMIT)
        .map(|object| object_entry(object, kind))
        .collect();
    let modified_objects: Vec<ObjectEntry> = file
        .table
        .original
        .iter()
        .take(DETAIL_LIMIT)
        .map(|object| object_entry(object, kind))
        .collect();

    ObjectCategory {
        map_file: file.kind.map_file().to_owned(),
        kind: kind_name.to_owned(),
        version: file.version,
        original: count(file.table.original.len()),
        custom: count(file.table.custom.len()),
        shown: count(custom_objects.len() + modified_objects.len()),
        custom_objects,
        modified_objects,
    }
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
    ///
    /// ⚠️ Where `subject` is set this **still begins** `"<subject>: "`, because it is the sentence
    /// `war3 map list` prints, and there the line stands alone. A consumer with its own column for
    /// the name uses `subject` and shows the message as it is — see `disposition_reason`, which is
    /// built from the core's `display_message`.
    pub message: String,
    /// What the diagnostic is about — a member name, a field id — or `None` when it is about the
    /// file as a whole.
    pub subject: Option<String>,
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
    pub fn from_parse(path: &str, archive: &war3_archive::Archive, map: &war3_map::Map) -> Self {
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
                let (disposition, reason) =
                    match war3_project::disposition::of(archive, name, &mut scratch) {
                        Ok(war3_project::Disposition::Textified { .. }) => {
                            ("text".to_string(), String::new())
                        }
                        Ok(war3_project::Disposition::KeptBinary { reason }) => {
                            ("binary".to_string(), reason)
                        }
                        Ok(war3_project::Disposition::KeptRaw) => (
                            "raw".to_string(),
                            "this workspace cannot decode it, so its stored block is kept"
                                .to_string(),
                        ),
                        Ok(war3_project::Disposition::Absent) => {
                            ("absent".to_string(), String::new())
                        }
                        // `of` only errors when the container is inconsistent, which the
                        // summary is not the place to fail over: the member still exists
                        // and still has a size.
                        Err(e) => ("binary".to_string(), e.to_string()),
                    };
                Some(Member {
                    name: name.to_owned(),
                    size: block.uncompressed_size,
                    disposition,
                    // ⚠️ Used as given. This table has a name column, so the message must not repeat
                    // the name — and it no longer does: the core keeps the `"<name>: "` prefix for
                    // the command line (where the line stands alone) and hands a consumer with its
                    // own column the prefix-free text through `Diagnostic::display_message`, which
                    // is what `Disposition::KeptBinary`'s reason is built from.
                    //
                    // It used to be stripped here, by string surgery on the core's own sentence:
                    // `strip_prefix(name).and_then(|r| r.strip_prefix(": "))`. That was a second
                    // implementation of the core's formatting, and it would have gone on quietly
                    // working until the wording changed.
                    disposition_reason: reason,
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
                subject: d.subject.clone(),
            })
            .collect();

        Self {
            path: path.to_owned(),
            // ⚠️ Markup decoded, and decoded by the **core**. A map name is usually
            // `|cffffff00IMBA 3.83f AI|r`, and those codes are a conclusion about what the bytes
            // mean — so `war3_map::plain` decides, and `war3 map info` calls the same function.
            // A `strip_prefix("|c")` here would be a second, quietly different answer: it would
            // miss a colour that is not first in the string, and it would leave `|r` behind.
            name: war3_map::plain(&map.metadata.name),
            author: war3_map::plain(&map.metadata.author),
            description: war3_map::plain(&map.metadata.description),
            recommended_players: map
                .metadata
                .recommended_players
                .as_deref()
                .map(war3_map::plain),
            // The same fields again, with the runs the author coloured kept separate. Both come
            // from `war3_map`, so the plain and the rich forms cannot disagree about the text.
            name_rich: RichText::from_core(&map.metadata.name),
            author_rich: RichText::from_core(&map.metadata.author),
            description_rich: RichText::from_core(&map.metadata.description),
            recommended_players_rich: map
                .metadata
                .recommended_players
                .as_deref()
                .map(RichText::from_core),
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
        // ⚠️ And the markup is gone too. A map name in the wild is
        // `|cffffff00IMBA 3.83f AI|r`, and a panel that showed those ten characters would be
        // showing the reader the colour code instead of the name. This asserts the *shape*
        // rather than one map's title, so it holds for any map pointed at.
        for (what, text) in [
            ("name", &summary.name),
            ("author", &summary.author),
            ("description", &summary.description),
        ] {
            assert!(
                !text.contains("|c"),
                "the {what} still carries a colour code: {text:?}"
            );
            assert!(
                !text.contains("\r\n"),
                "the {what} still carries the format's CRLF: {text:?}"
            );
        }
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
            summary
                .members
                .iter()
                .map(|m| &m.name)
                .eq(sorted.iter().map(|m| &m.name)),
            "members must be in a stable order"
        );
    }

    /// A map whose name carries colour codes must arrive decoded.
    ///
    /// `DotA_IMBA_3.83.w3x` stores its name as `|cffffff00IMBA 3.83 AI|r` and repeats the
    /// pattern in its description. Both codes are stripped by `war3_map::plain`, which is the
    /// same decoder `war3 map info` prints with — so the panel and the command line cannot
    /// disagree about what this map is called.
    ///
    /// Point `W3_TEST_MAP_UNCANNY` at that map. Skipped when it is unset, like the other
    /// real-map tests here.
    #[test]
    fn a_map_name_with_colour_codes_is_decoded() {
        let Some(path) = std::env::var("W3_TEST_MAP_UNCANNY").ok() else {
            return;
        };
        let archive = war3_archive::Archive::open(&path).expect("the map named by the variable");
        let map = war3_map::Map::from_source(&archive).expect("a parseable map");
        let summary = MapSummary::from_parse(&path, &archive, &map);

        // The raw fields carry the markup. If they did not, this test would be asserting
        // nothing and would pass on any map — which is the failure mode worth guarding.
        assert!(
            map.metadata.name.contains("|c"),
            "this test needs a map whose name has a colour code, got {:?}",
            map.metadata.name
        );
        assert!(
            !summary.name.contains('|'),
            "no code should survive: {:?}",
            summary.name
        );
        assert!(
            !summary.name.trim().is_empty(),
            "decoding must not eat the name itself: {:?}",
            summary.name
        );
        // The description is where the CRLF lives, and a label showing one would print a
        // stray carriage return.
        assert!(
            map.metadata.description.contains("\r\n"),
            "this test needs the format's line endings, got {:?}",
            map.metadata.description
        );
        assert!(
            !summary.description.contains('\r'),
            "CRLF should be normalised: {:?}",
            summary.description
        );
        assert!(
            summary.description.contains('\n'),
            "the line break itself must survive: {:?}",
            summary.description
        );
        eprintln!("decoded name: {:?}", summary.name);
    }
}
