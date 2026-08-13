// Group planning for popover scan lists: which rows fit within a visible budget.
// Exports: `GroupPlan`, `RowPlan`, `Membership`, `plan_groups`. Pure — no AppKit.
// Deps: crate::names, wd40::{disk, scanner}.

use crate::names::display_names;
use std::collections::HashSet;
use wd40::disk::sum_bytes;
use wd40::scanner::{ArtifactGroup, TargetDir};

/// Rows a non-empty group keeps even when the list is over budget.
const FLOOR_PER_GROUP: usize = 3;

/// One project row, resolved down to what the popover draws.
pub struct RowPlan<'a> {
    pub index: usize,
    pub target: &'a TargetDir,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub kind: &'static str,
}

/// One artifact group and the rows the popover has room to show for it.
pub struct GroupPlan<'a> {
    pub group: ArtifactGroup,
    pub rows: Vec<RowPlan<'a>>,
    pub count: usize,
    /// Rows this group has but the budget could not fit.
    pub hidden: usize,
    pub size: u64,
}

/// The visible row set and the group counts it was planned from. A later open
/// rebuilds only when this no longer matches — size hops patch labels, not
/// which rows belong on screen.
#[derive(Clone, PartialEq, Eq)]
pub struct Membership {
    rows: Vec<usize>,
    counts: Vec<(ArtifactGroup, usize)>,
}

impl Membership {
    pub fn of(targets: &[TargetDir], empty: &HashSet<usize>, limit: usize) -> Self {
        let plans = plan_groups(targets, empty, limit);
        Self {
            rows: plans.iter().flat_map(|p| p.rows.iter().map(|r| r.index)).collect(),
            counts: plans.iter().map(|p| (p.group, p.count)).collect(),
        }
    }
}

impl std::fmt::Debug for Membership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let counts: Vec<(&str, usize)> = self.counts.iter().map(|(g, n)| (g.key(), *n)).collect();
        f.debug_struct("Membership")
            .field("rows", &self.rows)
            .field("counts", &counts)
            .finish()
    }
}

/// Split `targets` into groups and decide which rows fit within `limit`.
///
/// `empty` is left out of the rows and out of the count with it: a target that
/// was measured and holds nothing frees nothing, so counting it would promise
/// a row the list does not have. The totals are untouched — an empty target
/// contributes no bytes to add or lose.
pub fn plan_groups<'a>(
    targets: &'a [TargetDir],
    empty: &HashSet<usize>,
    limit: usize,
) -> Vec<GroupPlan<'a>> {
    let names = display_names(targets);
    let members: Vec<(ArtifactGroup, Vec<(usize, &TargetDir)>)> = ArtifactGroup::ALL
        .iter()
        .map(|&group| {
            let found: Vec<(usize, &TargetDir)> = targets
                .iter()
                .enumerate()
                .filter(|(index, td)| td.kind.group() == group && !empty.contains(index))
                .collect();
            (group, found)
        })
        .filter(|(_, found)| !found.is_empty())
        .collect();

    let quota = allocate(&members.iter().map(|(_, m)| m.len()).collect::<Vec<_>>(), limit);

    members
        .into_iter()
        .zip(quota)
        .map(|((group, found), visible)| {
            let first_kind = found[0].1.kind.label();
            let mixed = found.iter().any(|(_, td)| td.kind.label() != first_kind);
            let rows: Vec<RowPlan> = found
                .iter()
                .take(visible)
                .map(|&(index, target)| RowPlan {
                    index,
                    target,
                    name: names[index].clone(),
                    kind: if mixed { target.kind.label() } else { "" },
                })
                .collect();
            GroupPlan {
                group,
                count: found.len(),
                hidden: found.len() - rows.len(),
                size: sum_bytes(found.iter().map(|(_, td)| td.size_bytes)),
                rows,
            }
        })
        .collect()
}

fn allocate(sizes: &[usize], limit: usize) -> Vec<usize> {
    let mut quota: Vec<usize> = sizes.iter().map(|&n| n.min(FLOOR_PER_GROUP)).collect();
    let mut spent: usize = quota.iter().sum();
    while spent > limit {
        let Some(slot) = quota.iter_mut().rev().find(|left| **left > 0) else { break };
        *slot -= 1;
        spent -= 1;
    }
    for (index, &size) in sizes.iter().enumerate() {
        let extra = (size - quota[index]).min(limit.saturating_sub(spent));
        quota[index] += extra;
        spent += extra;
    }
    quota
}

#[cfg(test)]
mod tests {
    use super::{allocate, plan_groups, Membership};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use wd40::scanner::{ArtifactKind, TargetDir};

    fn target(path: &str, kind: ArtifactKind, size: u64) -> TargetDir {
        TargetDir {
            path: PathBuf::from(path),
            size_bytes: size,
            last_modified: SystemTime::UNIX_EPOCH,
            kind,
        }
    }

    #[test]
    fn the_row_limit_is_shared_across_groups() {
        let targets = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 9),
            target("/w/b/target", ArtifactKind::RustTarget, 8),
            target("/w/c/node_modules", ArtifactKind::NodeModules, 7),
        ];
        let plans = plan_groups(&targets, &HashSet::new(), 2);
        assert_eq!(plans[0].rows.len(), 2);
        assert_eq!(plans[0].hidden, 0);
        assert_eq!(plans[1].rows.len(), 0);
        assert_eq!(plans[1].hidden, 1);
        assert_eq!(plans[1].count, 1);
    }

    #[test]
    fn a_small_group_keeps_its_floor_against_a_crowded_one() {
        assert_eq!(allocate(&[17, 3], 15), [12, 3]);
    }

    #[test]
    fn allocation_never_hands_out_more_than_the_limit() {
        for sizes in [vec![20], vec![9, 9, 9], vec![1, 1], vec![0, 40, 2]] {
            let quota = allocate(&sizes, 15);
            assert!(quota.iter().sum::<usize>() <= 15, "{sizes:?} -> {quota:?}");
            for (given, &available) in quota.iter().zip(sizes.iter()) {
                assert!(given <= &available, "{sizes:?} -> {quota:?}");
            }
        }
    }

    #[test]
    fn a_target_that_measures_empty_changes_membership() {
        let targets = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 9),
            target("/w/b/target", ArtifactKind::RustTarget, 4),
        ];
        let before = Membership::of(&targets, &HashSet::new(), 10);
        let after = Membership::of(&targets, &HashSet::from([1]), 10);
        assert_ne!(before, after);
        assert_eq!(after, Membership::of(&targets[..1], &HashSet::new(), 10));
    }

    #[test]
    fn membership_ignores_how_old_a_target_is() {
        let now = SystemTime::now();
        let young = TargetDir {
            path: PathBuf::from("/w/a/target"),
            size_bytes: 9,
            last_modified: now,
            kind: ArtifactKind::RustTarget,
        };
        let old = TargetDir {
            last_modified: now
                .checked_sub(std::time::Duration::from_secs(3 * 86_400))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            ..young.clone()
        };
        assert_eq!(
            Membership::of(&[young], &HashSet::new(), 10),
            Membership::of(&[old], &HashSet::new(), 10),
        );
    }

    #[test]
    fn a_nonzero_size_leaves_membership_alone() {
        let unmeasured = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 0),
            target("/w/b/target", ArtifactKind::RustTarget, 8),
        ];
        let settled = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 12),
            target("/w/b/target", ArtifactKind::RustTarget, 8),
        ];
        assert_eq!(
            Membership::of(&unmeasured, &HashSet::new(), 10),
            Membership::of(&settled, &HashSet::new(), 10),
        );
    }

    #[test]
    fn a_hidden_empty_target_still_changes_the_count() {
        let targets: Vec<TargetDir> = (0..7)
            .map(|i| target(&format!("/w/{i}/target"), ArtifactKind::RustTarget, 10 - i as u64))
            .collect();
        let before = Membership::of(&targets, &HashSet::new(), 6);
        let after = Membership::of(&targets, &HashSet::from([6]), 6);
        assert_ne!(before, after);
        assert_eq!(after, Membership::of(&targets[..6], &HashSet::new(), 6));
    }

    /// A row that frees nothing is not a row, and the count says so — the
    /// figure beside the group title is what the list can show, not what the
    /// scan happened to walk past.
    #[test]
    fn an_empty_target_leaves_the_rows_and_the_count() {
        let targets = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 9),
            target("/w/b/target", ArtifactKind::RustTarget, 0),
        ];
        let plans = plan_groups(&targets, &HashSet::from([1]), 10);
        assert_eq!(plans[0].rows.len(), 1);
        assert_eq!(plans[0].rows[0].index, 0);
        assert_eq!(plans[0].count, 1);
        assert_eq!(plans[0].hidden, 0);
        assert_eq!(plans[0].size, 9);
    }

    #[test]
    fn a_group_of_one_kind_leaves_the_kind_suffix_off() {
        let targets = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 9),
            target("/tmp/cc-target-b", ArtifactKind::TmpTarget, 8),
        ];
        let mixed = plan_groups(&targets, &HashSet::new(), 10);
        assert_eq!(mixed[0].rows[0].kind, "target");
        let single = plan_groups(&targets[..1], &HashSet::new(), 10);
        assert_eq!(single[0].rows[0].kind, "");
    }
}
