// The disk module at the top of the WD-40 menu: free space as the headline
// number, a capacity gauge, and the slice of it WD-40 can hand back.
// Exports: `DiskPanel`, `add_disk_panel`.
// Deps: objc2_app_kit, objc2_foundation, crate::{menu, style}, wd40.

use crate::menu::new_item;
use crate::style::{caption_font, gauge_font, headline_font, menu_font, text_width, Columns, Row};
use objc2_app_kit::{NSColor, NSMenu};
use objc2_foundation::{MainThreadMarker, NSString};
use objc2::rc::Retained;
use wd40::disk::DiskSpace;
use wd40::scanner::human_size;

const BLOCK: &str = "\u{2588}";
/// Narrowest gauge worth drawing, in blocks.
const MIN_CELLS: usize = 12;
/// Below this share of free space the headline turns orange, then red.
const LOW_FREE: f64 = 0.15;
const CRITICAL_FREE: f64 = 0.05;

/// Everything the panel reports, resolved before any drawing.
pub struct DiskPanel {
    pub disk: Option<DiskSpace>,
    pub reclaimable: u64,
    /// Sizes are still being computed, so `reclaimable` is not final.
    pub sizing: bool,
    /// APFS clones let sizes overlap, so `reclaimable` is an upper bound.
    pub approximate: bool,
}

/// Draw the panel `width` points wide — the width of one project row, so the
/// gauge and the rows below it share a right edge.
pub fn add_disk_panel(menu: &NSMenu, panel: &DiskPanel, width: f64, mtm: MainThreadMarker) {
    let gauge = gauge_font();
    let cell = text_width(BLOCK, &gauge).max(1.0);
    let cells = ((width / cell).round() as usize).max(MIN_CELLS);
    // The panel's right edge is the gauge's, whatever the font metrics turn out to be.
    let columns = Columns::ending_at(cell * cells as f64);

    let Some(disk) = panel.disk else {
        add_row(menu, artifact_row(panel, None), &menu_font(), columns, mtm);
        return;
    };

    add_row(menu, free_row(disk), &headline_font(), columns, mtm);
    add_row(menu, gauge_row(disk, panel, cells), &gauge, columns, mtm);
    if panel.sizing || panel.reclaimable > 0 {
        add_row(menu, artifact_row(panel, Some(disk)), &menu_font(), columns, mtm);
    }
}

/// `23.4G free` — the number the whole menu exists to move.
fn free_row(disk: DiskSpace) -> Row {
    let mut row = Row::new();
    row.push(&format!("{} free", human_size(disk.free_bytes)), Some(pressure_color(disk)));
    row.tab();
    row.push_styled(
        &format!("of {}", human_size(disk.total_bytes)),
        Some(NSColor::secondaryLabelColor()),
        Some(caption_font()),
    );
    row
}

/// Used space, split so the reclaimable slice is visible inside it.
fn gauge_row(disk: DiskSpace, panel: &DiskPanel, cells: usize) -> Row {
    let reclaimable = if panel.sizing { 0 } else { panel.reclaimable };
    let (used, artifacts, free) = gauge_cells(disk, reclaimable, cells);
    let mut row = Row::new();
    row.push(&BLOCK.repeat(used), Some(gray(0.62)));
    row.push(&BLOCK.repeat(artifacts), Some(artifact_color()));
    row.push(&BLOCK.repeat(free), Some(gray(0.16)));
    row
}

/// The orange slice, named — and what freeing it would leave.
fn artifact_row(panel: &DiskPanel, disk: Option<DiskSpace>) -> Row {
    let mut row = Row::new();
    if panel.sizing {
        row.push("measuring build artifacts\u{2026}", Some(NSColor::secondaryLabelColor()));
        return row;
    }

    let bound = if panel.approximate { "up to " } else { "" };
    row.push(
        &format!("{bound}{} in build artifacts", human_size(panel.reclaimable)),
        Some(artifact_color()),
    );
    if let Some(disk) = disk {
        let after = disk.free_bytes.saturating_add(panel.reclaimable).min(disk.total_bytes);
        row.tab();
        row.push_styled(
            &format!("\u{2192} {} free", human_size(after)),
            Some(NSColor::secondaryLabelColor()),
            Some(caption_font()),
        );
    }
    row
}

/// Rust orange: the same signal the menu bar icon uses for artifact weight.
fn artifact_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.87, 0.47, 0.18, 1.0)
}

/// Mid gray at a given opacity. Alpha over the menu's own material reads the
/// same in light and dark, which a semantic label color would not.
fn gray(alpha: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.5, 0.5, 0.5, alpha)
}

fn pressure_color(disk: DiskSpace) -> Retained<NSColor> {
    let free = disk.free_bytes as f64 / disk.total_bytes.max(1) as f64;
    if free < CRITICAL_FREE {
        NSColor::systemRedColor()
    } else if free < LOW_FREE {
        NSColor::systemOrangeColor()
    } else {
        NSColor::labelColor()
    }
}

/// Split `cells` blocks into (other used, artifacts, free). Any artifact space
/// at all claims a block, so the slice the menu talks about is never invisible.
fn gauge_cells(disk: DiskSpace, reclaimable: u64, cells: usize) -> (usize, usize, usize) {
    let used = disk.total_bytes.saturating_sub(disk.free_bytes);
    let artifacts = reclaimable.min(used);
    let mut artifact_cells = share(artifacts, disk.total_bytes, cells);
    if artifacts > 0 {
        artifact_cells = artifact_cells.max(1);
    }
    artifact_cells = artifact_cells.min(cells);
    let used_cells = share(used.saturating_sub(artifacts), disk.total_bytes, cells)
        .min(cells - artifact_cells);
    (used_cells, artifact_cells, cells - used_cells - artifact_cells)
}

fn share(part: u64, total: u64, cells: usize) -> usize {
    if total == 0 {
        return 0;
    }
    ((part as f64 / total as f64) * cells as f64).round() as usize
}

fn add_row(
    menu: &NSMenu,
    row: Row,
    font: &objc2_app_kit::NSFont,
    columns: Columns,
    mtm: MainThreadMarker,
) {
    let item = new_item(&NSString::from_str(""), None, mtm);
    item.setAttributedTitle(Some(&row.build(font, columns)));
    item.setEnabled(false);
    menu.addItem(&item);
}

#[cfg(test)]
mod tests {
    use super::{gauge_cells, DiskSpace};

    const CELLS: usize = 30;

    fn disk(free: u64, total: u64) -> DiskSpace {
        DiskSpace { free_bytes: free, total_bytes: total }
    }

    #[test]
    fn cells_always_add_up_to_the_gauge_width() {
        for (free, total, reclaimable) in
            [(0, 100, 0), (100, 100, 0), (23, 228, 4), (1, 3, 2), (50, 100, 90), (0, 0, 0)]
        {
            let (used, artifacts, free_cells) = gauge_cells(disk(free, total), reclaimable, CELLS);
            assert_eq!(used + artifacts + free_cells, CELLS, "{free}/{total}");
        }
    }

    #[test]
    fn a_sliver_of_artifacts_still_claims_a_block() {
        let (_, artifacts, _) = gauge_cells(disk(500, 1000), 1, CELLS);
        assert_eq!(artifacts, 1);
    }

    #[test]
    fn artifacts_cannot_exceed_used_space() {
        // A reclaimable figure larger than what is used (overlapping clones)
        // must not eat the free portion of the gauge.
        let (_, artifacts, free_cells) = gauge_cells(disk(900, 1000), 5_000, CELLS);
        assert_eq!(artifacts, 3);
        assert_eq!(free_cells, 27);
    }
}
