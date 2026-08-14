# Physical-cache staleness investigation

## Decision

**Yes, the reclaimable headline can be stale.** On this Mac, an in-place rewrite
of one 64 MiB APFS clone left both target sizes at 64 MiB and left both target
directory mtimes unchanged. WD-40's cached union stayed at 64 MiB; a forced fresh
extent read found 128 MiB. The observed stale amount was therefore 64 MiB, or a
2x under-report of the true union.

This is a real correctness bug, but not a daily failure for ordinary Cargo
rebuilds. A same-size in-place rewrite is routine and is enough to trigger it;
Cargo rebuilds more commonly create, replace, or remove files and therefore
change the aggregate fingerprint. Exact same-size rebuild output and equal-size
growth/shrinkage remain reachable, so the physical caches cannot claim that
`(path, size)` proves unchanged extents.

No production code was changed. The reproduction used disposable directories,
a scratch `HOME`, and a separate target directory with
`CARGO_DISK_GUARD_MIN_FREE_GB=3`. The repository's normal `CARGO_TARGET_DIR`
was not overridden for the project check.

## Root cause and cache order

The root cause is that physical extent identity is inferred from aggregate size
and shallow directory mtime, while APFS can detach a clone's extents during a
same-size write.

The second scan takes this path:

1. `Reclaim::measure` computes a fingerprint of `(target path, target size)` and
   asks the whole-list attribution cache first ([`src/reclaim.rs:41-47`](../src/reclaim.rs#L41-L47)).
2. If that cache misses, each target asks the process-lifetime extent cache for
   `(target root mtime, target size)` ([`src/reclaim.rs:53-63`](../src/reclaim.rs#L53-L63),
   [`src/extent_cache.rs:34-49`](../src/extent_cache.rs#L34-L49)).
3. Only misses at both levels call `read_target` and recompute the union.

Therefore the first cache to bite in the normal second scan is the attribution
cache. It returns the old whole-list answer before the extent cache is even
consulted. It is persisted by `cache::flush` to `~/.config/wd-40/cache.toml`
([`src/cache.rs:147-175`](../src/cache.rs#L147-L175)) and remains eligible for
six hours ([`src/cache.rs:34-38`](../src/cache.rs#L34-L38),
[`src/cache.rs:124-131`](../src/cache.rs#L124-L131)). In the same running app,
the extent cache is the fallback failure: it has no TTL and survives until the
process exits or `forget` removes that path ([`src/extent_cache.rs:14-18`](../src/extent_cache.rs#L14-L18),
[`src/extent_cache.rs:54-61`](../src/extent_cache.rs#L54-L61)).

The app starts reclaim after the logical sizes reach the screen and flushes the
attribution cache when that pass completes ([`src/tasks.rs:209-243`](../src/tasks.rs#L209-L243)).
That makes a stale physical value especially plausible on an automatic rescan:
the same paths and sizes cause an immediate cached result, while a changed path
set can miss attribution and still reuse old per-target extents.

## Reproduction

The filesystem observation was:

```text
$ mount | rg ' on / '
/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)
```

I created a 64 MiB random file, made two APFS clones with `cp -c`, and measured
the two directories with a disposable harness calling the real
`wd40::reclaim::Reclaim::measure` and `wd40::sizes::measure_dir` APIs:

```sh
REPRO=/private/tmp/wd40-physcache-repro.zOIcVx
dd if=/dev/urandom of=$REPRO/seed/payload.bin bs=1m count=64
cp -c $REPRO/seed/payload.bin $REPRO/a/payload.bin
cp -c $REPRO/seed/payload.bin $REPRO/b/payload.bin
CARGO_TARGET_DIR=$REPRO/aid-build-target/target \
CARGO_DISK_GUARD_MIN_FREE_GB=3 HOME=$REPRO/home \
cargo run --quiet --manifest-path $REPRO/Cargo.toml -- baseline $REPRO/a $REPRO/b
```

The baseline output was:

```text
64+0 records in
64+0 records out
67108864 bytes transferred in 0.120090 secs (558821417 bytes/sec)
baseline: app_sizes=[67108864, 67108864] reclaim_total=67108864
```

The persisted cache showed that the 64 MiB was one shared physical run, not
two summed file sizes:

```toml
[attribution]
fingerprint = [[".../a", 67108864], [".../b", 67108864]]

[attribution.value]
exclusive = [0, 0]
shared = [[[0, 1], 67108864]]
```

Before the rewrite, both target directories had the same mtime and both files
had 64 MiB of allocated blocks:

```text
/.../a mtime=1786691774 size=96 blocks=0
/.../b mtime=1786691774 size=96 blocks=0
/.../a/payload.bin mtime=1786691768 size=67108864 blocks=131072
/.../b/payload.bin mtime=1786691768 size=67108864 blocks=131072
```

I then rewrote only `b/payload.bin` in place, without changing its length:

```sh
dd if=/dev/urandom of=$REPRO/b/payload.bin bs=1m count=64 conv=notrunc
```

The output showed the decisive signal combination:

```text
64+0 records in
64+0 records out
67108864 bytes transferred in 0.135659 secs (494687887 bytes/sec)
/.../a mtime=1786691774 size=96 blocks=0
/.../b mtime=1786691774 size=96 blocks=0
/.../a/payload.bin mtime=1786691768 size=67108864 blocks=131072
/.../b/payload.bin mtime=1786691829 size=67108864 blocks=131072
```

The target-directory mtimes, file lengths, and allocated block counts stayed
the same. The rewritten file's mtime changed, but that is deep content below
the directory and is not read by either physical-cache key.

With the scratch cache still inside its six-hour lifetime, a new process ran
the app path again:

```text
cached: app_sizes=[67108864, 67108864] reclaim_total=67108864
```

That is the stale headline. I then called `cache::forget` for both paths before
measuring, which forces fresh extents:

```text
fresh: app_sizes=[67108864, 67108864] reclaim_total=134217728
```

The logical file size and WD-40 size fingerprint were unchanged, but the true
union grew by one 64 MiB run because `b` no longer shared the old run with `a`.
This confirms the APFS copy-on-write consequence rather than merely assuming
it.

## Which cache fails under an attribution miss?

To isolate level one, I measured `a2` and `b2`, paused the same process, rewrote
`b2/payload.bin` in place, then measured `a2`, `b2`, and a newly discovered empty
`c2`. Adding `c2` deliberately changed the whole-list fingerprint, so the
six-hour attribution entry could not answer. The per-target extent entries for
`a2` and `b2` still matched their old root mtimes and sizes:

```text
extent-first: app_sizes=[67108864, 67108864] reclaim_total=67108864
ready: press enter after rewriting B
extent-attribution-miss: app_sizes=[67108864, 67108864, 0] reclaim_total=67108864
extent-fresh: app_sizes=[67108864, 67108864, 0] reclaim_total=134217728
```

Thus the normal unchanged-list path is stopped by the persisted attribution
cache first; the process-lifetime extent cache independently reproduces the
staleness whenever attribution is bypassed by a changed path list or an expired
entry.

## Ordinary-use reachability

The experiment proves that a file rewritten in place at the same size is a
sufficient and simple trigger. That operation is not exotic in a build tree:
tools can rewrite generated state or an artifact without changing its length.
However, an ordinary Cargo rebuild usually changes several files and often
changes the aggregate allocated size, so this exact fingerprint collision is a
corner case of normal Cargo use rather than a daily outcome.

As a control, a disposable `cargo build` with a one-line source rewrite changed
the measured target size from `1134592` to `1306624` and then to `1355776` on a
second source rewrite; a no-op build kept the last value at `1355776`. This is
evidence against calling every rebuild a collision, not evidence that collisions
cannot occur. Two changes can also cancel in the aggregate, and an in-place
same-size write does not need cancellation at all.

The bug's possible error is not bounded by the logical-size delta (which is zero
in this case). It is the difference between the cached and current physical
union; the controlled under-report was 64 MiB. Larger clone-backed artifacts
can produce a larger absolute error.

## What was missed

* `TargetDir::size_bytes` is the actual fingerprint input. The sizing pass uses
  allocated size when available (`alloc_size.or(size)`), not a cryptographic
  content identity ([`src/sizes.rs:40-45`](../src/sizes.rs#L40-L45),
  [`src/sizes.rs:180-187`](../src/sizes.rs#L180-L187)). The reproduction kept
  both the logical file length and this app size equal.
* Physical union correctness depends on sharing relationships. A same-size
  rewrite can change whether two targets share a device run even when each
  target's own allocated size is unchanged.
* `forget` is tied to a target disappearing, not to a deep file mutation. It
  cannot repair the same-size rewrite shown here.
* `content_modified` exists for the logical-size cache, but Rust targets have
  `Duration::ZERO`, and the physical gates do not use it
  ([`src/cache.rs:24-31`](../src/cache.rs#L24-L31),
  [`src/cache.rs:52-60`](../src/cache.rs#L52-L60)). Even a descendant mtime is
  evidence of activity, not proof that extents are unchanged: timestamps can be
  restored and are stored at one-second precision, as the companion report
  documents ([`docs/investigation-scan-performance.md:129-154`](investigation-scan-performance.md#L129-L154)).

## Fix options (no fix applied)

### A. Remove both physical-cache hits

Bypass `cache::attribution_for` at [`src/reclaim.rs:43-47`](../src/reclaim.rs#L43-L47)
and `extent_cache::extents_of` at [`src/reclaim.rs:53-60`](../src/reclaim.rs#L53-L60),
or remove their writes. Every reclaimable headline then reflects a fresh
`read_target`/`combine` pass.

* Cost: restores the full physical scan on every reclaim pass; the existing
  code comment estimates up to 71 seconds on this Mac
  ([`src/tasks.rs:222-230`](../src/tasks.rs#L222-L230)).
* Benefit: the only direct option here that removes this cache's stale-answer
  path without requiring a writer protocol.

### B. Defer physical accounting until it is needed

Keep the logical scan responsive and start the fresh physical pass only when
the reclaimable view or a clean operation needs it, at the existing boundary in
[`src/tasks.rs:217-243`](../src/tasks.rs#L217-L243).

* Cost: the user waits when asking for physical reclaim, and the UI must clearly
  distinguish logical size from reclaimable device bytes.
* Benefit: avoids paying the physical pass on every automatic scan while keeping
  the physical answer fresh.

### C. Replace size keys with a trusted writer generation

Add a generation that every producer of a target must advance, and require an
unverifiable or missing generation to be a cache miss at
[`src/cache.rs:124-131`](../src/cache.rs#L124-L131) and
[`src/extent_cache.rs:34-39`](../src/extent_cache.rs#L34-L39).

* Cost: Cargo and every other writer would need to participate; a missed,
  dropped, or spoofed generation must fail closed. No such protocol exists in
  this codebase, so this is a design project rather than a local patch.
* Benefit: can make reuse safe without walking every descendant, if the protocol
  is genuinely authoritative.

### D. Use mtimes, FSEvents, or fingerprints as heuristics

Use a descendant mtime, FSEvents invalidation, hashes, or a deeper marker as a
miss hint around [`src/extent_cache.rs:28-39`](../src/extent_cache.rs#L28-L39).

* Cost: descendant checks approach the logical walk cost; hashes add reads;
  FSEvents needs persistence, coalescing/drop handling, and rescans.
* Safety: insufficient for an unconditional physical headline. A writer can
  preserve timestamps, and event absence is not proof of unchanged extents.
  This option improves hit rate or reduces stale duration only if it is paired
  with a safe fallback, not if it is treated as proof.

## Verification

The repository remained free of production-code changes. Verification used the
requested wrappers:

```text
$ aid build
succeeded: 0 errors, 0 warnings; command: cargo check; elapsed: 20.3s

$ aid test --lib
passed: 58 passed, 0 failed, 0 ignored; command: cargo test --lib; elapsed: 26.1s
```
