/**
 * Formatting helpers for values the interface displays.
 *
 * Deliberately small and presentation-only: nothing here decides what a byte means.
 */

/**
 * A byte count as a short human-readable string.
 *
 * Binary units (KiB, MiB) rather than decimal, because these are file sizes and the
 * numbers users compare them against — a file's size on disk — are binary too.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0)
    return '?'
  if (bytes < 1024)
    return `${bytes} B`
  const units = ['KiB', 'MiB', 'GiB']
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  // One decimal below 10 so small differences are visible, none above so the
  // column stays aligned.
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`
}

/**
 * The last path component, for a window title.
 *
 * Handles both separators because a Windows build can still be handed a path with
 * forward slashes by a file dialog.
 */
export function fileName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] || path
}

/** The flags word as the hex the `.w3i` stores. */
export function formatFlags(flags: number): string {
  return `0x${flags.toString(16).padStart(8, '0')}`
}
