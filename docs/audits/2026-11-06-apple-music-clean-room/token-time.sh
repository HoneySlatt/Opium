#!/usr/bin/env bash
# Times the developer token's reading from Apple's public web player, M1.12 (docs/measurements.md),
# the code before and after, alternately:
#
#   docs/audits/2026-11-06-apple-music-clean-room/token-time.sh <tree before> [runs]
#
# Inside nix develop. <tree before> is a worktree of Opium at the commit before the change
# (`git worktree add --detach <dir> c132830`); the code after is this tree. Each run downloads
# Apple's page and its main script once, without an account: a download outside a package
# manager, so only with the owner's go. The token is never printed, only its length. Runs are
# three seconds apart; 10 a side by default.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
after="$(cd "$here/../../.." && pwd)"
before="$(cd "$1" && pwd)"
runs=${2:-10}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for side in before after; do
    tree=$before
    [ "$side" = after ] && tree=$after
    mkdir -p "$work/$side/src"
    cat >"$work/$side/Cargo.toml" <<TOML
[package]
name = "token-time-$side"
version = "0.0.0"
edition = "2024"
[dependencies]
apple-music = { path = "$tree/crates/apple-music" }
reqwest = { version = "0.13.5", default-features = false, features = ["rustls"] }
tokio = { version = "1.54.1", features = ["rt", "macros"] }
[workspace]
[profile.release]
lto = "thin"
codegen-units = 1
TOML
    cat >"$work/$side/src/main.rs" <<'RS'
// One reading of the developer token, timed; the token never shown.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build().unwrap();
    let started = std::time::Instant::now();
    let token = apple_music::auth::developer_token(&client).await.expect("a token");
    println!("{:.1} {}", started.elapsed().as_secs_f64() * 1e3, token.len());
}
RS
    cp "$tree/Cargo.lock" "$work/$side/"
    (cd "$work/$side" && cargo build --release --offline -q)
done
for _ in $(seq "$runs"); do
    for side in before after; do
        echo "$side $("$work/$side/target/release/token-time-$side")"
        sleep 3
    done
done | python3 -I -c '
import collections, statistics, sys
times = collections.defaultdict(list)
for line in sys.stdin:
    side, ms, length = line.split()
    times[side].append(float(ms))
    print(f"  {side}: {ms} ms, a token of {length} characters")
for side in ("before", "after"):
    t = sorted(times[side])
    print(f"{side}: median {statistics.median(t):.1f} ms, from {t[0]:.1f} to {t[-1]:.1f}")'
