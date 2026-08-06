// The rust crust: how much of the plate it may cover for a given job, how it is
// painted, and how it reads underneath. Kept apart from the plate view because
// this is the part that has to be true to scale and true to the job — the
// legend quotes the same figures this sizes the region from.
// Exports: `crust_region`, `paint_plate`, `plate_edge`, `legend_text`, `phase`.
// Deps: crate::{header, metal, spray, state, treemap, widgets}, objc2 AppKit.

use crate::header::scan_size;
use crate::metal::{brushed, inset, outline, rings, rnd};
use crate::spray::draw_film;
use crate::state::CleanProgress;
use crate::treemap::{rust_tone, Tile, ACTIVE, BLOCKED, DONE, PART_GONE, SKIPPED};
use crate::widgets::CONTENT_WIDTH;
use objc2_app_kit::{NSBezierPath, NSColor};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use wd40::disk::DiskSpace;

/// Height of the plate the crust is measured against.
pub const PLATE_H: f64 = 190.0;
/// Smallest crust worth drawing: below this a tile cannot be seen or hit.
const MIN_CRUST: f64 = 26.0;

/// How a tile's state reads in the legend.
pub fn phase(state: u8) -> &'static str {
    match state {
        DONE => "removed",
        ACTIVE => "removing",
        SKIPPED => "left in place",
        PART_GONE => "partly removed \u{2014} the rest is still there",
        BLOCKED => "could not be removed",
        _ => "waiting",
    }
}

/// How much of the plate the crust may cover: the share of the whole volume
/// these targets occupy, so a nearly clean disk shows a nearly clean plate. The
/// region keeps the plate's aspect ratio and hangs off the top-left corner.
///
/// Below `MIN_CRUST` the true region would be too small to see or point at, so
/// it stops being to scale — the legend still carries the exact figure. With no
/// volume size to divide by we do not invent one: the crust fills the plate and
/// the legend says the drawing is not to scale.
pub fn crust_region(job_bytes: u64, disk: Option<DiskSpace>) -> (NSRect, String) {
    let plate = NSSize::new(CONTENT_WIDTH, PLATE_H);
    let full = NSRect::new(NSPoint::new(0.0, 0.0), plate);
    let Some(total) = disk.map(|d| d.total_bytes).filter(|total| *total > 0) else {
        return (full, "volume size unknown \u{2014} not to scale".to_string());
    };
    let fraction = (job_bytes as f64 / total as f64).clamp(0.0, 1.0);
    let scale = fraction.sqrt();
    let w = (plate.width * scale).clamp(MIN_CRUST, plate.width);
    let h = (plate.height * scale).clamp(MIN_CRUST * plate.height / plate.width, plate.height);
    (
        NSRect::new(NSPoint::new(0.0, plate.height - h), NSSize::new(w, h)),
        format!("{} of the volume is crust", share_text(fraction)),
    )
}

fn share_text(fraction: f64) -> String {
    match fraction * 100.0 {
        pct if pct >= 10.0 => format!("{pct:.0}%"),
        pct if pct >= 1.0 => format!("{pct:.1}%"),
        pct if pct >= 0.01 => format!("{pct:.2}%"),
        _ => "under 0.01%".to_string(),
    }
}

/// Everything under the spray: brushed steel, its turning marks, the residue
/// film over ground that is really clear, and a patch of crust for every target
/// still on disk.
pub fn paint_plate(bounds: NSRect, dark: bool, tiles: &[Tile]) {
    NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, 10.0, 10.0).addClip();
    brushed(bounds, dark);
    let centre = NSPoint::new(bounds.size.width / 2.0, bounds.size.height / 2.0);
    rings(centre, bounds.size.height * 0.44, dark);
    draw_film(bounds, tiles);
    for (index, tile) in tiles.iter().enumerate() {
        draw_tile(index, tile);
    }
}

/// The plate's edge, drawn last so nothing thrown across it paints over it.
pub fn plate_edge(bounds: NSRect) {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.11, 0.1, 0.09, 0.16).setStroke();
    let edge = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(inset(bounds, 0.5), 9.5, 9.5);
    edge.setLineWidth(1.0);
    edge.stroke();
}

/// What the line under the plate says: the tile the pointer is on, or the run's
/// own figures when it is on none of them.
pub fn legend_text(tiles: &[Tile], hover: isize, idle: &str) -> String {
    match usize::try_from(hover).ok().and_then(|index| tiles.get(index)) {
        Some(tile) => {
            format!("{} \u{00b7} {} \u{00b7} {}", tile.name, scan_size(tile.size), phase(tile.state))
        }
        None => idle.to_string(),
    }
}

/// The same line with nothing under the pointer: the share of the volume this
/// job is, and how much of it has really gone.
pub fn idle_text(share: &str, progress: &CleanProgress) -> String {
    format!("{share} \u{00b7} {}/{} clear", progress.done_count, progress.total_count)
}

/// Crust still on disk. Cleared tiles are drawn by the residue film instead.
pub fn draw_tile(index: usize, tile: &Tile) {
    if tile.state == DONE || tile.rect.size.width < 0.5 {
        return;
    }
    // Skipped, partly removed and blocked tiles are all still on the plate,
    // and are dimmed so they do not read as work still queued.
    let alpha = if tile.state == ACTIVE || tile.state == 0 { 1.0 } else { 0.5 };
    let (r, g, b) = rust_tone(index);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, alpha).setFill();
    NSBezierPath::fillRect(tile.rect);
    let mut seed = 0x9e37_79b9_u32 ^ (index as u32).wrapping_mul(2_654_435_761);
    let pits = ((tile.rect.size.width * tile.rect.size.height / 220.0) as usize).clamp(6, 70);
    for _ in 0..pits {
        let cx = tile.rect.origin.x + rnd(&mut seed) * tile.rect.size.width;
        let cy = tile.rect.origin.y + rnd(&mut seed) * tile.rect.size.height;
        let (pr, pg, pb) = if rnd(&mut seed) > 0.5 { (0.769, 0.529, 0.31) } else { (0.29, 0.173, 0.106) };
        NSColor::colorWithSRGBRed_green_blue_alpha(pr, pg, pb, (0.1 + rnd(&mut seed) * 0.26) * alpha).setFill();
        crate::metal::circle_path(NSPoint::new(cx, cy), 1.5 + rnd(&mut seed) * 5.5).fill();
    }
    NSColor::colorWithSRGBRed_green_blue_alpha(0.118, 0.067, 0.035, 0.55 * alpha).setStroke();
    outline(inset(tile.rect, 0.5), 1.0);
    if tile.state == ACTIVE {
        NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.94, 0.86, 0.75).setStroke();
        outline(inset(tile.rect, 1.5), 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{crust_region, share_text, MIN_CRUST, PLATE_H};
    use crate::widgets::CONTENT_WIDTH;
    use wd40::disk::DiskSpace;

    fn disk(total: u64) -> Option<DiskSpace> {
        Some(DiskSpace { free_bytes: total / 2, total_bytes: total })
    }

    #[test]
    fn the_crust_covers_the_share_of_the_volume_it_occupies() {
        let (quarter, share) = crust_region(250, disk(1000));
        let area = quarter.size.width * quarter.size.height;
        assert!((area / (CONTENT_WIDTH * PLATE_H) - 0.25).abs() < 0.01);
        assert_eq!(share, "25% of the volume is crust");
    }

    #[test]
    fn a_speck_of_crust_still_gets_a_hittable_patch() {
        let (tiny, share) = crust_region(1, disk(10_000_000));
        assert!(tiny.size.width >= MIN_CRUST);
        assert_eq!(share, "under 0.01% of the volume is crust");
    }

    #[test]
    fn without_a_volume_size_the_crust_fills_the_plate_and_says_so() {
        let (full, share) = crust_region(500, None);
        assert_eq!(full.size.width, CONTENT_WIDTH);
        assert!(share.contains("not to scale"));
    }

    #[test]
    fn share_text_keeps_small_fractions_readable() {
        assert_eq!(share_text(0.061), "6.1%");
        assert_eq!(share_text(0.0004), "0.04%");
    }
}
