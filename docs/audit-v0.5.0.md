# Audit of commit `0f09d18` (`v0.5.0`)

Scope: read-only comparison of `0f09d18` with parent
`0690f0a79a9f049dabb9da8bc9c23040ea0f87f5`. Findings are recorded below as
the audit progresses. No application source files are changed.

## 1. Cleanup entry points and orchestration

- **PASS — Per-project cleanup preserves its deletion set.** The parent reads
  the selected row's index, clones only that target's path, and calls
  `remove_dir_all` on that path (`0f09d18^:src/main.rs:79`,
  `0f09d18^:src/main.rs:81`, `0f09d18^:src/main.rs:85`). The new handler
  selects the same indexed target and passes the same cloned path
  (`0f09d18:src/main.rs:72`, `0f09d18:src/main.rs:74`,
  `0f09d18:src/main.rs:78`); `spawn_remove` calls `remove_dir_all` exactly once
  on it (`0f09d18:src/tasks.rs:134`, `0f09d18:src/tasks.rs:135`). The reported
  size is still diagnostic only.

- **PASS — Clean All preserves its deletion set.** The parent copies every
  `TargetDir` field for every element of `state.targets`, reconstructs that
  vector, and passes it to `clean_all` (`0f09d18^:src/main.rs:95`,
  `0f09d18^:src/main.rs:99`, `0f09d18^:src/main.rs:102`). The new handler
  clones the entire vector and passes it to the same unchanged function
  (`0f09d18:src/main.rs:84`, `0f09d18:src/main.rs:85`,
  `0f09d18:src/tasks.rs:116`, `0f09d18:src/tasks.rs:118`). `TargetDir`'s new
  derived `Clone` covers exactly its four existing fields
  (`0f09d18:src/scanner.rs:12`-`18`), so this changes representation, not path
  selection.

- **PASS — Per-group cleanup preserves its deletion set.** Both versions
  filter the same `state.targets` snapshot with
  `target.kind.group() == group` (`0f09d18^:src/main.rs:133`-`137`,
  `0f09d18:src/main.rs:100`-`106`). The parent reconstructs each matching
  target then calls `clean_all` (`0f09d18^:src/main.rs:140`-`144`); the new
  code clones each matching target and calls the same `clean_all` through
  `spawn_clean_all` (`0f09d18:src/main.rs:109`,
  `0f09d18:src/tasks.rs:116`-`118`).

- **PASS — Manual Clean Old preserves its deletion set.** Both versions
  snapshot all current targets and calculate
  `max_age_days.saturating_mul(86_400)` before background execution
  (`0f09d18^:src/main.rs:111`-`122`, `0f09d18:src/main.rs:40`-`42`,
  `0f09d18:src/main.rs:90`-`93`). The moved helper calls the same `clean_old`
  function with that snapshot and duration (`0f09d18:src/tasks.rs:125`-`127`).

- **PASS — Auto-clean-after-scan preserves its deletion set.** In both
  versions, phase two first publishes `SIZES_RESULT`, atomically clears
  `POST_SCAN_CLEAN`, then snapshots every newly sized target plus the current
  age threshold and calls `clean_old` (`0f09d18^:src/main.rs:275`-`302`,
  `0f09d18:src/tasks.rs:99`-`113`, `0f09d18:src/tasks.rs:125`-`127`).
  `src/cleaner.rs` is byte-for-byte unchanged by the commit, so its age test,
  symlink/directory recheck, and removal loop are also unchanged
  (`0f09d18:src/cleaner.rs:24`-`68`).

- **PASS — The scan/clean state machine is behaviorally preserved.**
  `SCANNING.swap(true)` still rejects a second scan before
  `POST_SCAN_CLEAN` is set; successful phase two still clears `SCANNING`
  before optionally starting cleanup (`0f09d18^:src/main.rs:354`-`362`,
  `0f09d18:src/tasks.rs:44`-`51`, `0f09d18:src/tasks.rs:100`-`112`).
  `CLEANING.swap(true)` still rejects a second cleanup, starts the animation,
  enforces the same two-second minimum, and dispatches completion to the main
  queue (`0f09d18^:src/main.rs:416`-`435`,
  `0f09d18:src/tasks.rs:141`-`153`). Auto-clean and auto-scan callbacks still
  decline work when either flag is set (`0f09d18^:src/main.rs:215`-`229`,
  `0f09d18:src/main.rs:176`-`188`, `0f09d18:src/tasks.rs:39`-`41`).

- **PASS — All four timers retain their scheduling and invalidation
  behavior.** `ANIM_TIMER` is repeating at 0.25 seconds and is invalidated on
  cleanup completion; `AUTO_TIMER` is invalidated before replacement and when
  disabled; `SCAN_TIMER` remains a single repeating five-minute timer; and
  one-shot `SHINE_TIMER` is invalidated/taken when it fires before starting
  the rescan (`0f09d18^:src/main.rs:307`-`335`,
  `0f09d18^:src/main.rs:447`-`527`, `0f09d18:src/tasks.rs:156`-`171`,
  `0f09d18:src/tasks.rs:190`-`237`). The generic `schedule` helper does not
  invalidate on overwrite, but every reachable replacement has the same
  gating or explicit invalidation as the parent: cleanup is guarded by
  `CLEANING`, auto-clean calls `stop_auto_clean`, and auto-scan is installed
  once at startup.

- **PASS — The three libdispatch completions preserve main-thread delivery
  and selector behavior.** Discovery, sizing, and cleanup still write their
  mutex-protected result (where applicable) before `dispatch_async_f` to
  `_dispatch_main_q`; each trampoline still looks up the retained
  thread-local handler and sends `scanDone:`, `sizesDone:`, or `cleanDone:`
  with a null object argument (`0f09d18^:src/main.rs:385`-`445`,
  `0f09d18:src/tasks.rs:55`-`58`, `0f09d18:src/tasks.rs:92`-`96`,
  `0f09d18:src/tasks.rs:146`-`153`, `0f09d18:src/tasks.rs:239`-`267`).
  Same-kind re-entry remains serialized by the same atomics, automatic
  cross-operation entry remains guarded by both flags, and the AppKit
  callbacks remain on the main thread. As in the parent, `start_clean` does
  not inspect `SCANNING` and `start_scan` does not inspect `CLEANING`; a manual
  Rescan selected from the still-attached menu during the cleaning animation
  can therefore make both flags true. That pre-existing cross-operation
  behavior was neither fixed nor worsened by the split.

## 2. Runtime-loaded Sparkle updater

- **PASS — Controller allocation and initialization use the correct +1
  ownership path.** `alloc` is typed as `Allocated<AnyObject>` and that value
  is consumed by the `initWithStartingUpdater:updaterDelegate:
  userDriverDelegate:` method-family send, whose result is stored as
  `Retained<AnyObject>` (`0f09d18:src/updater.rs:23`-`34`). Sparkle declares
  this initializer nonnull and both delegates nullable
  (`.sparkle/Sparkle.framework/Headers/SPUStandardUpdaterController.h:90`-`97`).
  The retained controller is then owned by `Updater`, which is owned by the
  thread-local `AppState` for the app lifetime (`0f09d18:src/updater.rs:15`-`17`,
  `0f09d18:src/main.rs:245`-`250`). Its eventual drop balances the initializer's
  +1 ownership.

- **PASS — The +0 `updater` property is retained correctly.** Sparkle declares
  `updater` as a non-copying readonly property, so its getter follows +0
  convention
  (`.sparkle/Sparkle.framework/Headers/SPUStandardUpdaterController.h:59`-`64`).
  Giving `msg_send!` the return type `Retained<AnyObject>` makes objc2's
  non-owning method-family conversion retain the returned object; each short
  local lifetime is released after the getter/setter call
  (`0f09d18:src/updater.rs:42`-`65`). This is not an over-release or a leak.
  Keeping the controller retained would also keep the property valid, but the
  explicit `Retained` is the safe objc2 representation across the subsequent
  message send.

- **PASS — Framework absent/unbundled failure is safe.** A missing
  `privateFrameworksPath` or a path that does not form an `NSBundle` returns
  `None` through `?` (`0f09d18:src/updater.rs:73`-`75`). `Updater::start`
  propagates that as `None` (`0f09d18:src/updater.rs:23`-`24`), stores no
  controller, and both the settings toggle and update menu item are omitted
  (`0f09d18:src/settings.rs:45`-`47`,
  `0f09d18:src/menu.rs:213`-`218`). No Objective-C message is sent to a missing
  class/object, and temporary retained Foundation objects drop normally.

- **PASS — `NSBundle.load() == false` is safe and non-leaking.** The code logs
  once and returns `None` before class allocation
  (`0f09d18:src/updater.rs:75`-`79`). The local `NSBundle`, path string, and
  framework-path string are all objc2 retained values and release on return;
  the application continues without updater UI.

- **PASS — Class absent after a successful load is safe.** The final
  `AnyClass::get` remains optional (`0f09d18:src/updater.rs:80`), so an
  unexpected framework lacking `SPUStandardUpdaterController` again yields
  `Updater::start() == None` without allocating or messaging a controller.
  The loaded image may remain resident according to `NSBundle`/dyld behavior,
  but there is no lost Rust-owned retain and no crash. This path is silent,
  unlike `load() == false`; adding a diagnostic would improve supportability
  but is not a correctness requirement.

## 3. What else was missed

- **FAIL — The rename has no upgrade cleanup for the previously installed app
  and LaunchAgent.** The parent installed `/Applications/Rust Cleaner.app` and
  `~/Library/LaunchAgents/com.wd40.rust-cleaner.plist`
  (`0f09d18^:Makefile:2`-`7`, `0f09d18^:Makefile:22`-`38`; the old plist points
  to the old app/binary at
  `0f09d18^:com.wd40.rust-cleaner.plist:5`-`10`). The new `install` and
  `uninstall` paths touch only `WD-40.app` and `com.wd40.app.plist`
  (`0f09d18:Makefile:19`-`28`). An existing source-install user can therefore
  retain the old app and old login job while installing the new app, causing
  two menu-bar apps at login (or a stale job after the old app is manually
  removed). The v0.5.0 installer/uninstaller should explicitly boot out and
  remove those two old identifiers/paths once.

- **FAIL — Launch-at-login UI state can disagree with launchd after an
  operation failure.** `is_enabled` tests only whether the plist file exists
  (`0f09d18:src/autostart.rs:15`-`17`). Enable writes the plist before
  `bootstrap`, but does not remove it if `bootstrap` fails
  (`0f09d18:src/autostart.rs:33`-`40`), so the handler shows an error and then
  refreshes the toggle as enabled (`0f09d18:src/main.rs:150`-`157`). Disable
  ignores every `bootout` error and can report success after deleting the file
  while the job remains loaded for the session
  (`0f09d18:src/autostart.rs:24`-`30`). Roll back the plist on failed
  bootstrap, propagate meaningful bootout failures, and derive displayed
  state from launchd (or track file and loaded state separately).

- **FAIL — Release notes are inserted into XML without guarding the CDATA
  terminator.** Arbitrary `NOTES` is interpolated directly inside a CDATA
  section (`0f09d18:scripts/release.sh:7`-`9`,
  `0f09d18:scripts/release.sh:47`-`58`). Notes containing `]]>` produce a
  malformed appcast (and can inject following XML), yet the script uploads it
  without `plutil`/XML validation (`0f09d18:scripts/release.sh:64`-`71`).
  Split/escape the CDATA terminator or generate XML with an XML-aware tool,
  then validate before upload.

- **FAIL — The new orchestration/updater behavior has no automated regression
  coverage.** The commit adds `src/tasks.rs` and `src/updater.rs` but adds no
  tests for cleanup snapshot equivalence, rejected re-entry,
  `POST_SCAN_CLEAN`, timer replacement/invalidation, or the three updater
  failure modes. The static equivalence proof above is strong, but these are
  precisely the stateful paths most likely to regress in a later edit.

## Verification

- **PASS — Repository-level static checks used in this audit succeeded.**
  `git diff --check 0f09d18^ 0f09d18` was clean. The committed `Info.plist`
  passes `plutil -lint`, all three new shell scripts pass `bash -n`, and the
  produced `dist/WD-40.app` passes
  `codesign --verify --deep --strict`; `otool -L` confirms the menu binary has
  no static Sparkle dependency, consistent with runtime loading.

- **FAIL — A fresh compiler verification could not complete in this
  restricted audit environment.** The required command, `aid build check -p
  wd40`, invoked `cargo check -p wd40` without overriding `CARGO_TARGET_DIR`,
  but Cargo was denied permission to open the warm shared target's
  `.cargo-build-lock`. The tool reported zero compiler diagnostics before that
  infrastructure error. This is a verification limitation, not evidence of a
  source error.

Overall verdict: FIX

---

## Resolution (applied in `fix/audit-v0.5.0`)

- **Finding 1 (no upgrade cleanup) — FIXED.** `make install` now depends on a
  `migrate-legacy` target that boots out `com.wd40.rust-cleaner`, deletes its
  plist, and removes `/Applications/Rust Cleaner.app`. Marked in the Makefile
  as deletable once no 0.4.x install remains, since the project forbids
  permanent migration shims.

- **Finding 2 (login-item state disagreement) — FIXED, differently than
  suggested.** Live testing showed the `launchctl` calls were themselves the
  defect: `bootstrap` spawned a second menu bar instance beside the running
  app, and `bootout` then terminated the app mid-handler, leaving the plist
  behind and the toggle stuck on. `src/autostart.rs` now only writes or removes
  the LaunchAgent — launchd loads `~/Library/LaunchAgents` at login on its own,
  so there is no second source of truth to disagree with. Verified by toggling
  from the Settings window: enable creates the plist, disable removes it, the
  app survives both, and the checkbox reflects reality.

- **Finding 3 (CDATA injection in release notes) — FIXED.** `scripts/release.sh`
  splits any `]]>` in `NOTES` across two CDATA sections and refuses to upload an
  appcast that fails `xmllint --noout`. Verified against the exact payload from
  the finding: previously a parse error, now well-formed with the text
  preserved verbatim as character data.

- **Finding 4 (no regression coverage) — PARTIALLY ADDRESSED, still open.**
  Tests were added for the pure helpers only. The stateful orchestration in
  `src/tasks.rs` and the updater failure paths remain untested: they are bound
  to AppKit main-thread state and NSTimer scheduling, which the current design
  cannot exercise from `cargo test`. Making them testable needs a seam that
  does not exist yet, so this is recorded as a known gap rather than closed.

- **Verification FAIL (cargo check blocked by the sandbox) — NOT A DEFECT.**
  Independently confirmed outside the audit sandbox: `cargo build --release`
  and `cargo test` are clean, and the audit's own retry reported `cargo check
  -p wd40` at 0 errors and 0 warnings.
