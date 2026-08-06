// Physical extents on APFS: which device bytes a file's contents actually sit
// on. `aid` seeds session target dirs with `cp -Rc`, and a clone shares its
// source's extents, so summing allocated sizes counts the same device bytes
// once per copy. Unioning extents counts them once, which is what deleting
// them returns. See docs/apfs-clone-overcount.md.
// Exports: `Attribution`, `attribute`.
// Deps: std, libc, walkdir.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

/// `struct log2phys` from <sys/fcntl.h>. For `F_LOG2PHYS_EXT` both `off`
/// fields are in/out: bytes into the file going in, bytes into the device
/// coming back.
///
/// The header wraps this in `#pragma pack(4)`, so it is 20 bytes with the
/// offsets at 4 and 12 — not the 24 that natural alignment would give. A plain
/// `repr(C)` here reads the length field off the end of the offset and returns
/// numbers in the exabytes.
#[repr(C, packed(4))]
struct Log2Phys {
    flags: libc::c_uint,
    contigbytes: libc::off_t,
    devoffset: libc::off_t,
}

/// One reference to a run of device bytes, and which root made it.
struct Ref {
    offset: u64,
    len: u64,
    owner: u32,
}

/// What a set of roots occupies on the device, split so that a later question
/// about any subset can be answered without walking the disk again.
pub struct Attribution {
    /// Device bytes only this root refers to. Freed whenever it is removed.
    exclusive: Vec<u64>,
    /// Device bytes shared by exactly this set of roots. Freed only when every
    /// one of them goes.
    shared: Vec<(Vec<u32>, u64)>,
}

impl Attribution {
    /// Bytes the whole set occupies — every shared run counted once.
    pub fn total(&self) -> u64 {
        let exclusive: u64 = self.exclusive.iter().copied().fold(0, u64::saturating_add);
        self.shared
            .iter()
            .map(|(_, bytes)| *bytes)
            .fold(exclusive, u64::saturating_add)
    }

    /// Bytes removing exactly `selected` would return. A shared run counts only
    /// when every root holding it is going, which is the whole point: deleting
    /// one clone of a pair frees nothing the other still refers to.
    pub fn union_of(&self, selected: &[bool]) -> u64 {
        let mut total: u64 = 0;
        for (root, bytes) in self.exclusive.iter().enumerate() {
            if selected.get(root).copied().unwrap_or(false) {
                total = total.saturating_add(*bytes);
            }
        }
        for (owners, bytes) in &self.shared {
            let all_going = owners
                .iter()
                .all(|owner| selected.get(*owner as usize).copied().unwrap_or(false));
            if all_going {
                total = total.saturating_add(*bytes);
            }
        }
        total
    }

    /// What this root alone accounts for — its own bytes plus its share of
    /// nothing. Used to rank rows without promising the shared part twice.
    pub fn exclusive(&self, root: usize) -> u64 {
        self.exclusive.get(root).copied().unwrap_or(0)
    }
}

/// Read the extent map of every file under `roots` and work out who holds what.
pub fn attribute(roots: &[PathBuf]) -> Attribution {
    // One `open` and a few `fcntl`s per file is a lot of syscalls; take them at
    // a priority that yields the disk to whatever the user is running.
    crate::qos::background();
    let mut refs: Vec<Ref> = Vec::new();
    let mut exclusive = vec![0_u64; roots.len()];
    for (owner, root) in roots.iter().enumerate() {
        collect_root(root, owner as u32, &mut refs, &mut exclusive[owner]);
    }
    let shared = sweep(&mut refs, &mut exclusive);
    Attribution { exclusive, shared }
}

/// Walk one root, mapping each file. A file whose extents cannot be read at all
/// (a compressed file keeps its data in an xattr, and has none) falls back to
/// its allocated size, credited to this root alone — losing it would quietly
/// shrink the total, and undercounting what a delete returns is the one
/// direction this must not fail in.
fn collect_root(root: &Path, owner: u32, refs: &mut Vec<Ref>, unmapped: &mut u64) {
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() || meta.size() == 0 {
            continue;
        }
        let before = refs.len();
        file_extents(entry.path(), meta.size(), owner, refs);
        if refs.len() == before {
            *unmapped = unmapped.saturating_add(meta.blocks().saturating_mul(512));
        }
    }
}

fn file_extents(path: &Path, size: u64, owner: u32, refs: &mut Vec<Ref>) {
    let Ok(file) = std::fs::File::open(path) else { return };
    let mut pos: u64 = 0;
    while pos < size {
        let mut query = Log2Phys {
            flags: 0,
            contigbytes: (size - pos) as libc::off_t,
            devoffset: pos as libc::off_t,
        };
        let code = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_LOG2PHYS_EXT, &mut query) };
        // Copied out by value: a reference into a packed struct is not allowed.
        let (length, device) = (query.contigbytes, query.devoffset);
        if code < 0 || length <= 0 || device < 0 {
            return;
        }
        refs.push(Ref { offset: device as u64, len: length as u64, owner });
        pos = pos.saturating_add(length as u64);
    }
}

/// Sweep the device from low offset to high. Between one boundary and the next
/// the set of roots covering those bytes does not change, so each span is
/// credited once — to a single root, or to the set that shares it.
fn sweep(refs: &mut [Ref], exclusive: &mut [u64]) -> Vec<(Vec<u32>, u64)> {
    let mut events: Vec<(u64, u32, bool)> = Vec::with_capacity(refs.len() * 2);
    for reference in refs.iter() {
        events.push((reference.offset, reference.owner, true));
        events.push((reference.offset.saturating_add(reference.len), reference.owner, false));
    }
    // Ends before starts at the same offset, so a run that stops exactly where
    // the next begins is not read as an overlap.
    events.sort_unstable_by_key(|(at, owner, opening)| (*at, *opening, *owner));

    let mut active: HashMap<u32, usize> = HashMap::new();
    let mut shared: HashMap<Vec<u32>, u64> = HashMap::new();
    let mut cursor = events.first().map(|(at, _, _)| *at).unwrap_or(0);
    for (at, owner, opening) in events {
        if at > cursor && !active.is_empty() {
            credit(&active, at - cursor, exclusive, &mut shared);
        }
        cursor = cursor.max(at);
        match opening {
            true => *active.entry(owner).or_insert(0) += 1,
            false => {
                if let Some(count) = active.get_mut(&owner) {
                    *count -= 1;
                    if *count == 0 {
                        active.remove(&owner);
                    }
                }
            }
        }
    }
    shared.into_iter().collect()
}

fn credit(
    active: &HashMap<u32, usize>,
    bytes: u64,
    exclusive: &mut [u64],
    shared: &mut HashMap<Vec<u32>, u64>,
) {
    if active.len() == 1 {
        let owner = *active.keys().next().unwrap_or(&0) as usize;
        if let Some(slot) = exclusive.get_mut(owner) {
            *slot = slot.saturating_add(bytes);
        }
        return;
    }
    let mut owners: Vec<u32> = active.keys().copied().collect();
    owners.sort_unstable();
    let slot = shared.entry(owners).or_insert(0);
    *slot = slot.saturating_add(bytes);
}

#[cfg(test)]
mod tests {
    use super::attribute;
    use std::path::PathBuf;
    use std::process::Command;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("wd40-extents-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    /// `cp -c` is a clone on APFS and an ordinary copy anywhere else, so a test
    /// that depends on sharing has to check it got one.
    fn clone_file(from: &PathBuf, to: &PathBuf) -> bool {
        Command::new("cp").arg("-c").arg(from).arg(to).status().is_ok_and(|s| s.success())
    }

    fn tree(root: &PathBuf, name: &str, bytes: usize) -> PathBuf {
        let dir = root.join(name);
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("payload.bin");
        let _ = std::fs::write(&file, vec![b'x'; bytes]);
        file
    }

    /// The whole point: a clone must not be counted twice, and an independent
    /// copy must be.
    #[test]
    fn a_clone_is_counted_once_and_a_copy_twice() {
        let root = scratch("clone");
        let source = tree(&root, "source", 4 << 20);
        let cloned = root.join("cloned");
        let copied = root.join("copied");
        let _ = std::fs::create_dir_all(&cloned);
        let _ = std::fs::create_dir_all(&copied);
        if !clone_file(&source, &cloned.join("payload.bin")) {
            eprintln!("skipped: cp -c unavailable");
            return;
        }
        // Not `std::fs::copy`: on macOS it goes through `fcopyfile` with
        // COPYFILE_CLONE and produces a clone, which is the very thing this
        // file is meant to be the opposite of. Write the bytes again instead.
        let _ = std::fs::write(copied.join("payload.bin"), vec![b'y'; 4 << 20]);

        let roots = vec![root.join("source"), cloned.clone(), copied.clone()];
        let found = attribute(&roots);
        let four_mb = (4 << 20) as u64;

        // Three roots holding 4 MB each, but only two distinct copies exist.
        let total = found.total();
        assert!(
            total >= four_mb * 2 && total < four_mb * 3,
            "clone must not add a third copy: {total}"
        );

        // Removing the clone alone frees nothing — the source still holds those
        // bytes. Removing the independent copy frees all of its own.
        assert_eq!(found.union_of(&[false, true, false]), 0);
        assert!(found.union_of(&[false, false, true]) >= four_mb);

        // Removing source and clone together frees the bytes they share.
        assert!(found.union_of(&[true, true, false]) >= four_mb);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_absent_root_contributes_nothing() {
        let found = attribute(&[scratch("absent")]);
        assert_eq!(found.total(), 0);
        assert_eq!(found.exclusive(0), 0);
    }
}

