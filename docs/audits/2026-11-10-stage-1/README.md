# The stage 1 audit's proofs

The tests and scripts that prove the findings of the [stage 1 audit](../2026-11-10-stage-1.md), as its reviewers wrote them at `46e7a5a`, some heads since corrected to say what was run. One folder per dimension. Nothing here is built by the
workspace or by CI. A fix starts by putting the test in place, seeing it fail, then making it pass.

## How to run them

- **A test for a crate's own module** (most `*.rs` files): its head names the file and the module
  it belongs in. Paste it there, then run `cargo test -p <crate> <test name>` inside the
  development shell.
- **`data/proofs/` and `edges/proofs/`**: two crates of their own, outside the workspace, that run
  findings through the crates' public items:

  ```sh
  cd docs/audits/2026-11-10-stage-1/data/proofs
  CARGO_TARGET_DIR=/tmp/opium-audit-proofs cargo test --offline
  ```

- **`correctness/single_instance.rs`** needs a session bus of its own; its head gives the command
  (`dbus-run-session`, off the network). **`correctness/two_instances.sh`** starts the app twice on
  a throwaway profile in a headless Weston; it was not run.
- **`correctness/nan_volume_harness/build.sh`** copies the output's fill, which is private, into a
  temporary folder, builds it there and runs it.
- **`perf/bench/`**: the benchmark of copied code behind the figures at 100,000 songs, and
  `cover_race`, which shows an album's cover made up to eight times at once; a crate of its own.
  Each binary takes a folder outside the repository to work in:

  ```sh
  cd docs/audits/2026-11-10-stage-1/perf/bench
  CARGO_TARGET_DIR=/tmp/opium-audit-bench cargo run --release --bin m18-perf-bench -- /tmp/opium-bench
  CARGO_TARGET_DIR=/tmp/opium-audit-bench cargo run --release --bin cover_race -- /tmp/opium-covers
  ```

- **`a11y/contrast.py`** computes every built-in theme's contrast ratios as both apps draw them
  (`python3 -I docs/audits/2026-11-10-stage-1/a11y/contrast.py .`); **`a11y/phone_targets.py`**
  lists the phone's tap targets, by reading the Compose code.
- **`dependencies/deny.toml`**: a `cargo deny` configuration that passes at `46e7a5a` only through
  exceptions, each with its reason; proposed, not in use.

## Which file proves which finding

| Finding | Files |
|---|---|
| L1 | `data/proofs` (p1a, p1b, p1c), `edges/proofs`, `data/downloads-store-tests.rs`, `edges/downloads_store.rs` |
| L2 | `correctness/single_instance.rs`, `data/proofs` (p5), `correctness/two_instances.sh` |
| L3 | `correctness/settings_kept.rs`, `data/desktop-tests-settings_folder_not_utf8.rs`, `data/desktop-tests-settings_unreadable_kept_aside.rs`, `edges/desktop_settings.rs` |
| L4, L9 | `data/proofs` (p3, p6), `data/synced-tests.rs` |
| L5 | `data/proofs` (p7) |
| L8 | `data/proofs` (p2), `edges/proofs`, `data/downloads-store-tests.rs` (its third test) |
| L10 | `debt/tag_write_zeros_after_power_cut.rs` |
| L11 | `data/desktop-tests-settings_newer_keys_kept.rs` |
| S2, S3 | `security/navidrome_address_password.rs`, `security/soundcloud_write_paths.rs` |
| S4, S5, W3, W6 | `edges/proofs`; also `edges/synced_tests.rs`, `edges/downloads_store.rs`, `edges/downloads_queue.rs` |
| W1 | `edges/proofs` (`replica_player_a_nan_volume_runs_the_gain_away`), `correctness/nan_volume_harness/`, `correctness/nan_volume.rs`, `edges/player_output.rs` |
| W2, W5 | `tests/apple_music_paging_test.rs`, `tests/player_tests.rs` |
| W4 | `perf/connect_long_play_list.rs` |
| W8 | `readability/xdg_folders.rs` |
| W11 | `correctness/file_path.rs`, `security/lrc_offset.rs` (`correctness/lrc_offset.rs` shows the same overflow), `edges/proofs`, `edges/local_scan.rs`, `perf/local_cover_made_once.rs`, `perf/bench` (`cover_race`) |
| The paused decrypt thread | `rest/cenc_stream_idle_decrypt.rs` |
| The stuck read, gone | `debt/player_drop_while_stalled.rs` (passes) |
| Gaps in the tests | `tests/synced_tests.rs`, `tests/downloads_store_tests.rs`, `tests/cenc_tests.rs`, `tests/opium_core_dedupe_tests.rs`, `tests/connect_tests.rs`, `tests/player_tests.rs`, `tests/soundcloud_navidrome_tests.rs` (these pass today and fail on the mutants they name), `tests/desktop_backend_tests.rs` (never compiled) |
| Contrast, accessibility | `a11y/contrast.py`, `a11y/theme_contrast.rs`, `a11y/ui_a11y_probe.rs`, `a11y/phone_targets.py` |
| The figures at 100,000 songs | `perf/bench/` |
| The mutants per crate | `tests/mutants-per-crate.txt` |
| Dependencies | `dependencies/deny.toml` |

## What was run, and what was not

- **Run during the audit, failing at `46e7a5a` as their heads say**: `data/proofs` and
  `edges/proofs` (every test), the `security/` tests, `correctness/settings_kept.rs`,
  `correctness/file_path.rs`, `correctness/single_instance.rs`, `correctness/lrc_offset.rs`, `correctness/nan_volume_harness/` (by its author), the three
  `data/desktop-tests-settings_*.rs`, `readability/xdg_folders.rs`,
  `debt/tag_write_zeros_after_power_cut.rs`, `rest/cenc_stream_idle_decrypt.rs`,
  `perf/connect_long_play_list.rs`, the failing tests of `tests/apple_music_paging_test.rs` and
  `tests/player_tests.rs`; `debt/player_drop_while_stalled.rs` passes, as it should; the
  reviewer of the tests ran the passing tests of `tests/` against the mutants they name.
- **Never compiled**, though written for the code at `46e7a5a`: the six `edges/*.rs` module tests,
  `data/downloads-store-tests.rs`, `data/synced-tests.rs`, `correctness/nan_volume.rs`,
  `perf/local_cover_made_once.rs`, `tests/desktop_backend_tests.rs` and the two `a11y/*.rs`. Each
  has a twin that ran (in `data/proofs` or `edges/proofs`), but for the last three.
- **Five tests that no fix in the crates can make pass**, because they test a copy or a library
  rather than Opium's code: `data/proofs/tests/proofs.rs`'s p4 (toml's and `read_to_string`'s own
  behaviour), and in `edges/proofs/tests/edges.rs` the two settings tests on a local
  `LikeSettings` and the two `replica_` tests on copies of private code (the cover one a race that
  does not fail every run). They show the defect; the fix is proven by the module test beside
  them.
- **`correctness/single_instance_queue_probe.rs`** is a probe of the bus's queue, not a test of a
  fix.

The comments in these files name the audit's working files (`.delegation/m18-…`, numbered outputs
such as `14.out`) and its tasks (`m18-data` and the like), and number findings as each reviewer
did (F1, D3…): those were the reviewers' notes and outputs, kept on the machine that ran the audit
and not in the repository. The table above maps the files to the record's findings.
