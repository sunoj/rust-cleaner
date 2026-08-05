// Capacity gauge math for the popover disk header (shared by scan/clean/done).
// Exports: `gauge_fractions`, `GaugeParts`. Pure — no AppKit.
// Deps: wd40::disk::DiskSpace.

use wd40::disk::DiskSpace;

/// Fractions of the bar: other used, reclaimable artifacts, free. Sum ≈ 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaugeParts {
    pub used: f64,
    pub artifacts: f64,
    pub free: f64,
}

/// Split disk capacity into the three header segments the design draws.
pub fn gauge_fractions(disk: DiskSpace, reclaimable: u64) -> GaugeParts {
    let total = disk.total_bytes.max(1) as f64;
    let used = disk.total_bytes.saturating_sub(disk.free_bytes);
    let artifacts = reclaimable.min(used) as f64 / total;
    let other = used.saturating_sub(reclaimable.min(used)) as f64 / total;
    let free = disk.free_bytes as f64 / total;
    // Guarantee a visible artifact sliver when anything is reclaimable.
    let (other, artifacts) = if reclaimable > 0 && artifacts < 0.02 {
        let bump = 0.02 - artifacts;
        ((other - bump).max(0.0), 0.02)
    } else {
        (other, artifacts)
    };
    GaugeParts { used: other, artifacts, free }
}

/// Integer cell split used by unit tests (same semantics as the old menu gauge).
#[cfg(test)]
pub fn gauge_cells(disk: DiskSpace, reclaimable: u64, cells: usize) -> (usize, usize, usize) {
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

#[cfg(test)]
fn share(part: u64, total: u64, cells: usize) -> usize {
    if total == 0 {
        return 0;
    }
    ((part as f64 / total as f64) * cells as f64).round() as usize
}

#[cfg(test)]
mod tests {
    use super::gauge_cells;
    use wd40::disk::DiskSpace;

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
        let (_, artifacts, free_cells) = gauge_cells(disk(900, 1000), 5_000, CELLS);
        assert_eq!(artifacts, 3);
        assert_eq!(free_cells, 27);
    }
}
