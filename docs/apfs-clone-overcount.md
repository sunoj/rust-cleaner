# WD-40 overstates reclaimable space on APFS clones

Status: confirmed, not yet fixed. Found 2026-07-29 while verifying a menu total.

## Symptom

The menu reported `Clean All — 86.4G` with five `smart-router` rows of ~16G
each. Actual reclaimable space was roughly 20G. The free-space line
(`9.3G free of 228.3G`) was correct; only the artifact total was wrong.

## Cause

`aid` seeds each session's Cargo target directory with `cp -Rc`
(`ai-dispatch/src/agent/cargo_target.rs:239-246`). On macOS `-c` is
`clonefile`: an APFS copy-on-write clone that shares blocks with the source
until one side is written.

WD-40 sizes directories from `st_blocks`, via `getattrlistbulk` with a
`walkdir` fallback (`src/scanner.rs`). For a cloned file `st_blocks` reports
the full allocated size on *both* copies, because APFS does not attribute
shared blocks to one owner. Summing them counts the same physical blocks once
per clone.

This is not a WD-40 parsing bug — `du`, and Finder's "Get Info", report the
same inflated numbers. The arithmetic is not recoverable from the per-file
metadata `stat` returns — but it *is* recoverable from the extent map, which
`stat` does not expose and `fcntl` does. See "The figure is recoverable after
all".

## Evidence

Controlled clone test:

```
source (du)        160M
clone  (du)        160M
actual disk used     0 MB      # measured as df delta across the cp -Rc
```

Volume-level cross-check, which rules out the alternative explanation that the
clones had genuinely diverged:

```
volume used         198Gi (14:35)  ->  202Gi (14:56)   = +4Gi actual
du(smart-router)     16G  (13:03)  ->   85G  (14:56)   = +69G reported
```

A real +69G was impossible: only 9Gi was free at the start. About 65G of the
reported growth was shared clone blocks.

## Why it matters here specifically

WD-40 exists for the `aid` worktree workflow, and that workflow runs entirely
on clone-seeded target dirs. The tool overstates reclaimable space exactly in
its main use case, and `Clean All — 86.4G` is a promise it cannot keep. A user
low on disk may delete a whole session's build cache and recover almost
nothing.

Observed downstream: a peer session, reading the same inflated `du` figure,
was about to prune four session target dirs expecting ~64G back. The real
recovery would have been ~4G, leaving it still blocked.

## The figure is recoverable after all

Added 2026-08-06. The claim above that the number cannot be recovered was
tested and is wrong. `fcntl(F_LOG2PHYS_EXT)` returns the *physical* device
offset backing a logical range of a file. Clones report the same physical
offsets as their source; an ordinary copy reports different ones:

```
orig    logical 0 -> phys 188260524032  len 1179648
clone   logical 0 -> phys 188260524032  len 1179648   # identical: shared
copy    logical 0 ->  phys 79764803584  len  262144   # different: its own
```

So the true figure is the union of every file's physical extents. Sort the
(offset, length) pairs, merge overlaps, and the total is the number of distinct
physical bytes the set occupies — which is what deleting all of it returns.

Validated against a set whose answer is known by construction — an 8 MB file,
a clone of it, and a real copy:

```
files 3   naive sum 24.00 M   unique physical 16.00 M   overcount 8.00 M
```

16 MB is exactly right: the original and the independent copy, with the clone
contributing nothing of its own.

On the real tree:

```
~/.cargo-target   99,758 files   152,383 extents
naive sum        43.2 G
unique physical   9.9 G
overcount        33.3 G          # 4.4x
```

Cost: 8.19 s wall for 99,758 files, of which 0.17 s is user time and 6.14 s is
system — it is bound by one `open` plus a handful of `fcntl` calls per file,
not by computation. The `getattrlistbulk` path measures the same tree in about
0.87 s, so exactness costs roughly 9x. That is affordable only alongside a size
cache, and only where clones are plausible: `sizes_may_overlap` already names
that set.

### The limit of the method

The union is exact *within the set it is given*. A physical block also
referenced by a file outside the set is not reclaimable, and unioning cannot
see that reference, so such a block is still counted. Establishing it would
mean reading the extent map of everything outside the set. The honest claim is
"exact within the selection", not "exact".

## Fix options

1. **Report logical size, label it.** Cheapest and honest: keep the current sum
   but present group totals as "up to X" and drop the promise from the Clean
   action titles. Does not make the number correct.
2. **Detect clone-seeded siblings.** Under a shared `CARGO_TARGET_DIR` root,
   treat `<project>/<session>/` dirs as overlapping with their base and count
   the base once plus each sibling's divergence. Needs a cheap divergence
   estimate; `st_birthtime`/`st_mtime` newer than the clone point is a
   heuristic, not exact.
3. **Measure true recovery empirically.** Report free-space delta after a
   clean instead of predicting it beforehand. Accurate but only after the fact.

Option 1 is a correctness-of-claim fix and should land regardless; option 2 is
the only one that makes the displayed number meaningful.

## Note on existing behavior

`CHANGELOG.md` 0.4.1 already added a nested-size adjustment so an ancestor
target subtracts a descendant's size. That handles containment, which is a
different problem from block sharing between siblings, and does not help here.

## Caveat on the measured figure

`Reclaimed` is a free-space delta, so concurrent writes by other processes are
counted against it. A self-test that deleted a 300M target on a busy machine
reported `Reclaimed 215.6M` because other builds consumed ~85M during the same
seconds. The delta is still the honest number — it is what the volume actually
gained — but it is not an exact attribution to the delete, and on an idle
machine it will match the summed size closely.

Both lines are printed deliberately: the sum says what was removed, the delta
says what the disk got back, and their disagreement is the information.
