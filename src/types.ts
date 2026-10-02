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

/** Everything the interface shows about one open map. */
export interface MapSummary {
  path: string
  name: string
  author: string
  description: string
  recommendedPlayers: string | null
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
