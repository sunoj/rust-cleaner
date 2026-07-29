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
same inflated numbers. The arithmetic is simply not recoverable from per-file
metadata.

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
