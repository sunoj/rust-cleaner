// Per-item clean selection: nothing starts checked; the user opts in.
// Exports: `default_selection`, `selected_bytes`, `is_recent`.
// Deps: std, wd40::scanner::TargetDir.

use std::collections::HashSet;
use std::time::{Duration, SystemTime};
use wd40::scanner::{ArtifactGroup, TargetDir};

const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GroupSelection {
    Off,
    Mixed,
    On,
}

/// A single row is only ever on or off; only a group can be part-ticked.
impl From<bool> for GroupSelection {
    fn from(on: bool) -> Self {
        match on {
            true => Self::On,
            false => Self::Off,
        }
    }
}

/// True when the artifact is newer than `max_age_days` (still "recent").
pub fn is_recent(target: &TargetDir, max_age_days: u64) -> bool {
    let max_age = Duration::from_secs(max_age_days.saturating_mul(SECONDS_PER_DAY));
    SystemTime::now()
        .duration_since(target.last_modified)
        .map(|age| age < max_age)
        .unwrap_or(false)
}

/// Nothing is checked after a scan. Deleting here is irreversible and skips the
/// Trash, so a popover that opens already armed to delete is not a default
/// anyone should be handed — the user opts in, per row or per group.
pub fn default_selection(_targets: &[TargetDir], _max_age_days: u64) -> HashSet<usize> {
    HashSet::new()
}

pub fn selected_bytes(targets: &[TargetDir], selected: &HashSet<usize>) -> u64 {
    selected
        .iter()
        .filter_map(|&i| targets.get(i).map(|td| td.size_bytes))
        .fold(0_u64, |sum, n| sum.saturating_add(n))
}

pub fn group_selection(
    targets: &[TargetDir],
    selected: &HashSet<usize>,
    group: ArtifactGroup,
) -> GroupSelection {
    let indices = targets.iter().enumerate().filter(|(_, target)| target.kind.group() == group);
    let (count, selected_count) = indices.fold((0, 0), |(count, selected_count), (index, _)| {
        (count + 1, selected_count + usize::from(selected.contains(&index)))
    });
    match selected_count {
        0 => GroupSelection::Off,
        selected_count if selected_count == count => GroupSelection::On,
        _ => GroupSelection::Mixed,
    }
}

pub fn toggle_group(targets: &[TargetDir], selected: &mut HashSet<usize>, group: ArtifactGroup) {
    let indices: Vec<usize> = targets
        .iter()
        .enumerate()
        .filter(|(_, target)| target.kind.group() == group)
        .map(|(index, _)| index)
        .collect();
    let clear = indices.iter().all(|index| selected.contains(index));
    for index in indices {
        if clear {
            selected.remove(&index);
        } else {
            selected.insert(index);
        }
    }
}

#[allow(dead_code)]
pub fn age_days(target: &TargetDir) -> u64 {
    SystemTime::now()
        .duration_since(target.last_modified)
        .map(|d| d.as_secs() / SECONDS_PER_DAY)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{default_selection, group_selection, is_recent, toggle_group, GroupSelection};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};
    use wd40::scanner::{ArtifactGroup, ArtifactKind, TargetDir};

    fn target(age_days: u64) -> TargetDir {
        TargetDir {
            path: PathBuf::from("/tmp/cc-target-x"),
            size_bytes: 10,
            last_modified: SystemTime::now() - Duration::from_secs(age_days * 86_400),
            kind: ArtifactKind::TmpTarget,
        }
    }

    #[test]
    fn nothing_is_armed_to_delete_by_default() {
        // Age still drives the "recent" badge, but it must not pre-arm a row:
        // the popover opens with an empty selection whatever the ages are.
        let targets = vec![target(1), target(10)];
        assert!(default_selection(&targets, 3).is_empty());
        assert!(is_recent(&targets[0], 3));
        assert!(!is_recent(&targets[1], 3));
    }

    #[test]
    fn group_toggle_selects_all_then_clears_all() {
        let targets = vec![target(1), target(10)];
        let mut selected = HashSet::from([0]);
        assert!(group_selection(&targets, &selected, ArtifactGroup::Rust) == GroupSelection::Mixed);
        toggle_group(&targets, &mut selected, ArtifactGroup::Rust);
        assert!(group_selection(&targets, &selected, ArtifactGroup::Rust) == GroupSelection::On);
        toggle_group(&targets, &mut selected, ArtifactGroup::Rust);
        assert!(group_selection(&targets, &selected, ArtifactGroup::Rust) == GroupSelection::Off);
    }
}
