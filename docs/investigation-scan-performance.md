# Scan-performance investigation

## Decision

**Recommendation: keep `RustTarget` and `TmpTarget` uncached, and make the
physical pass cheaper or less eager. Do not ship a Rust-target cache based on
any signal tested here.** No candidate provides the required proof that a
cached physical size cannot be stale. The existing `Duration::ZERO` policy is
the only option in this set that preserves that safety property for arbitrary
writes ([`src/cache.rs:24-31`](../src/cache.rs#L24-L31)).

The useful optimization boundary is the physical extent pass: keep the cheap
logical size scan responsive, then defer or parallelize extent reads within a
bounded I/O budget. A cache could be safe only if every writer participates in
a trusted generation protocol and WD-40 treats missing, dropped, or
unverifiable generations as a cache miss. Cargo targets do not expose such a
protocol in this codebase.

## Questions investigated

1. Whether target markers, mtimes, fingerprint directories/files, or FSEvents
   are complete change signals.
2. Whether `content_modified` is already that signal and what it costs.
3. Where the scan spends time: walk, metadata, open, and
   `F_LOG2PHYS_EXT`.
4. Whether a non-caching design is cheaper and what adjacent correctness risks
   deserve follow-up.

## Evidence and controls

The repository was clean at the start and remains free of production-code
changes. The supplied 0.59 s / 2.33 s warm full-scan result was not repeated;
the prompt already establishes it, and repeating it would add noisy evidence
without isolating a cause. The real-target microbenchmarks below were serial,
warm-cache runs against one target. I could not inspect peer-build state:
`ps -axo pid=,comm=,args=` returned `operation not permitted`, and
`pgrep -af rustc` reported `sysmond service not found`. Therefore these numbers
do not claim to control the machine-wide peer-build load.

Build-event experiments were isolated from peer sessions in disposable Cargo
projects under `/var/folders/.../aid-build-target/...`, with
`CARGO_DISK_GUARD_MIN_FREE_GB=3`. The shared `CARGO_TARGET_DIR` was not used
for writes. The experiment command was:

```sh
export CARGO_TARGET_DIR="$TMPDIR/aid-build-target/<experiment>/target"
export CARGO_DISK_GUARD_MIN_FREE_GB=3
cargo new --quiet --bin "$TMPDIR/aid-build-target/<experiment>/project"
cargo build --manifest-path "$TMPDIR/aid-build-target/<experiment>/project/Cargo.toml"
```

The signal snapshots used macOS `stat -f '%m'`, `find`, and deliberate 1.2 s
gaps between writes. The 1.2 s gap avoids hiding a result behind the cache's
seconds conversion; it does not make timestamp equality a correctness proof
([`src/cache.rs:209-215`](../src/cache.rs#L209-L215)).

## 1. Candidate change signals

### Controlled observations

The first disposable target was built, then a file below `target/debug/deps`
was touched, rewritten at the same length, created at 8 MiB, and an existing
deep file was grown from 1 MiB to 8 MiB. The observed mtime values were:

| Event | target root | `.rustc_info.json` | `CACHEDIR.TAG` | `debug` | newest immediate child of `debug` | `.fingerprint` dir | newest file below target |
|---|---:|---:|---:|---:|---:|---:|---:|
| Initial build | 1786690336 | 1786690336 | 1786690334 | 1786690336 | 1786690336 | 1786690334 | 1786690336 |
| Touch deep existing file | unchanged | unchanged | unchanged | unchanged | unchanged | unchanged | 1786690412 |
| Rewrite deep file, same size | unchanged | unchanged | unchanged | unchanged | unchanged | unchanged | 1786690413 |
| Add deep 8 MiB file | unchanged | unchanged | unchanged | 1786690416 | 1786690414 | unchanged | 1786690414 |
| Grow existing deep file to 8 MiB | unchanged | unchanged | unchanged | unchanged | unchanged | unchanged | 1786690664 |

The last row is the decisive false-negative: the target's physical/logical
content grew, but the root, `debug`, immediate-child, and fingerprint-directory
signals did not move. The full descendant-newest signal did move. The output
came from the command sequence `touch`, `dd ... conv=notrunc`, `dd ... count=8`,
and the `stat`/`find` snapshot described above; the temporary files were
deleted with the experiment.

A clean Cargo event experiment gave this second table:

| Event | target root | `.rustc_info.json` | `CACHEDIR.TAG` | `debug` | `.fingerprint` dir | newest fingerprint file | newest file below target |
|---|---:|---:|---:|---:|---:|---:|---:|
| First build | 1786690457 | 1786690457 | 1786690457 | 1786690457 | 1786690457 | 1786690457 | 1786690457 |
| Touch source, rebuild | unchanged | unchanged | unchanged | 1786690459 | unchanged | 1786690459 | 1786690459 |
| No-op Cargo build | unchanged | unchanged | unchanged | 1786690460 | unchanged | unchanged | unchanged |

The no-op build changing `debug` is a false positive in this run: its lock-file
activity changed the directory mtime, so this signal would miss no work but
would reduce cache hits. Cargo's target layout is consequently not a target
content generation counter ([Cargo build command](https://doc.rust-lang.org/cargo/commands/cargo-build.html)).

### Candidate assessment

| Candidate | Moves on every tested size-changing write? | False-unchanged result | Check cost on the real target | Confidence and conclusion |
|---|---|---|---|---|
| `target/.rustc_info.json` | No | Deep add/grow and Cargo rebuild left it unchanged | One metadata check; marker pair averaged 2.66–2.82 µs | **Verified** incomplete; Cargo/rustc bookkeeping, not output state |
| `target/CACHEDIR.TAG` | No | Never moved after creation | Included in the same 2-file marker check | **Verified** static cache tag, not a change signal. Its convention identifies cache content; it does not promise mutation tracking ([specification](https://bford.info/cachedir/)) |
| mtime of target root | No | All deep writes left it unchanged | One metadata check averaged 0.61–0.71 µs | **Verified** incomplete; this is exactly the current bug in `measurement_of` ([`src/cache.rs:76-87`](../src/cache.rs#L76-L87)) |
| mtime of `target/{debug,release}` | No | Deep in-place rewrite/grow left `debug` unchanged | One or two metadata checks | **Verified** incomplete; direct child creation and Cargo rebuild moved it, and a no-op build also moved it |
| newest mtime among immediate children of `debug`/`release` | No | In-place deep grow left the parent `deps` mtime unchanged | Branch/fingerprint/immediate-child metadata averaged 21.7–23.1 µs | **Verified** incomplete; catches entry creation/removal, not writes to an existing deep file |
| `target/*/.fingerprint` directory mtime | No | Deep writes and Cargo rebuild left the directory unchanged | Part of the branch check | **Verified** incomplete; the newest fingerprint *file* moved on source rebuild, but unrelated target writes did not |
| newest mtime among all descendants | Yes for touch, same-size rewrite, deep add, and deep grow | No false negative in this experiment | Not cheap: it requires a complete descendant walk. WD-40's existing bulk walk took 14.6–19.1 ms on this target | **Observed but not sufficient**; mtime can be restored, and cache persistence truncates to seconds |
| FSEvents | An event is a useful invalidation hint, not a state value | Absence of an event cannot prove unchanged content without a complete, loss-free stream and fallback rescan | No local client was added; it requires a stream, run-loop, event-ID persistence, and drop/overflow handling | **Verified from Apple documentation** unsuitable as the sole cache proof: events are directory-granular, coalesced, and may require a recursive rescan ([Apple guide](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/TechnologyOverview/TechnologyOverview.html), [event flags](https://developer.apple.com/documentation/coreservices/1455361/fseventstreameventflags/kfseventstreameventflagmustscansubdirs)) |

The Apple guide explicitly says FSEvents is not fine-grained file-change
notification, and recommends kqueue for a particular file. It also documents
latency and temporal coalescing in `FSEventStreamCreate`; those properties make
it a good wake-up/invalidation mechanism, not a proof that no write occurred
([`FSEventStreamCreate`](https://developer.apple.com/documentation/coreservices/1443980/fseventstreamcreate?language=objc),
[`Kernel Queues`](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/KernelQueues/KernelQueues.html)).

## 2. `content_modified`

The relevant signatures are:

```rust
// src/cache.rs:76
pub fn measurement_of(path: &Path, kind: ArtifactKind) -> Option<(u64, SystemTime)>

// src/cache.rs:90
pub fn put_measurement(path: &Path, kind: ArtifactKind, bytes: u64,
                       content_modified: SystemTime)

// src/sizes.rs:136
pub fn measure_dir_with_modified(path: &Path) -> Measurement
```

`content_modified` is conceptually the candidate in question 1: the newest
descendant timestamp. It is recorded in `Sized_`, returned by
`measurement_of`, and then ignored by the freshness predicate. Freshness uses
only `entry.modified == modified` (the target directory's own mtime) plus TTL
([`src/cache.rs:52-60`](../src/cache.rs#L52-L60),
[`src/cache.rs:81-87`](../src/cache.rs#L81-L87)). For Rust targets there is no
stored measurement at all: `ttl` is zero and `put_measurement` returns before
metadata or storage ([`src/cache.rs:24-31`](../src/cache.rs#L24-L31),
[`src/cache.rs:90-109`](../src/cache.rs#L90-L109)). So `content_modified` is
not currently a usable Rust-target cache key.

Computationally, it is cheaper than physical extents but not free. The normal
size walk already requests `modified_time`, `size`, and `alloc_size` through
`getattrlistbulk` for every directory entry, folds the maximum timestamp, and
returns it with the size ([`src/sizes.rs:20-28`](../src/sizes.rs#L20-L28),
[`src/sizes.rs:135-155`](../src/sizes.rs#L135-L155),
[`src/sizes.rs:165-190`](../src/sizes.rs#L165-L190)). Computing it as a
pre-check therefore costs approximately the logical walk it is intended to
save. On the real target, that existing bulk pass measured 14.6–19.1 ms.

The safety problem is separate from cost. `std::fs::File::set_modified` can
change a file's mtime, and this cache converts `SystemTime` to whole seconds;
a writer can therefore rewrite bytes and restore the observed timestamp, or
two changes can collapse into one stored second ([Rust `File::set_modified`](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.set_modified),
[`src/cache.rs:209-211`](../src/cache.rs#L209-L211)). A newest-mtime equality is
not a guarantee that bytes or physical extents are unchanged.

## 3. Where the time goes

The physical path has these actual signatures and stages:

```rust
// src/extents.rs:97
pub fn read_target(root: &Path) -> TargetExtents

// src/extents.rs:134
fn collect_root(root: &Path, owner: u32, refs: &mut Vec<Ref>, unmapped: &mut u64)

// src/extents.rs:152
fn file_extents(path: &Path, size: u64, owner: u32, refs: &mut Vec<Ref>)
```

`collect_root` walks with `WalkDir`, calls `entry.metadata()` for each entry,
opens each non-empty file, and `file_extents` calls `fcntl(F_LOG2PHYS_EXT)` at
each extent boundary ([`src/extents.rs:129-169`](../src/extents.rs#L129-L169)).
The macOS SDK header and man page define that ioctl as an in/out
`struct log2phys`: the input is file offset and requested length; the output
is contiguous allocated bytes and device offset ([SDK `sys/fcntl.h`](file:///Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk/System/Library/Frameworks/Kernel.framework/Versions/A/Headers/sys/fcntl.h),
[SDK `fcntl(2)`](file:///Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk/usr/share/man/man2/fcntl.2)). It is not literally one call per
file when a file is fragmented: the source loops, and the harness observed
35,179 calls for 2,110 non-empty files, or 16.7 calls per file.

### Stage decomposition

The temporary harness ran the four equivalent stages serially on
`/Users/mingsun/.cargo-target/wd-40/research-scan-perf`:

```text
entries: 2,309; non-empty files: 2,110; logical bytes: 918,178,185
walk only:       4.6–7.7 ms (warm)
walk + metadata: 7.5–7.9 ms (warm; first run 102.3 ms)
walk + open:    28.9–30.2 ms
walk + extents: 43.3–45.3 ms; 35,179 fcntl calls
```

Derived per-file figures are approximately 3.7 µs for the metadata stage,
13.8 µs for open, and 20.6 µs for the complete extent stage. The incremental
extent portion is about 14.4 ms for this run, or 0.41 µs per fcntl. These are
warm-cache, one-target observations; they are useful for attribution, not a
machine-wide multiplier. The current bulk logical pass measured separately at
14.6–19.1 ms because it uses `getattrlistbulk`, not the harness's per-entry
`WalkDir` metadata path.

The logical walk is already bounded and parallel: `size_targets` starts up to
`qos::workers()` threads, and `workers()` is capped at 6
([`src/sizes.rs:47-97`](../src/sizes.rs#L47-L97),
[`src/qos.rs:39-50`](../src/qos.rs#L39-L50)). The physical extent pass is not
parallel at the target-list level: `Reclaim::measure` maps
`read_target` sequentially before calling `combine`
([`src/reclaim.rs:37-66`](../src/reclaim.rs#L37-L66)). The supplied
`sys > real` result is consistent with parallel logical workers, but process
inspection was unavailable and the full run was not repeated, so it cannot
identify the exact phase by itself.

## 4. What was missed and options

### Option A — keep the current Rust-target no-cache policy (recommended)

* **Safety:** Guaranteed with respect to this cache: Rust targets never use a
  remembered size. A target is measured before its figure is accepted; no
  candidate signal is trusted as a proof.
* **Cost:** Pays the physical pass whenever `Reclaim::measure` cannot reuse its
  other caches. No production change is proposed in this report.
* **Where:** [`src/cache.rs:24-31`](../src/cache.rs#L24-L31), with deferred
  reclaim at [`src/tasks.rs:168-193`](../src/tasks.rs#L168-L193).
* **Trade-off:** The 14.6 GB / 29-target machine pays for work dominated by
  Rust targets, as described in the prompt; the UI can continue showing the
  logical sum until physical reclaim completes, which the current task flow
  already does ([`src/tasks.rs:173-176`](../src/tasks.rs#L173-L176)).

### Option B — defer physical extents unless the user needs reclaimable bytes

* **Safety:** Guaranteed for the default logical figure if it is labelled as
  allocated/logical size; physical reclaim remains a fresh pass. Do not call a
  deferred logical sum a physical union.
* **Cost:** Removes the extent pass from the common scan path; adds latency only
  when the user opens a reclaimable-bytes view or starts a clean action.
* **Where:** The existing boundary is [`src/tasks.rs:168-193`](../src/tasks.rs#L168-L193);
  the physical API is [`src/reclaim.rs:37-66`](../src/reclaim.rs#L37-L66).
* **Trade-off:** UI semantics need a clear “logical size” versus “reclaimable
  physical bytes” distinction. This is the simplest speed win that does not
  weaken correctness.

### Option C — bounded parallelism for `read_target`

* **Safety:** Preserved: every target still gets a fresh extent read, and
  `combine` still unions the resulting maps. The only change is scheduling.
* **Cost:** Lower wall time is plausible because target reads are independent,
  but total syscalls and storage pressure do not fall. Memory rises with
  simultaneously retained extent vectors; too many workers compete with
  `rustc`, so reuse the existing 2–6 worker cap and background QoS.
* **Where:** [`src/reclaim.rs:53-64`](../src/reclaim.rs#L53-L64),
  [`src/extents.rs:95-108`](../src/extents.rs#L95-L108),
  [`src/qos.rs:20-50`](../src/qos.rs#L20-L50).
* **Evidence status:** Recommended as a benchmarkable implementation option,
  not claimed as a measured 3x improvement. Peer-build interference was not
  observable on this machine.

### Option D — cache on root/deep mtimes, fingerprints, or FSEvents

* **Safety:** Not acceptable. The controlled false negatives above prove the
  shallow signals do not cover in-place deep writes. Descendant mtime and
  FSEvents add detection coverage but do not prove unchanged bytes, and an
  FSEvents stream must handle coalescing, drops, event IDs, and rescan flags.
* **Cost:** Root/marker checks are microseconds; descendant checks cost a full
  walk; FSEvents adds a persistent watcher and still needs a correctness
  fallback. None justifies weakening the safety rule.
* **Where:** Existing root-only freshness is [`src/cache.rs:81-87`](../src/cache.rs#L81-L87);
  existing extent reuse is [`src/extent_cache.rs:28-49`](../src/extent_cache.rs#L28-L49).

### Adjacent correctness finding — physical caches are already weaker than size TTL

`Reclaim::measure` first accepts a six-hour attribution cache keyed by only
`(path, bytes)` ([`src/reclaim.rs:43-47`](../src/reclaim.rs#L43-L47),
[`src/cache.rs:124-131`](../src/cache.rs#L124-L131)). On a miss,
`extent_cache::extents_of` accepts a per-target extent map when only the target
root mtime and total bytes match ([`src/extent_cache.rs:20-39`](../src/extent_cache.rs#L20-L39)). A same-size deep rewrite is not proven to
invalidate either cache. On APFS, a rewrite can change copy-on-write physical
extents even when the logical size is unchanged; this should be audited before
any claim that the physical union is never stale. This report does not change
that production behavior.

## Findings summary

* **[FINDING] No shallow Cargo target signal is complete.** Root mtime,
  `.rustc_info.json`, `CACHEDIR.TAG`, branch mtime, immediate-child newest
  mtime, and fingerprint-directory mtime all missed a deep in-place growth
  event in the controlled experiment. **Confidence: Verified** for the tested
  filesystem and operations ([`src/extents.rs:129-169`](../src/extents.rs#L129-L169),
  experiment output above).
* **[FINDING] `content_modified` is the right shape but not a proof.** It is
  returned but not used for freshness, costs a descendant walk when computed
  independently, is persisted at one-second precision, and can be restored by
  a writer. **Confidence: Verified** ([`src/cache.rs:52-60`](../src/cache.rs#L52-L60),
  [`src/cache.rs:81-87`](../src/cache.rs#L81-L87)).
* **[FINDING] The measured target's warm physical pass is dominated by open,
  extent, and per-entry filesystem work—not the directory enumeration alone.**
  The harness observed 2,110 files, 35,179 extent calls, and 43.3–45.3 ms for
  the full extent stage. **Confidence: Verified locally, uncertain as a
  machine-wide extrapolation.**
* **[FINDING] The best safe speed path is scheduling/deferment, not cache
  invalidation.** Bounded parallel fresh extent reads and user-triggered
  physical accounting preserve the no-stale-number guarantee. **Confidence:
  Likely design recommendation; benchmark before implementation.**
* **[FINDING] Physical attribution caching needs a separate safety audit.** Its
  path+size and root-mtime+size keys are weaker than the disabled Rust size TTL.
  **Confidence: Verified from code; physical consequence depends on APFS
  copy-on-write behavior.**

## Sources

* Local implementation: [`src/cache.rs`](../src/cache.rs),
  [`src/extents.rs`](../src/extents.rs), [`src/sizes.rs`](../src/sizes.rs),
  [`src/reclaim.rs`](../src/reclaim.rs), [`src/extent_cache.rs`](../src/extent_cache.rs),
  [`src/tasks.rs`](../src/tasks.rs), [`src/qos.rs`](../src/qos.rs).
* Apple, [File System Events Programming Guide](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/TechnologyOverview/TechnologyOverview.html),
  [Using the FSEvents API](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/UsingtheFSEventsFramework/UsingtheFSEventsFramework.html),
  and [Kernel Queues](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/KernelQueues/KernelQueues.html).
* Apple, [`FSEventStreamCreate`](https://developer.apple.com/documentation/coreservices/1443980/fseventstreamcreate?language=objc)
  and [`kFSEventStreamEventFlagMustScanSubDirs`](https://developer.apple.com/documentation/coreservices/1455361/fseventstreameventflags/kfseventstreameventflagmustscansubdirs).
* Rust, [`File::set_modified`](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.set_modified)
  and [`cargo build`](https://doc.rust-lang.org/cargo/commands/cargo-build.html).
* Bryan Ford, [Cache Directory Tagging Specification](https://bford.info/cachedir/).
* Local macOS SDK: `/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk/System/Library/Frameworks/Kernel.framework/Versions/A/Headers/sys/fcntl.h` and `/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk/usr/share/man/man2/fcntl.2`.
