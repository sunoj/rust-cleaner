// Group planning for popover scan lists: which rows fit within a visible budget.
// Exports: `GroupPlan`, `RowPlan`, `plan_groups`. Pure — no AppKit.
// Deps: crate::names, wd40::{disk, scanner}.

use crate::names::display_names;
use wd40::disk::sum_bytes;
use wd40::scanner::{ArtifactGroup, TargetDir};

/// Rows a non-empty group keeps even when the list is over budget.
const FLOOR_PER_GROUP: usize = 3;

/// One project row, resolved down to what the popover draws.
pub struct RowPlan<'a> {
    pub index: usize,
    pub target: &'a TargetDir,
    pub name: String,
    pub kind: &'static str,
}

/// One artifact group and the rows the popover has room to show for it.
pub struct GroupPlan<'a> {
    pub group: ArtifactGroup,
    pub rows: Vec<RowPlan<'a>>,
    pub count: usize,
    pub hidden: usize,
    pub size: u64,
}

/// Split `targets` into groups and decide which rows fit within `limit`.
pub fn plan_groups(targets: &[TargetDir], limit: usize) -> Vec<GroupPlan<'_>> {
    let names = display_names(targets);
    let members: Vec<(ArtifactGroup, Vec<(usize, &TargetDir)>)> = ArtifactGroup::ALL
        .iter()
        .map(|&group| {
            let found: Vec<(usize, &TargetDir)> = targets
                .iter()
                .enumerate()
                .filter(|(_, td)| td.kind.group() == group)
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
    use super::{allocate, plan_groups};
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
        let plans = plan_groups(&targets, 2);
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
    fn a_group_of_one_kind_leaves_the_kind_suffix_off() {
        let targets = vec![
            target("/w/a/target", ArtifactKind::RustTarget, 9),
            target("/tmp/cc-target-b", ArtifactKind::TmpTarget, 8),
        ];
        let mixed = plan_groups(&targets, 10);
        assert_eq!(mixed[0].rows[0].kind, "target");
        let single = plan_groups(&targets[..1], 10);
        assert_eq!(single[0].rows[0].kind, "");
    }
}
