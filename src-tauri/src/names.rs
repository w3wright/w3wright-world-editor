//! Object name resolution: the engine that turns an ID into something a reader recognises.
//!
//! # Why there is one of these for the whole application
//!
//! `war3_game::Resolver::load` reads 21 `*Strings.txt` files, two `.slk` tables and two 370 KB
//! string tables out of the game's archives. Measured on this machine: **59 ms** for the tables, and
//! **593 ms** to open the archives in the first place. Per lookup that is nothing — a resolve is
//! 0.37 µs — but per *command* it would be catastrophic, and a unit view asks about dozens of IDs.
//!
//! So the expensive part is done once and kept. ⚠️ Note which part that is: opening the archives, not
//! reading the names. Caching the *answer* per ID would be the obvious move and would save almost
//! nothing; caching the *engine* is what removes the 593 ms.
//!
//! # Two layers, invalidated by two different things
//!
//! | Layer | Changes when |
//! | --- | --- |
//! | the game's tables — `Resolver` | the game directory changes |
//! | the map's own names — overrides | a different map is opened |
//!
//! Keeping the game tables across a map change is the point: opening a second map must not pay the
//! 593 ms again, and a user comparing two maps does exactly that.
//!
//! The lock is taken per command rather than held, and a poisoned lock is treated as "no names
//! available" rather than as a panic: a catalogue that cannot be read is a slow lookup, not a reason
//! to bring the editor down.

use std::sync::{Arc, Mutex, OnceLock};

use war3_game::{GameAssets, Resolver};
use war3_meta::ObjectKind;

use crate::dto::{NameAnswer, NameQuery};
use crate::settings;

/// The engine, once it has been built.
///
/// An `Arc` rather than a clone: the engine is 2,740 IDs and 7,925 string keys, and copying that per
/// command would trade the 593 ms it exists to avoid for milliseconds that are just as easy not to
/// spend.
///
/// `None` inside the `Option` means "no game installation", which is a state worth caching too: a
/// machine without the game must not pay 593 ms of failed archive opens on every lookup.
static ENGINE: OnceLock<Mutex<Option<Arc<Resolver>>>> = OnceLock::new();

/// The open map's `war3map.wts`, kept for the same reason the engine is.
///
/// ⚠️ A field value like `TRIGSTR_4227` resolves through the **map's** table and not the game's, so
/// reading it needs the map — and re-reading it per object row would open the archive again for every
/// row of the object view. It is stored here beside the engine because both are "what is loaded right
/// now", and both are invalidated by the same two events: a different map, or a different game.
static MAP_STRINGS: OnceLock<Mutex<Option<war3_map::StringTable>>> = OnceLock::new();

/// Resolves a batch of IDs to names.
///
/// ⚠️ Batched because the caller is a table: a unit view holds 121 records and asks about the IDs
/// among them, and one IPC round trip per ID would be 121 round trips to answer one screen.
#[must_use]
pub fn resolve(queries: &[NameQuery]) -> Vec<NameAnswer> {
    let Some(resolver) = engine() else {
        // No installation: every ID stands for itself. That is the same answer an object the game
        // has never heard of gets, so the interface needs no second code path for it.
        return queries
            .iter()
            .map(|q| NameAnswer {
                kind: q.kind.clone(),
                id: q.id.clone(),
                name: q.id.clone(),
                source: "id".to_string(),
            })
            .collect();
    };

    queries
        .iter()
        .map(|query| {
            let kind = parse_kind(&query.kind);
            let resolved = resolver.resolve(kind, &query.id);
            NameAnswer {
                kind: query.kind.clone(),
                id: query.id.clone(),
                // ⚠️ Markup is **not** stripped here. Names can carry `|cffffff00…|r`, and decoding
                // that is `war3_map::plain`'s job — one function, used by the command line and the
                // panels alike. Doing it in this layer as well would be a second answer to a question
                // the core has already answered once.
                name: resolved.name,
                source: resolved.source.label().to_string(),
            }
        })
        .collect()
}

/// Records the names a map gives its own objects.
///
/// The map's `.w3u` and its six siblings hold the objects the author created and the ones they
/// renamed, in a field called `unam` — usually as a `TRIGSTR_nnn` reference into the map's own
/// `war3map.wts`, which is why the string table is read alongside.
///
/// ⚠️ This replaces the engine's map layer rather than adding to it, so opening a second map cannot
/// leave the first one's names in place. What it does **not** do is reload the game tables: those
/// depend on the installation, not the map, and a user comparing two maps should not pay 593 ms per
/// map.
///
/// ⚠️ Takes an already-open archive rather than a path. Opening one means reading four MPQ hash and
/// block tables — 593 ms on this machine, which is the cost this module exists to pay once — and
/// `open_map` has one open already.
///
/// A map whose object files are absent or unreadable simply contributes no overrides: the game's
/// names and the ids still answer, which is the same degradation as having no map open at all.
pub fn set_map(archive: &war3_archive::Archive) {
    // The map's string table, which `unam` values point into. Absent is normal — a map with no
    // translated text has none — and a reference with nothing to resolve it comes back as written.
    let strings = archive
        .read_file("war3map.wts")
        .ok()
        .map(|bytes| war3_map::StringTable::parse(&bytes));
    set_map_strings(strings.clone());

    let mut parsed: Vec<(war3_object::ObjectKind, war3_object::ObjectFile)> = Vec::new();
    for kind in war3_object::ObjectKind::ALL {
        let Ok(bytes) = archive.read_file(kind.map_file()) else {
            continue;
        };
        match war3_object::ObjectFile::parse(kind, &bytes) {
            Ok(file) => parsed.push((kind, file)),
            // A category that will not parse is skipped rather than failing the whole map: the other
            // six are still worth naming, and `read_objects` reports the fault where a user is looking
            // at that table.
            Err(_) => continue,
        }
    }
    // ⚠️ `names_for` and not a loop here: it takes the name field **per kind** (`unam` for units and
    // items, `gnam` for upgrades, `bnam`, `dnam`, `anam`) and skips the one kind that has none. A loop
    // passing `unam` for everything was the first version, and it resolved units while silently
    // failing for every other kind.
    let names = war3_object::names_for(
        parsed.iter().map(|(kind, file)| (*kind, file)),
        strings.as_ref(),
        war3_game::name_field,
    );

    // ⚠️ **Builds the engine if it is not up yet**, rather than returning.
    //
    // This is the ordering trap the first version fell into: `open_map` runs before anything has
    // asked for a name, so the slot was still `None`, the map's names were dropped, and every custom
    // object went on showing its id — with no error and no way to tell that the lookup had been
    // skipped rather than missed. Building here costs the archive open (which `open_map` has already
    // paid) plus the table load, and it is what the first lookup would have done anyway.
    //
    // ⚠️ **The guard is dropped before `replace`**, and that is not tidiness: `replace` takes the same
    // lock, so calling it while this one is held deadlocks the whole application — `open_map` never
    // returns and every later command queues behind it. That is exactly what happened, and it is why
    // the scoped block below exists instead of a single `let`.
    let resolver = {
        let Ok(mut guard) = engine_slot().lock() else {
            return;
        };
        if guard.is_none() {
            *guard = build().map(Arc::new);
        }
        guard.clone()
    };
    let Some(resolver) = resolver else {
        // No game installation, so there is nothing to add the map's names to. The interface shows
        // ids in that case anyway, and this is the state a machine without the game is in — not an
        // error.
        return;
    };

    // ⚠️ Cloned, cleared and fed, rather than replaced: replacing would drop the game tables and pay
    // the 593 ms again on the next lookup, which is the whole cost this module exists to avoid.
    // `clear_overrides` is what makes mutating it safe — without it, `set_overrides` would pile one
    // map's names on top of another's and the second map would show the first one's objects.
    let mut updated = (*resolver).clone();
    updated.clear_overrides();
    updated.set_overrides(
        names
            .iter()
            .map(|(kind, id, name)| (kind, id.to_string(), name.to_string())),
    );
    replace(updated);
}

/// How many field labels were resolved, for a report.
///
/// Read from the same resident resolver the names come from, so it costs nothing: the metadata tables
/// were read once, at load.
#[must_use]
pub fn field_label(kind: ObjectKind, field_id: &str) -> Option<String> {
    let resolver = engine()?;
    resolver.field(kind, field_id).and_then(|f| f.label.clone())
}

/// What kind of value a field holds, e.g. `abilityList`.
///
/// ⚠️ This is what tells the interface whether a value's parts are **object ids** worth resolving.
/// It comes from the metadata's `type` column rather than from a prefix on the field id, because a
/// prefix rule (`u` = unit) is a convention this workspace would have invented.
#[must_use]
pub fn field_type(kind: ObjectKind, field_id: &str) -> Option<String> {
    let resolver = engine()?;
    resolver
        .field(kind, field_id)
        .and_then(|f| f.type_name.clone())
}

/// The text for a word-valued field, e.g. `attackType` + `hero` → 英雄.
#[must_use]
pub fn word_text(type_name: &str, value: &str) -> Option<String> {
    let resolver = engine()?;
    resolver.word(type_name, value)
}

/// What to show for a field's value, when the value is **not** a list of object ids.
///
/// Three shapes end up here, and they are the three that a real map's object data contains:
///
/// | Field | Stored | Shown |
/// | --- | --- | --- |
/// | `ua1t` | `hero` | 英雄 — a **word**, whose text is in the editor's own table |
/// | `uhpm` | `100` | `100` — a number, which is already its own text |
/// | `unam` | `TRIGSTR_4227` | the map's text for 4227, or the reference when the map has none |
///
/// ⚠️ The word case is the one that cannot be done anywhere but here: `hero` means nothing without
/// `UI\UnitEditorData.txt`, and its text then goes through a second `WESTRING_*` hop. A caller that
/// received only `hero` would have to hard-code the mapping, in one language.
///
/// A `TRIGSTR_` reference resolves through the **map's** string table, which lives in the archive
/// rather than in the resolver — hence the argument. A reference the map does not define comes back
/// as written, which is what tells a reader that the map's text is missing rather than empty.
///
/// ⚠️ Takes only the **type word** and not the kind and field id, because the type word is the whole
/// input: everything this decides follows from it. An earlier signature carried the kind and the field
/// id and used neither, which is an interface claiming a dependency it does not have.
#[must_use]
pub fn value_text(
    value: &str,
    field_type: Option<&str>,
    map_strings: Option<&war3_map::StringTable>,
) -> String {
    if value.is_empty() {
        return String::new();
    }

    // A **word list** — `ua1g`'s `air,debris,enemies,ground,item,structure,ward` is the target-allowed
    // flags. ⚠️ These are not object ids: nothing about `air` is a four-character code, so the id
    // resolver is the wrong tool and would query seven tables for seven guaranteed misses. Each part is
    // a key in the editor's own table, exactly like a single word.
    if war3_game::value_shape(field_type) == war3_game::ValueShape::WordList {
        return value
            .split(',')
            .map(|part| {
                word_text(field_type.unwrap_or_default(), part.trim())
                    .unwrap_or_else(|| part.trim().to_string())
            })
            .collect::<Vec<_>>()
            .join(", ");
    }

    // A word value: the type word names the editor's section, and the value its entry.
    if let Some(text) = field_type.and_then(|ty| word_text(ty, value)) {
        return text;
    }

    // A reference into the map's own string table.
    if war3_map::parse_trigstr(value).is_some() {
        let Some(strings) = map_strings else {
            return value.to_string();
        };
        let mut scratch = war3_core::diag::Diagnostics::new();
        return strings.resolve(value, &mut scratch);
    }

    // Anything else — a number, a flag, plain text — is already what a reader should see.
    value.to_string()
}

/// How many IDs the engine knows, per kind.
///
/// Empty when no installation is configured, which is a state the settings screen reports rather
/// than an error: the editor works without a game, showing IDs where names would be.
#[must_use]
pub fn stats() -> Vec<(String, usize)> {
    engine().map_or_else(Vec::new, |resolver| {
        resolver
            .stats()
            .per_kind
            .into_iter()
            .map(|(kind, count)| (kind.to_string(), count))
            .collect()
    })
}

/// Drops the engine entirely, for when the game installation changes.
///
/// Unlike [`set_map`] this does discard the game tables, because the one thing that can have changed
/// is where they come from. The map's strings go too: a different installation may have a different
/// map open, and a stale table would resolve references against the wrong map's text.
pub fn forget() {
    if let Ok(mut slot) = engine_slot().lock() {
        *slot = None;
    }
    set_map_strings(None);
}

/// The open map's string table, for a caller that has a `TRIGSTR_nnn` to resolve.
///
/// `None` when no map is open, or when the open one has no `war3map.wts` — which is different from an
/// empty table and is why this is an `Option` rather than an empty table.
#[must_use]
pub fn map_strings() -> Option<war3_map::StringTable> {
    MAP_STRINGS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

/// Replaces the cached map string table.
fn set_map_strings(strings: Option<war3_map::StringTable>) {
    if let Ok(mut slot) = MAP_STRINGS.get_or_init(|| Mutex::new(None)).lock() {
        *slot = strings;
    }
}

/// Replaces the cached engine.
fn replace(resolver: Resolver) {
    if let Ok(mut slot) = engine_slot().lock() {
        *slot = Some(Arc::new(resolver));
    }
}

/// The resolver, building it on first use.
fn engine() -> Option<Arc<Resolver>> {
    let slot = engine_slot();
    let Ok(mut guard) = slot.lock() else {
        // A poisoned lock means another thread panicked while holding it. Reporting "no names" is
        // the degradation the rest of this file follows; a panic here would take the editor with it.
        return None;
    };
    if guard.is_none() {
        *guard = build().map(Arc::new);
    }
    // The `Arc` is cloned, not the engine: this is once per command, and holding the lock across the
    // whole command would serialize every panel behind the first one to ask.
    guard.clone()
}

/// The slot, created on first use.
fn engine_slot() -> &'static Mutex<Option<Arc<Resolver>>> {
    ENGINE.get_or_init(|| Mutex::new(None))
}

/// Builds an engine, or reports that there is no game to build one from.
fn build() -> Option<Resolver> {
    let settings = settings::load().ok()?;
    let game = settings::find_game(&settings)?;
    let assets = GameAssets::open(&game.dir);
    Some(Resolver::load(&assets))
}

/// The kind a query names, by the core's own spelling.
///
/// Falls back to `Unit` for a word the core does not know: the worst case is that the wrong table is
/// consulted and the ID comes back unchanged, which is what an unrecognised ID shows anyway.
fn parse_kind(name: &str) -> ObjectKind {
    ObjectKind::ALL
        .into_iter()
        .find(|kind| format!("{kind}").eq_ignore_ascii_case(name))
        .unwrap_or(ObjectKind::Unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unknown kind does not panic and does not invent an answer.
    #[test]
    fn an_unrecognised_kind_falls_back_rather_than_failing() {
        assert_eq!(parse_kind("unit"), ObjectKind::Unit);
        assert_eq!(parse_kind("UNIT"), ObjectKind::Unit);
        assert_eq!(parse_kind("ability"), ObjectKind::Ability);
        assert_eq!(parse_kind("nonsense"), ObjectKind::Unit);
    }

    /// ⚠️ With no game installation every ID answers as itself, which is the **same** answer an
    /// object the game has never heard of gets. The interface therefore needs no second code path
    /// for "there is no game" — and that is the point of doing it this way rather than returning an
    /// empty list.
    #[test]
    fn with_no_installation_every_id_stands_for_itself() {
        let answers = resolve(&[NameQuery {
            kind: "unit".to_string(),
            id: "hfoo".to_string(),
        }]);
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].id, "hfoo");
        assert!(!answers[0].name.is_empty(), "an answer always has a name");
        assert!(
            ["map", "game", "id"].contains(&answers[0].source.as_str()),
            "source must be one of the core's three labels, got {}",
            answers[0].source
        );
    }

    /// An empty batch is an empty answer, not a failure.
    #[test]
    fn an_empty_batch_resolves_to_nothing() {
        assert!(resolve(&[]).is_empty());
    }
}
