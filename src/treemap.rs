// Squarified treemap of the targets a clean is working through: one tile per
// target, tile area proportional to its bytes on disk, largest first.
// Exports: `Tile`, tile state constants, `tiles`, `contains`.
// Deps: crate::state, objc2_foundation geometry, wd40::disk.

use crate::state::{CleanItemStatus, CleanProgress};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use wd40::disk::sum_bytes;

/// Past this the tiles stop being readable, so the tail becomes a single tile.
const MAX_TILES: usize = 14;
pub const PENDING: u8 = 0;
pub const ACTIVE: u8 = 1;
pub const DONE: u8 = 2;
pub const SKIPPED: u8 = 3;

pub struct Tile {
    pub rect: NSRect,
    pub name: String,
    pub size: u64,
    pub state: u8,
}

/// The largest targets each get a tile; the rest collapse into one, so a queue
/// of sixty node_modules does not become sixty unreadable slivers.
pub fn tiles(p: &CleanProgress, width: f64, height: f64) -> Vec<Tile> {
    let mut order: Vec<usize> = (0..p.items.len()).collect();
    order.sort_by(|a, b| p.items[*b].size_bytes.cmp(&p.items[*a].size_bytes));
    let head = order.len().min(MAX_TILES);
    let mut sizes: Vec<f64> = Vec::new();
    let mut meta: Vec<(String, u64, u8)> = Vec::new();
    for &index in &order[..head] {
        let item = &p.items[index];
        sizes.push(item.size_bytes.max(1) as f64);
        meta.push((item.name.clone(), item.size_bytes, tile_state(item.status)));
    }
    if let Some(tail) = Some(&order[head..]).filter(|tail| !tail.is_empty()) {
        let bytes = sum_bytes(tail.iter().map(|i| p.items[*i].size_bytes));
        let states: Vec<u8> = tail.iter().map(|i| tile_state(p.items[*i].status)).collect();
        let state = match (states.iter().all(|s| *s == DONE), states.iter().any(|s| *s != PENDING)) {
            (true, _) => DONE,
            (false, true) => ACTIVE,
            _ => PENDING,
        };
        sizes.push(bytes.max(1) as f64);
        meta.push((format!("{} smaller targets", tail.len()), bytes, state));
    }
    squarify(&sizes, width, height)
        .into_iter()
        .zip(meta)
        .map(|(rect, (name, size, state))| Tile { rect, name, size, state })
        .collect()
}

fn tile_state(status: CleanItemStatus) -> u8 {
    match status {
        CleanItemStatus::Pending => PENDING,
        CleanItemStatus::Active => ACTIVE,
        CleanItemStatus::Done => DONE,
        CleanItemStatus::Skipped => SKIPPED,
    }
}

pub fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

/// Squarify (Bruls et al.): fill the short side with a row of tiles for as long
/// as adding one improves the worst aspect ratio, then recurse on what is left.
fn squarify(sizes: &[f64], width: f64, height: f64) -> Vec<NSRect> {
    let mut out: Vec<NSRect> = Vec::with_capacity(sizes.len());
    let (mut x, mut y, mut w, mut h) = (0.0_f64, 0.0_f64, width, height);
    let mut left: f64 = sizes.iter().sum();
    let mut i = 0;
    while i < sizes.len() && left > 0.0 && w > 0.5 && h > 0.5 {
        let vertical = w >= h;
        let side = if vertical { h } else { w };
        let scale = w * h / left;
        let mut n = 1;
        while i + n < sizes.len()
            && worst(&sizes[i..i + n + 1], side, scale) <= worst(&sizes[i..i + n], side, scale)
        {
            n += 1;
        }
        let row = &sizes[i..i + n];
        let sum: f64 = row.iter().sum();
        let thick = sum * scale / side;
        let mut off = 0.0;
        for value in row {
            let len = value * scale / thick;
            out.push(match vertical {
                true => NSRect::new(NSPoint::new(x, y + off), NSSize::new(thick, len)),
                false => NSRect::new(NSPoint::new(x + off, y), NSSize::new(len, thick)),
            });
            off += len;
        }
        if vertical {
            (x, w) = (x + thick, w - thick);
        } else {
            (y, h) = (y + thick, h - thick);
        }
        left -= sum;
        i += n;
    }
    out.resize(sizes.len(), NSRect::new(NSPoint::new(x, y), NSSize::new(0.0, 0.0)));
    out
}

fn worst(row: &[f64], side: f64, scale: f64) -> f64 {
    let sum: f64 = row.iter().sum::<f64>() * scale;
    let max = row.iter().copied().fold(f64::MIN, f64::max) * scale;
    let min = row.iter().copied().fold(f64::MAX, f64::min) * scale;
    if sum <= 0.0 || min <= 0.0 || side <= 0.0 {
        return f64::MAX;
    }
    (side * side * max / (sum * sum)).max(sum * sum / (side * side * min))
}

#[cfg(test)]
mod tests {
    use super::squarify;

    #[test]
    fn every_tile_keeps_its_share_of_the_area() {
        let sizes = [50.0, 25.0, 15.0, 6.0, 4.0];
        let rects = squarify(&sizes, 300.0, 200.0);
        let total: f64 = sizes.iter().sum();
        for (size, rect) in sizes.iter().zip(&rects) {
            let area = rect.size.width * rect.size.height;
            assert!((area - size / total * 300.0 * 200.0).abs() < 1.0, "{area}");
        }
    }

    #[test]
    fn tiles_stay_inside_the_plate() {
        let rects = squarify(&[9.0, 4.0, 2.0, 1.0], 120.0, 80.0);
        for rect in rects {
            assert!(rect.origin.x >= -0.01 && rect.origin.x + rect.size.width <= 120.01);
            assert!(rect.origin.y >= -0.01 && rect.origin.y + rect.size.height <= 80.01);
        }
    }
}
