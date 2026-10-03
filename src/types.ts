/**
 * The shape of what the backend sends.
 *
 * # Why these are written out by hand
 *
 * They mirror `src-tauri/src/dto.rs`. Generating them would mean a codegen step in
 * the build for nine small interfaces, and the mismatch it would prevent is caught
 * immediately: a renamed field fails at the first `invoke`, and the DTO's own test
 * pins the Rust side.
 *
 * So when a field changes there, it changes here in the same commit. The
 * `camelCase` names are produced by `#[serde(rename_all = "camelCase")]` on the Rust
 * side, not translated here.
 */

/** A colour as stored in the map, before the renderer decides how light to draw it. */
export interface SegmentColour {
  red: number
  green: number
  blue: number
}

/** One run of text with an optional colour. */
export interface RichSegment {
  text: string
  /** The author's colour for this run, or `null` for text left at the default. */
  colour: SegmentColour | null
}

/**
 * A map text field, split into the runs the author coloured.
 *
 * Always at least one segment: a field with no markup is one segment with no colour, so a panel
 * renders both cases through one path.
 *
 * ⚠️ There is no alpha. WC3 writes one and it is `0` on most real text — the Sentinel's
 * `|c00ff0303` — which is fully transparent; the game ignores it, and the core does not send it
 * so that no renderer can honour it by mistake.
 */
export interface RichText {
  segments: RichSegment[]
}

/** One archive member. */
export interface Member {
  name: string
  size: number
  /** `text`, `binary`, `raw` or `absent` — what `extract` would do with it. */
  disposition: string
  /** Why, when the disposition is not `text`. Empty otherwise. */
  dispositionReason: string
}

/** Whether the map can be turned into a source project. */
export interface Extractability {
  ok: boolean
  usedBlocks: number
  namedBlocks: number
  /** The core's own words for the refusal, or `null` when there would not be one. */
  reason: string | null
}

/**
 * One diagnostic.
 *
 * `severity` and `code` are the core's own spellings — the same strings the
 * command-line tool prints — so a message can be compared between the two by eye.
 */
export interface DiagnosticView {
  severity: string
  code: string
  message: string
}

/** Terrain measurements, all as the core reports them. */
export interface TerrainInfo {
  version: number
  recordSize: number
  textureBits: number
  maxTextures: number
  width: number
  height: number
  tileWidth: number
  tileHeight: number
  tileCount: number
  tileset: string
  customTileset: boolean
  centerOffsetX: number
  centerOffsetY: number
  worldOriginX: number
  worldOriginY: number
  minWorldHeight: number
  maxWorldHeight: number
  groundTextureCount: number
  cliffTextureCount: number
  usedGroundTextures: number[]
  groundTextureIds: string[]
  cliffTextureIds: string[]
  rampPoints: number
  blightPoints: number
  waterPoints: number
  boundaryPoints: number
  /** 16 buckets, one per ground texture index. */
  layerHistogram: number[]
}

/** A map's terrain, or the reason it has none. */
export interface TerrainView {
  path: string
  ok: boolean
  error: string | null
  info: TerrainInfo | null
  diagnostics: DiagnosticView[]
}

/**
 * One entry in a type or player histogram.
 *
 * `key` is the field name the panel prints, so it is snake_case where a player is
 * concerned and a four-character id where a type is: renaming it would put a name on
 * screen that appears in no file and in no `war3` command.
 */
export interface TypeCount {
  key: string
  count: number
}

/**
 * A histogram of record types, with the ranked head already extracted.
 *
 * ⚠️ **The interface does not sort or slice this.** `leaders` and `counts` arrive
 * computed from the backend, because "most common types" is the same list
 * `war3 map units` prints and two implementations of it would be free to disagree.
 *
 * `war3 map units` prints **twelve** rows and LostTemple has **22** distinct unit
 * types, so `distinct` and `leaders.length` differ on real maps. The panel says so
 * rather than presenting the head as the whole picture.
 */
export interface TypeSummary {
  distinct: number
  total: number
  /** Every type and its count, most common first. */
  counts: TypeCount[]
  /** The same list, truncated to what the panel displays. */
  leaders: TypeCount[]
}

/** One placed unit or item, as the list shows it. */
export interface UnitRecord {
  kind: string
  variation: number
  x: number
  y: number
  z: number
  rotation: number
  player: number
  /** `-1` means the type's default, which needs the object data to resolve. */
  hitPoints: number
  /** `-1` for the type's default, `0` for a unit with no mana. */
  mana: number
  /** `null` on version 7 files, which have no such field. */
  itemTable: number | null
  dropSets: number
  inventory: number
  abilities: number
  gold: number
  targetAcquisition: number
  heroLevel: number
  creationNumber: number
}

/** `war3mapUnits.doo`'s header numbers and histograms. */
export interface UnitInfo {
  version: number
  subversion: number
  records: number
  levelled: number
  placedItems: number
  /** How many records `records` holds below — `records` capped, not `records`. */
  shown: number
  types: TypeSummary
  players: TypeCount[]
}

/**
 * A map's placed units and items, or the reason there are none.
 *
 * `ok: false` means the map has no readable `war3mapUnits.doo`, and `reason` says
 * which of the two cases it is. That is a state, not a failure: a map without units
 * is a valid map, and the panel says so instead of showing an error.
 */
export interface UnitView {
  path: string
  ok: boolean
  reason: string | null
  info: UnitInfo | null
  records: UnitRecord[]
  diagnostics: DiagnosticView[]
}

/** One placed doodad, as the list shows it. */
export interface DoodadRecord {
  kind: string
  variation: number
  x: number
  y: number
  z: number
  rotation: number
  flags: number
  /** That byte as the core names it — `visible solid`, `unknown`, and so on. */
  flagsName: string
  /** Life as a percentage of the type's default. */
  life: number
  itemTable: number
  itemSets: number
  editorId: number
}

/** `war3map.doo`'s header numbers and histogram. */
export interface DoodadInfo {
  version: number
  subversion: number
  specialVersion: number
  records: number
  special: number
  /** The special doodads' types. Usually empty: they are the ones cliffs are made of. */
  specialKinds: TypeCount[]
  shown: number
  types: TypeSummary
}

/** A map's placed doodads, or the reason there are none. */
export interface DoodadView {
  path: string
  ok: boolean
  reason: string | null
  info: DoodadInfo | null
  records: DoodadRecord[]
  diagnostics: DiagnosticView[]
}

/** One field change on an object. */
export interface ObjectModification {
  field: string
  /**
   * The value as text.
   *
   * The core distinguishes four storage types (integer, real, unreal, string) and
   * this is already flattened, because nothing on a read-only screen acts on the
   * type.
   */
  value: string
  /** Which level the value applies to, `0` meaning every level. */
  level: number | null
  dataIndicator: number | null
}

/** One object the map creates or modifies. */
export interface ObjectEntry {
  id: string
  /** Empty for a modified original, whose `id` *is* the base object. */
  baseId: string
  hero: boolean
  modifications: number
  /** At most twelve, with `modifications` saying how many there really are. */
  shown: ObjectModification[]
}

/** One category of object data: units, items, abilities and four others. */
export interface ObjectCategory {
  mapFile: string
  kind: string
  version: number
  original: number
  custom: number
  shown: number
  customObjects: ObjectEntry[]
  modifiedObjects: ObjectEntry[]
}

/**
 * A map's object data.
 *
 * ⚠️ Field ids are shown **unresolved**. `war3 map objects` names a field only with
 * `--game-dir`, because the metadata tables that turn `uhpm` into "Hit Points" live
 * in the game's archives rather than the map. This app has no game-directory setting
 * yet, so it shows ids and values — the same output the command line produces without
 * that flag, not a degraded version of it.
 */
export interface ObjectView {
  path: string
  categories: ObjectCategory[]
  diagnostics: DiagnosticView[]
}

/** Everything the interface shows about one open map. */
export interface MapSummary {
  path: string
  /**
   * The map name, plain: no `TRIGSTR_`, no markup codes.
   *
   * This is the form for a window title or a tooltip, where a colour cannot be drawn.
   * `nameRich` is the same text with the author's colours, and is what the panel draws.
   */
  name: string
  author: string
  description: string
  recommendedPlayers: string | null
  /** The map name split into the runs the author coloured. */
  nameRich: RichText
  /** The author's name, split the same way. */
  authorRich: RichText
  /** The description, split the same way. */
  descriptionRich: RichText
  recommendedPlayersRich: RichText | null
  formatVersion: number
  tileset: string | null
  playableWidth: number
  playableHeight: number
  flags: number
  memberCount: number
  totalMemberBytes: number
  stringCount: number
  members: Member[]
  extractable: Extractability
  diagnostics: DiagnosticView[]
}

/**
 * What a save would do, measured — not done.
 *
 * Every field is a comparison of two byte strings. There is deliberately no
 * "members preserved" flag: that would be a claim the measurement does not make,
 * and reading it as verified would be the mistake.
 */
export interface SavePreview {
  path: string
  originalBytes: number
  rebuiltBytes: number
  memberCount: number
  /** Offset of the first difference, or `null` when the rebuild is byte identical. */
  firstDifference: number | null
  /** One sentence stating what the numbers mean, composed by the core. */
  note: string
}
