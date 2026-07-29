# Round-two audit of commit `4cfb8c1`

Scope: read-only re-audit of commit `4cfb8c1` against its parent, with the
claims in `docs/audit-v0.5.0.md` treated as assertions to verify. Application
source files are unchanged. Evidence and findings are recorded as the audit
progresses.

## 1. Resolution claims

- **FAIL — Removing `launchctl` avoids the immediate duplicate/termination
  failure, but it does not create the claimed single source of truth.** The new
  implementation deliberately defers the change until the next login
  (`4cfb8c1:src/autostart.rs:22`-`24`) and derives checked state solely from
  whether a path exists (`4cfb8c1:src/autostart.rs:18`-`20`). It never checks
  whether the plist is readable/valid, whether its recorded executable still
  exists, or whether macOS has approved/disabled the background item. Apple
  documents that users can disable background items and that Service
  Management exposes an
  [enabled/requires-approval status](https://developer.apple.com/documentation/servicemanagement/smappservice/status-swift.enum/requiresapproval);
  file existence alone therefore cannot prove what will run at next login.
  There is also an
  application-local disagreement path: the Settings checkbox changes before
  its action runs, `set_enabled` can fail, and the error path rebuilds only the
  menu (`4cfb8c1:src/main.rs:181`-`189`), not the retained Settings window.
  Thus failed enable leaves the visible box checked with no plist, while failed
  disable leaves it unchecked with the plist still present. A partial
  `fs::write` failure can also leave a present but truncated plist because the
  write is directly to the final path (`4cfb8c1:src/autostart.rs:35`-`39`).
  The narrower claim that a complete valid plist in
  `~/Library/LaunchAgents` is a next-login launch mechanism is sound, but the
  Resolution's “no second source of truth to disagree with” claim is false.

- **PASS — The `main.rs` to `state.rs` extraction preserves the old state
  behavior.** `AppState` still has the same four fields and the three helpers
  retain the same calculations (`4cfb8c1:src/state.rs:20`-`45`; compare
  `4cfb8c1^:src/main.rs:35`-`61`). The thread-local still contains
  `RefCell<Option<AppState>>`, and `with_state`/`with_state_ret` retain the same
  borrow, absence, and callback semantics (`4cfb8c1:src/state.rs:16`-`18`,
  `4cfb8c1:src/state.rs:51`-`62`; compare
  `4cfb8c1^:src/main.rs:30`-`33`, `4cfb8c1^:src/main.rs:264`-`274`).
  Startup installs the same values before the same initial menu refresh, scan,
  and timers (`4cfb8c1:src/main.rs:273`-`298`; compare
  `4cfb8c1^:src/main.rs:234`-`262`).

- **PASS — The low-level window-control extraction is internally
  behavior-preserving, but it is not evidence that the old Settings submenu
  behavior was preserved.** Construction and lookup use unique view tags and
  preserve the selected value/state transformations
  (`4cfb8c1:src/controls.rs:40`-`49`,
  `4cfb8c1:src/controls.rs:61`-`108`,
  `4cfb8c1:src/controls.rs:110`-`139`). The informational Scan Rules submenu is
  also a direct move of the old loop and selector wiring
  (`4cfb8c1:src/rules_menu.rs:14`-`28`; compare
  `4cfb8c1^:src/settings.rs:57`-`71`). The overall submenu-to-window change is
  intentionally new behavior and is assessed separately below.

- **PASS — The CDATA fix prevents publication of a malformed appcast for all
  shell-representable release-note inputs, not only the reported example.**
  Shell global substitution splits every occurrence of `]]>` rather than only
  the first (`4cfb8c1:scripts/release.sh:47`-`49`). More importantly, the
  completed document is checked by `xmllint` before either upload begins
  (`4cfb8c1:scripts/release.sh:68`-`82`), so other malformed XML inputs
  (including invalid UTF-8/XML 1.0 characters) fail closed. Local
  round-trip checks passed for zero, one, and multiple CDATA terminators,
  literal markup, quotes, and newlines; `xmllint` also rejected a control-byte
  case. Shell arguments cannot contain NUL, so there is no untested NUL input
  path.

## 2. Settings round-trip, mutation ordering, and window lifecycle

- **FAIL — A Settings-window edit can silently reset a valid numeric config
  value that the edited control did not represent.** `Config` accepts arbitrary
  `u64` values for both `auto_clean_hours` and `max_age_days`
  (`4cfb8c1:src/config.rs:10`-`18`), but the window offers only five intervals
  and four ages (`4cfb8c1:src/settings_window.rs:33`-`40`). Control construction
  adds only those choices, and `select_value` simply returns without selecting
  the configured value when no tag matches
  (`4cfb8c1:src/controls.rs:82`-`94`,
  `4cfb8c1:src/controls.rs:97`-`108`). The popup therefore continues to expose
  its existing/default selection rather than the config value. Any popup or
  artifact checkbox then reads *both* popups and writes them both, plus all
  artifact types, back to config (`4cfb8c1:src/settings_window.rs:80`-`94`,
  `4cfb8c1:src/main.rs:159`-`169`). For example, a supported config value
  `auto_clean_hours = 2` can become `0` merely by changing the age or an
  artifact checkbox. The old submenu handlers wrote only the selected field
  (`4cfb8c1^:src/main.rs:123`-`148`), so this is a regression.

- **FAIL — `artifact_types` is reconstructed rather than round-tripped.**
  Reading the window iterates only the hard-coded `ARTIFACT_DIRS` list and
  emits checked names in canonical list order
  (`4cfb8c1:src/settings_window.rs:87`-`92`;
  `4cfb8c1:src/config.rs:7`-`18`). The handler replaces the entire stored
  vector even when the user changed only a popup
  (`4cfb8c1:src/main.rs:163`-`169`). Unknown entries, duplicates, and original
  ordering are silently discarded. Unknown names are not currently accepted
  by `is_dev_artifact` (`4cfb8c1:src/scanner.rs:276`-`292`), so the immediate
  scan semantics of those names are already inert; nevertheless the claim that
  no setting is silently lost is false, and the rewrite also destroys
  forward-compatible/manual config data. Known artifact membership does
  round-trip correctly as a set because build/refresh checks each known name
  and readback uses the same indexed constants
  (`4cfb8c1:src/settings_window.rs:74`-`77`,
  `4cfb8c1:src/settings_window.rs:138`-`145`).

- **FAIL — Changing artifact types does not invalidate or replace scan
  results, so disabled artifacts remain actionable.** A scan snapshots the
  whole config before spawning (`4cfb8c1:src/tasks.rs:43`-`58`) and both
  completion phases later replace `state.targets` with that snapshot's results
  (`4cfb8c1:src/tasks.rs:74`-`96`,
  `4cfb8c1:src/tasks.rs:99`-`107`). `settings_changed` saves the new artifact
  types but does not start a new scan (`4cfb8c1:src/main.rs:159`-`179`).
  Therefore changing types during an in-flight scan can publish stale targets
  *after* the change, and changing them while idle leaves old targets until the
  next manual/five-minute scan. During that interval, Clean All, Clean Old, and
  per-project actions still operate on the stale target vector
  (`4cfb8c1:src/main.rs:41`-`80`). A user can uncheck a type and then delete
  artifacts of that supposedly disabled type.

- **PASS — There is no simultaneous in-memory config write between the window
  and current menu/background paths.** All Objective-C actions and dispatched
  completions enter on the AppKit main thread, and background workers receive
  owned snapshots rather than `AppState` access
  (`4cfb8c1:src/tasks.rs:43`-`58`,
  `4cfb8c1:src/tasks.rs:239`-`261`). Background completions mutate only
  `targets`, not `config` (`4cfb8c1:src/tasks.rs:80`-`83`,
  `4cfb8c1:src/tasks.rs:102`-`106`). The new menu has no reachable config
  setter: it opens Settings and otherwise exposes cleaning, rescan, rules, and
  update-check actions (`4cfb8c1:src/menu.rs:52`-`85`). The old
  `handleSetAutoInterval:` and `handleSetMaxAge:` methods remain in
  `main.rs:94`-`119`, but no item in commit `4cfb8c1` targets them. Thus the
  observed loss is caused by whole-window serialization and stale scan
  publication, not a data race.

- **PASS — Closing and reopening the Settings window has a coherent
  application-lifetime lifecycle.** The window is explicitly not released on
  close, is strongly retained in one thread-local slot, and is reused
  (`4cfb8c1:src/settings_window.rs:42`-`63`,
  `4cfb8c1:src/settings_window.rs:103`-`115`). Reopening refreshes login,
  updater, numeric, and artifact controls from current state before ordering
  the same window front (`4cfb8c1:src/settings_window.rs:47`-`78`). The slot is
  never cleared, which intentionally keeps one window and its controls alive
  until process exit; it does not accumulate windows or use a released window.
  While the window remains open it is not proactively refreshed by unrelated
  state changes, but no current reachable menu/background path changes the
  represented config fields. The launch-item error case described above is the
  concrete stale-control exception.

## 3. Additional missed issues

- **FAIL — The new Objective-C action methods declare the wrong sender
  classes.** The Settings window wires `NSButton` instances to
  `settingsToggleLoginItem:`, `settingsToggleAutoUpdate:`, and
  `settingsCheckForUpdates:`, and wires `NSPopUpButton`/`NSButton` instances to
  `settingsChanged:` (`4cfb8c1:src/settings_window.rs:128`-`153`;
  `4cfb8c1:src/controls.rs:40`-`49`,
  `4cfb8c1:src/controls.rs:75`-`94`). All four Rust implementations instead
  declare the sender as `&NSMenuItem`
  (`4cfb8c1:src/main.rs:160`-`163`,
  `4cfb8c1:src/main.rs:181`-`184`,
  `4cfb8c1:src/main.rs:191`-`193`,
  `4cfb8c1:src/main.rs:201`-`202`). Objective-C's runtime encoding treats all
  of these as object pointers, so the two handlers that call `state` happen to
  work because both `NSButton` and `NSMenuItem` answer that selector. The Rust
  class contract is still false: objc2 references are expected to name the
  actual class (or a superclass/generic `AnyObject`), and generated AppKit
  APIs use `&AnyObject` for generic action senders. These signatures should use
  `&NSButton`, `&NSPopUpButton` where specific, or `&AnyObject`; relying on
  unrelated wrapper types is unsound and makes future sender use hazardous.

- **FAIL — User-facing release/install text contradicts the implementation.**
  The changelog still says the launch toggle “writes and bootstraps” the
  LaunchAgent (`4cfb8c1:CHANGELOG.md:18`) although this commit removes
  bootstrap, and `make install` still tells users to use a “Settings submenu”
  (`4cfb8c1:Makefile:27`-`30`) although the commit replaces it with a window.
  The same changelog has mutually inconsistent entries describing both the
  dedicated window and a move into `Settings ▸`
  (`4cfb8c1:CHANGELOG.md:5`-`12`). These do not alter runtime behavior, but they
  make the release claim and installation guidance unreliable.

- **FAIL — The new destructive Settings flow has no regression coverage.**
  Commit `4cfb8c1` adds only three `autostart` string-generation tests
  (`4cfb8c1:src/autostart.rs:72`-`94`) alongside pre-existing disk/style tests.
  There are no tests for unsupported numeric values, whole-window readback,
  artifact membership, failed-toggle resynchronization, close/reopen refresh,
  or an artifact change racing an in-flight scan. The previous audit's
  Resolution explicitly leaves orchestration testing open, but it does not
  identify these newly introduced Settings-specific cases.

## Verification

- **PASS — Static repository checks succeeded.** `git diff --check
  4cfb8c1^ 4cfb8c1` was clean, the committed release script passed `bash -n`,
  and the committed `Info.plist` passed `plutil -lint`. A local AppKit probe
  also confirmed that adding items to a fresh non-pull-down `NSPopUpButton`
  leaves index 0 selected, establishing the numeric-reset path above when
  `select_value` finds no matching tag.

- **PASS — The appcast guard was exercised beyond the original payload.**
  Five release-note shapes round-tripped exactly through the committed
  substitution and `xmllint`: plain text, one `]]>`, multiple `]]>`
  occurrences, XML markup/quotes, and embedded newlines. A note containing an
  XML 1.0 control byte was rejected, which is the required fail-closed
  behavior.

- **FAIL — Compiler verification remains blocked by the shared-target
  sandbox, not by a compiler diagnostic.** A clean archive of commit
  `4cfb8c1` was checked with the required `aid build check -p wd40` command,
  without overriding `CARGO_TARGET_DIR`. Cargo was denied permission to open
  `/Users/mingsun/.cargo-target/dev-cleaner/_base/debug/.cargo-build-lock`;
  `aid` reported 0 compiler errors and 0 warnings before that infrastructure
  failure. This limitation does not weaken the concrete source-level failures
  above.

Overall verdict: FIX
