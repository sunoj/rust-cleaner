# Physical extent-pass cost and safe-skipping investigation

## Decision

**No metadata-only rule examined here can safely exclude every file that might
share a block.** APFS’s content identifier is useful for finding positive clone
candidates, but unequal identifiers do not prove that no blocks remain shared
after a partial clone rewrite. Treating a negative candidate result as
“unshared” would overstate reclaimable bytes.

The recommendation is to make the physical number opt-in or post-action. Keep
the fast logical/allocated sum visible, label it as such, and use the existing
free-space delta after cleaning as the factual result. When a pre-clean physical
union is explicitly requested, run a fresh bounded extent pass. Do not add a
heuristic skip or extend the existing physical caches.

This follows the two required main investigations:
[physical-cache staleness](investigation-physical-cache-staleness.md) reproduced
a stale union after a same-size APFS clone rewrite, and
[scan performance](investigation-scan-performance.md) found that timestamps do
not prove unchanged physical extents.

## Questions investigated

1. Whether size, mtime, inode, hard-link, sparse-file, or APFS content-ID rules
   can exclude files without missing a shared extent.
2. What the current physical pass costs after bounded parallelism, including
   logical bulk metadata and unioning.
3. Whether macOS exposes a bulk, per-file, or early-exit extent API cheaper than
   repeated fcntl(F_LOG2PHYS_EXT) calls.
4. Whether changing the accounting promise can make the pass unnecessary.

## Current call path and correctness boundary

The relevant signatures are:

~~~rust
// src/sizes.rs:47, 134
pub fn size_targets(targets: &[TargetDir], on_size: impl Fn(SizedTarget) + Sync)
pub fn measure_dir_with_modified(path: &Path) -> Measurement

// src/extents.rs:97, 111
pub fn read_target(root: &Path) -> TargetExtents
pub fn combine(per_target: &[TargetExtents]) -> Attribution

// src/reclaim.rs:37, 101
pub fn Reclaim::measure(targets: &[TargetDir]) -> Option<Self>
fn read_targets(targets: &[TargetDir]) -> Vec<crate::extents::TargetExtents>
~~~

size_targets already asks getattrlistbulk for name, object type, logical size,
allocated size, and modified time ([src/sizes.rs:18-28](../src/sizes.rs#L18-L28));
the directory reader folds those values into the logical pass
([src/sizes.rs:165-190](../src/sizes.rs#L165-L190)). That metadata does not
identify physical sharing.

read_target walks every non-empty regular file, opens it, and loops until the
logical range is consumed ([src/extents.rs:129-169](../src/extents.rs#L129-L169)).
combine sorts physical start/end events and credits each device span to the set
of distinct target owners ([src/extents.rs:172-225](../src/extents.rs#L172-L225)).
The current pool reads targets independently and combines them after all reads
finish ([src/reclaim.rs:101-151](../src/reclaim.rs#L101-L151)).

The app selects whole target directories, not individual files. Two clones
inside one target are therefore counted once by the same-owner sweep and do not
change that target's union total. Sharing between selected targets does change
the answer. Omitting the other end counts blocks as reclaimable while an
unselected target still holds them; see [src/reclaim.rs:25-36](../src/reclaim.rs#L25-L36).

## 1. Candidate rules and failure cases

### Observed APFS fixture

The fixture was created on the root APFS volume:

~~~sh
REPRO=$(mktemp -d /private/tmp/wd40-extent-rules.XXXXXX)
mkdir -p "$REPRO/a" "$REPRO/b" "$REPRO/inside"
dd if=/dev/zero of="$REPRO/a/source.bin" bs=1m count=8
cp -c "$REPRO/a/source.bin" "$REPRO/b/clone.bin"
cp -c "$REPRO/a/source.bin" "$REPRO/inside/clone.bin"
ln "$REPRO/a/source.bin" "$REPRO/b/hardlink.bin"
truncate -s 16m "$REPRO/a/sparse.bin"
dd if=/dev/zero of="$REPRO/a/sparse.bin" bs=4096 count=1 seek=1024 conv=notrunc
dd if=/dev/zero of="$REPRO/b/clone.bin" bs=1m count=1 seek=3 conv=notrunc
touch -r "$REPRO/a/source.bin" "$REPRO/b/clone.bin"
mount | rg ' on / '
stat -f 'path=%N inode=%i links=%l logical=%z blocks=%b mtime=%m' \
  "$REPRO/a/source.bin" "$REPRO/b/clone.bin" \
  "$REPRO/inside/clone.bin" "$REPRO/b/hardlink.bin" "$REPRO/a/sparse.bin"
du -h "$REPRO/a/source.bin" "$REPRO/b/clone.bin" \
  "$REPRO/inside/clone.bin" "$REPRO/b/hardlink.bin" "$REPRO/a/sparse.bin"
~~~

Observed output, with the temporary path shortened:

~~~text
/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)
source.bin  inode=386418346 links=2 logical=8388608  blocks=16384 mtime=1786697900
b/clone.bin inode=386418347 links=1 logical=8388608  blocks=16384 mtime=1786697900
inside/clone.bin inode=386418348 links=1 logical=8388608 blocks=16384 mtime=1786697900
b/hardlink.bin inode=386418346 links=2 logical=8388608  blocks=16384 mtime=1786697900
sparse.bin inode=386418351 links=1 logical=16777216 blocks=32    mtime=1786697900

8.0M source.bin
8.0M b/clone.bin
8.0M inside/clone.bin
8.0M b/hardlink.bin
16K sparse.bin
~~~

The inside clone was then touched without changing data:

~~~text
source.bin       inode=386418346 logical=8388608 mtime=1786697900
inside/clone.bin inode=386418348 logical=8388608 mtime=1786697985
source.bin       content=386418346 mayShare=true
inside/clone.bin content=386418346 mayShare=true
~~~

The content-ID command was:

~~~sh
swift - "$REPRO/a/source.bin" "$REPRO/inside/clone.bin" <<'SWIFT'
import Foundation
let keys: Set<URLResourceKey> = [.fileContentIdentifierKey, .mayShareFileContentKey]
for path in CommandLine.arguments.dropFirst() {
    let values = try URL(fileURLWithPath: path).resourceValues(forKeys: keys)
    print("path=\(path) content=\(String(describing: values.fileContentIdentifier)) mayShare=\(String(describing: values.mayShareFileContent))")
}
SWIFT
~~~

Apple documents NSURLFileContentIdentifierKey as an APFS identifier for a
file's content data stream and says only a clone and its original can have the
same identifier ([Apple Foundation URL resource key](https://developer.apple.com/documentation/foundation/urlresourcekey/filecontentidentifierkey?language=objc)).
The observation supports equal IDs as likely clone pairs. It does not support
unequal IDs as a skip rule: the partially rewritten b/clone.bin changed to a
new content ID in the same fixture, while Apple's APFS documentation says
writes are placed elsewhere and unmodified blocks continue to be shared
([About Apple File System](https://developer.apple.com/documentation/foundation/about-apple-file-system?changes=_7)).

### Rule assessment

| Rule | Breaker | Error if skipped |
|---|---|---|
| Matching logical size | Truncate or append to a clone. A shared prefix can remain while sizes differ. | Missed share; **more reclaimable bytes** than deletion returns. |
| Matching allocated size | Rewrite a clone in place at the same length, or change logical length while allocation stays equal. | Missed shared ranges; **over-report**. |
| Matching mtime | Touch a still-shared clone, or restore its mtime after a write. | Missed share; **over-report**. Equality can also hide changed extents. |
| Matching inode/file ID | APFS clone/source have distinct inodes (386418346 vs 386418348) but can share blocks. | Missed clone share; **over-report**. |
| Inode as hard-link shortcut | Hard links have one inode, but the same inode may be represented in two selected targets. Reuse is safe only if every owner receives a reference. | Dropped ownership makes blocks look exclusive; **over-report**. |
| APFS content identifier | A partial clone rewrite changes the clone ID while unmodified blocks remain shared. | ID mismatch misses residual sharing; **over-report**. Equal IDs are positive hints only. |
| Sparse-file exclusion | A 16 MiB sparse file used 16 KiB. Sparse clones can retain allocated shared runs after partial writes. | Skipping every sparse file misses allocated shared runs; **over-report**. |
| Same-target-only matching | A clone can be inside one target. It does not affect the current whole-target union, but would affect future file-level deletion. | No current whole-target error; future file-level accounting could **over-report**. |

Apple defines file ID as the volume-local file-system object identifier
([getattrlist(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getattrlist.2.html)).
The hard-link consequence follows the owner-counting map in
[src/extents.rs:185-200](../src/extents.rs#L185-L200). The current implementation
already skips zero-length files at [src/extents.rs:140-143](../src/extents.rs#L140-L143);
a file with provably zero allocated bytes is the only useful trivial safe skip.

**[FINDING] No useful negative candidate rule has the required guarantee.**
Every ordinary metadata rule has a constructed false negative, and the error
direction is unsafe over-reporting.

## 2. Where the current time goes

### Measurement method

The project check used the warm shared target:

~~~text
$ aid build
succeeded: 0 errors, 0 warnings; command: cargo check; elapsed: 76ms
~~~

The measurement harness was disposable, used a path dependency, and built under
a scratch target with the disk guard:

~~~sh
CARGO_TARGET_DIR="$TMPDIR/aid-build-target/extent-cost.u31Ep3"
CARGO_DISK_GUARD_MIN_FREE_GB=3 HOME=/Users/mingsun \
  cargo run --quiet --manifest-path .research-extent-bench/Cargo.toml
~~~

It discovered 25 targets: 7 Rust targets, one build directory, developer
caches, empty simulator-cache roots, the Cargo registry, and one toolchain.
The output began:

~~~text
discovery targets=25 wall_ms=73.912
target kind=build   /Users/mingsun/Develop/mac/wd-40/dist
target kind=target  /Users/mingsun/.cargo-target/wd-40/research-extent-cost
target kind=target  /Users/mingsun/.cargo-target/ai-dispatch/fix-fallback-target-gc
target kind=target  /Users/mingsun/.cargo-target/ai-dispatch/fix-probe-timeout-model-validation
target kind=target  /Users/mingsun/.cargo-target/ai-dispatch/_base
target kind=target  /Users/mingsun/.cargo-target/poolstrade-compounder
target kind=target  /Users/mingsun/.cargo-target/poolstrade-compounder/investigate-pricing-staleness
target kind=target  /Users/mingsun/.cargo-target/poolstrade-compounder/x86_64-unknown-linux-gnu
target kind=cache   /Users/mingsun/Library/Caches/org.swift.swiftpm
...
target kind=toolchain /Users/mingsun/.rustup/toolchains/1.95.0-aarch64-apple-darwin
~~~

Peer builds changed the live workload while the target count stayed 25: it
grew from 229,875 to 236,109 entries and from 211,623 to 217,851 non-empty
files. Timings below therefore name the run rather than claim a stable machine
benchmark.

### Results

The later, background-QoS run produced:

~~~text
bulk_getattrlistbulk workers=6 wall_ms=752.725 entries=25 allocated_bytes=43273715712
app_size_targets workers=6 wall_ms=1532.220 settled=25
physical_trace workers=1 wall_ms=16729.056 entries=236109 nonempty=217851 open_calls=217851 fcntl_calls=514702 walk_ms=1520.028 metadata_ms=3038.172 open_ms=10404.868 fcntl_ms=1231.555
physical_trace workers=2 wall_ms=10300.174 ...
physical_trace workers=4 wall_ms=6927.611 ...
physical_trace workers=6 wall_ms=6481.921 ... walk_ms=2277.808 metadata_ms=3514.808 open_ms=16080.682 fcntl_ms=1354.192
combine_sweep runs=514702 wall_ms=739.348
app_extent_pool workers=6 read_wall_ms=6124.427 combine_wall_ms=852.866
~~~

Stage counters are summed across worker threads, not wall time. They attribute
16.08 s of aggregate open work, 1.35 s of fcntl work, 3.51 s of metadata work,
and 2.28 s of walk-iterator work. The approximate per-call figures were 74 us
per open and 2.6 us per fcntl; timing instrumentation adds a small cost.

**[FINDING] The money is now open plus filesystem traversal and background I/O
contention, not the fcntl loop alone.** The exact public pass measured 6.12 s
for the six-worker read pool and 0.85 s for combine. One worker took 16.73 s;
two took 10.30 s; four took 6.93 s; six took 6.48 s. Six workers help, but do
not scale linearly.

The direct bulk metadata pass was 0.75–1.71 s across the two runs, and the
application size_targets pass was 1.53–1.63 s. The earlier 19 ms logical result
was a different, much smaller workload, not this live 25-target set.

## 3. API alternatives and early exit

The local SDK gives the relevant signatures and layout:

~~~c
int fcntl(int fildes, int cmd, ...);

#pragma pack(4)
struct log2phys {
    unsigned int l2p_flags;
    off_t l2p_contigbytes; /* IN queried bytes; OUT contiguous bytes */
    off_t l2p_devoffset;   /* IN file offset; OUT device offset */
};
#pragma pack()
~~~

F_LOG2PHYS returns the device address for the current file offset;
F_LOG2PHYS_EXT is the in/out range form. The macOS SDK documents no bulk
“return this file's complete map” command ([fcntl(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html),
[local sys/fcntl.h](file:///Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/Frameworks/Kernel.framework/Versions/A/Headers/sys/fcntl.h)).
The current loop already uses the range form correctly and advances by the
returned contiguous length ([src/extents.rs:152-169](../src/extents.rs#L152-L169)).

getattrlist exposes ATTR_FILE_ALLOCSIZE, ATTR_CMN_FILEID, and the old
ATTR_FILE_DATAEXTENTS attribute. The SDK defines DATAEXTENTS as only eight
diskextent records and labels it “obsolete, HFS-specific”
([local sys/attr.h](file:///Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/Frameworks/Kernel.framework/Versions/A/Headers/sys/attr.h#L539-L587));
the Apple man page likewise describes only the first eight extents
([getattrlist(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getattrlist.2.html)).
It is not a complete APFS replacement, especially for fragmented files.

There is no documented refcount or “this file has no shared physical range”
result in these APIs. Seeing one unique range cannot justify stopping: a later
range can still be shared. A safe early exit needs a complete map or an
authoritative filesystem/writer protocol.

**[FINDING] No cheaper documented API provides the whole APFS extent map in one
call.** getattrlistbulk provides metadata and allocated size, and Foundation
provides content-ID hints, but exact block sharing still needs the extent map
or a trusted producer protocol.

## 4. Options, costs, and risks

### A. Remove the pre-clean physical promise — recommended

Use the logical allocated sum for the scan, label it “allocated size” or “up to
this amount,” and report actual free-space delta after a clean. The post-action
path already reads free_before/free_after and computes the delta in
[src/tasks_clean.rs:126-149](../src/tasks_clean.rs#L126-L149).

* Cost: no exact pre-clean reclaimable figure or per-selection prediction; an
  explicit physical view still needs option B.
* Risk: free-space delta includes concurrent allocations/deallocations, as
  documented in [docs/apfs-clone-overcount.md:136-143](apfs-clone-overcount.md#L136-L143).
* Safety: removes the stale pre-clean claim instead of guessing it.

### B. Defer a fresh physical pass until explicitly needed

Keep the existing boundary at [src/tasks.rs:239-263](../src/tasks.rs#L239-L263)
and run fresh Reclaim::measure only for an explicit physical view or action.

* Cost: roughly 6 s on this current noisy workload; memory holds extent vectors
  until combine.
* Risk: UI semantics must distinguish logical allocated bytes from reclaimable
  device bytes.
* Safety: fresh accounting remains exact within the target set.

### C. Keep bounded parallel fresh reads

The implementation is at [src/reclaim.rs:101-151](../src/reclaim.rs#L101-L151)
and the cap at [src/qos.rs:39-50](../src/qos.rs#L39-L50).

* Evidence: 16.73 s at one worker, 10.30 s at two, 6.93 s at four, and 6.48 s
  at six; the public six-worker read was 6.12 s.
* Cost: same opens and fcntl calls, more disk pressure and retained vectors.
* Risk: more workers contend with peer builds; six is near diminishing returns.
* Safety: all targets are still read and combined.

### D. Use APFS content IDs only as positive prioritization hints

Query NSURLFileContentIdentifierKey and NSURLMayShareFileContentKey while
collecting metadata. Equal IDs can prioritize likely clone pairs, but they must
not be used to skip unequal IDs. The signal would sit beside
[src/sizes.rs:165-190](../src/sizes.rs#L165-L190), not replace
[src/extents.rs:152-169](../src/extents.rs#L152-L169).

* Cost: an additional Foundation metadata lookup per candidate; no guaranteed
  reduction unless a complete map can be safely reused.
* Risk: the partial rewrite false negative would over-report reclaimable bytes.
* Recommendation: diagnostics or scheduling only, not exact exclusion.

### E. Reuse maps under a trusted writer generation

Make every producer advance an authoritative generation and treat missing,
dropped, or unverifiable generations as cache misses. Current gates are at
[src/reclaim.rs:43-55](../src/reclaim.rs#L43-L55) and
[src/extent_cache.rs:38-57](../src/extent_cache.rs#L38-L57).

* Cost: Cargo, aid, ad-hoc build tools, and every other writer must participate.
* Risk: any uninstrumented writer or lost generation recreates stale accounting.
* Safety: potentially safe only if the generation is genuinely authoritative;
  no such protocol exists here.

### F. Replace prediction with post-delete measurement

This is option A in implementation form: do not spend the extent pass to predict
a number that cleaning can measure from the volume. Keep the existing before/
after measurement, but do not call it exact per-target attribution.

* Cost: the answer arrives after irreversible deletion and concurrent volume
  activity can affect it.
* Benefit: the physical pass is unnecessary for the common scan and the result
  cannot be stale because it is measured after the action.

## Recommendation summary

* **[FINDING] Safe skip:** only trivial zero-allocated files can be excluded
  without a sharing question; the current code already excludes zero-length
  files. No useful metadata rule proves unshared extents.
* **[FINDING] Current bottleneck:** the public fresh extent read was 6.12 s and
  exact combine 0.85 s; aggregate open work was much larger than aggregate
  fcntl work. The current 19 ms logical figure does not describe this workload.
* **[FINDING] API:** F_LOG2PHYS_EXT is a per-contiguous-range query. The documented
  getattrlist extent attribute is an obsolete eight-entry HFS record, not a
  complete APFS map. No safe early-exit condition was found.
* **[FINDING] Product:** stop promising a pre-clean physical number. Show
  logical allocated bytes, defer exact unioning to an explicit request, and
  report actual post-clean free-space change.

No production code was changed. The disposable benchmark harness was removed
after measurement; the sole intended deliverable is this report.

## Sources

* Local implementation: [src/extents.rs](../src/extents.rs),
  [src/reclaim.rs](../src/reclaim.rs), [src/sizes.rs](../src/sizes.rs),
  [src/qos.rs](../src/qos.rs), [src/tasks.rs](../src/tasks.rs),
  [src/tasks_clean.rs](../src/tasks_clean.rs), [src/extent_cache.rs](../src/extent_cache.rs).
* Prior evidence: [investigation-scan-performance.md](investigation-scan-performance.md),
  [investigation-physical-cache-staleness.md](investigation-physical-cache-staleness.md),
  [apfs-clone-overcount.md](apfs-clone-overcount.md).
* Apple, [About Apple File System](https://developer.apple.com/documentation/foundation/about-apple-file-system?changes=_7).
* Apple, [NSURLFileContentIdentifierKey](https://developer.apple.com/documentation/foundation/urlresourcekey/filecontentidentifierkey?language=objc)
  and [NSURLFileIdentifierKey](https://developer.apple.com/documentation/foundation/urlresourcekey/fileidentifierkey?language=objc).
* Apple, [fcntl(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html)
  and [getattrlist(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getattrlist.2.html).
* Local macOS SDK headers:
  /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/Frameworks/Kernel.framework/Versions/A/Headers/sys/fcntl.h
  and the corresponding sys/attr.h.

