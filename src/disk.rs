// Disk stats utilities for WD-40.
// Exports: `DiskSpace`, `disk_space`, `sum_bytes`.
// Deps: libc, std::os::unix::ffi::OsStrExt.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

pub fn disk_space(path: &Path) -> Option<DiskSpace> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
    let rc = unsafe { libc::statfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }

    let stats = unsafe { stats.assume_init() };
    let block_size = u64::try_from(stats.f_bsize).ok()?;
    let free_blocks = u64::try_from(stats.f_bavail).ok()?;
    let total_blocks = u64::try_from(stats.f_blocks).ok()?;

    Some(DiskSpace {
        free_bytes: block_size.saturating_mul(free_blocks),
        total_bytes: block_size.saturating_mul(total_blocks),
    })
}

pub fn sum_bytes<I>(values: I) -> u64
where
    I: IntoIterator<Item = u64>,
{
    values
        .into_iter()
        .fold(0_u64, |total, value| total.saturating_add(value))
}

#[cfg(test)]
mod tests {
    use super::{disk_space, sum_bytes};
    use std::path::Path;

    #[test]
    fn disk_space_reports_current_volume() {
        let stats = disk_space(Path::new(".")).expect("disk stats");
        assert!(stats.total_bytes > 0);
        assert!(stats.free_bytes <= stats.total_bytes);
    }

    #[test]
    fn sum_bytes_saturates_on_overflow() {
        let total = sum_bytes([u64::MAX - 5, 10]);
        assert_eq!(total, u64::MAX);
    }
}
