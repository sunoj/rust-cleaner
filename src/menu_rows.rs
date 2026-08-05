// Project rows for the WD-40 menu: grouping, row layout, and the hover path row.
// Exports: `GroupPlan`, `RowPlan`, `plan_groups`, `widest_label`, `project_row`, `path_row`.
// Deps: objc2_app_kit, crate::{names, style}, wd40::{disk, scanner}.

use crate::names::{age, display_names, display_path};
use crate::style::{fit_width, text_width, Columns, Row, MAX_NAME_WIDTH};
use objc2_app_kit::{NSColor, NSFont};
use wd40::disk::sum_bytes;
use wd40::scanner::{human_size, ArtifactGroup, TargetDir};

/// Blocks in a project's relative-size bar.
const MAX_BAR: usize = 8;
/// Rows a non-empty group keeps even when the menu is over budget, so a small
/// group is never reduced to a header and "3 more not shown".
const FLOOR_PER_GROUP: usize = 3;

/// One project row, resolved down to what the menu draws.
pub struct RowPlan<'a> {
    /// Index into `AppState::targets`, carried as the menu item's tag.
    pub index: usize,
    pub target: &'a TargetDir,
    pub name: String,
    /// Kind suffix; empty unless the group mixes kinds.
    pub kind: &'static str,
}

/// One artifact group and the rows the menu has room to show for it.
pub struct GroupPlan<'a> {
    pub group: ArtifactGroup,
    pub rows: Vec<RowPlan<'a>>,
    pub count: usize,
    pub hidden: usize,
    pub size: u64,
}

/// Split `targets` into groups and decide which rows fit within `limit`.
/// Pure enough to test: no AppKit, no measurement.
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
            // The kind only adds information where a group mixes kinds (target
            // vs tmp-target); elsewhere it repeats the header and steals width.
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

/// Hand out `limit` rows across groups: a floor for each group first, then the
/// remainder in order, so the biggest group cannot crowd the others out.
fn allocate(sizes: &[usize], limit: usize) -> Vec<usize> {
    let mut quota: Vec<usize> = sizes.iter().map(|&n| n.min(FLOOR_PER_GROUP)).collect();
    let mut spent: usize = quota.iter().sum();
    // The floors alone can exceed a small limit; give back from the last group.
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

/// Widest label the menu will draw, in points — what the name column sizes to.
pub fn widest_label(plans: &[GroupPlan], font: &NSFont) -> f64 {
    plans
        .iter()
        .flat_map(|plan| plan.rows.iter())
        .map(|row| text_width(&row.name, font) + kind_width(row, font))
        .fold(0.0_f64, f64::max)
}

/// Width of a full project row, in points — what the disk gauge spans so the
/// two halves of the menu share one right edge.
pub fn row_width(columns: Columns, font: &NSFont) -> f64 {
    columns.bar_start() + text_width(&"\u{2588}".repeat(MAX_BAR), font)
}

pub fn project_row(row: &RowPlan, max_size: u64, sizing: bool, font: &NSFont) -> Row {
    let mut out = Row::new();
    // Name and kind share one column; overflowing it would shove the size past
    // its tab stop and wrap the bar onto a second line.
    let budget = MAX_NAME_WIDTH - kind_width(row, font);
    out.push(&fit_width(&row.name, font, budget), None);
    if !row.kind.is_empty() {
        out.push(&format!("  {}", row.kind), Some(NSColor::tertiaryLabelColor()));
    }
    out.tab();
    if sizing {
        out.push("\u{2026}", Some(NSColor::tertiaryLabelColor()));
        return out;
    }
    out.push(&human_size(row.target.size_bytes), Some(NSColor::secondaryLabelColor()));
    out.tab();
    let ratio = row.target.size_bytes as f64 / max_size.max(1) as f64;
    let filled = (ratio * MAX_BAR as f64).ceil().max(1.0) as usize;
    out.push(&"\u{2588}".repeat(filled.min(MAX_BAR)), Some(NSColor::tertiaryLabelColor()));
    out
}

/// What a row shows while the pointer is on it: the full path the short name
/// stands for, and how stale the artifacts are. Given the whole row width,
/// since it replaces the size and bar columns for as long as it is up.
pub fn path_row(target: &TargetDir, font: &NSFont, width: f64) -> Row {
    let detail = format!(" \u{2022} {}", age(target.last_modified));
    let path = fit_width(
        &display_path(&target.path),
        font,
        width - text_width(&detail, font),
    );
    // The artifact directory itself is the part worth reading; the tree it sits
    // in is context.
    let split = path.rfind('/').map(|at| at + 1).unwrap_or(0);
    let mut row = Row::new();
    row.push(&path[..split], Some(NSColor::secondaryLabelColor()));
    row.push(&path[split..], None);
    row.push(&detail, Some(NSColor::tertiaryLabelColor()));
    row
}

fn kind_width(row: &RowPlan, font: &NSFont) -> f64 {
    if row.kind.is_empty() {
        0.0
    } else {
        text_width(&format!("  {}", row.kind), font)
    }
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
        // 17 Rust targets must not push a 3-entry group down to nothing.
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
