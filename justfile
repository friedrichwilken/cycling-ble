# cycling-ble — justfile
# Targets for efficient AI-assisted development: only errors reach stdout.
# Usage: just check | just build | just test | just clippy | just fmt | just clean
#        just api | just api-check
# check/build/test/clippy pass extra arguments through to cargo, e.g.
# `just test --features zwift-click`.

set shell := ["/bin/zsh", "-l", "-c"]

export CARGO_TERM_COLOR := "never"

# Nightly toolchain and cargo-public-api version for the API snapshot.
# Pinned (here and in .github/workflows/ci.yml) because the snapshot's
# exact text depends on both; bump them together, then run `just api`.
api_nightly := "nightly-2026-09-27"
api_tool_version := "0.52.0"

# Type-check only — fast, errors only.
check *args:
    @cargo check -q --message-format=short {{args}} 2>&1 | grep "^error" || echo "ok"

# Build only — errors only.
build *args:
    @cargo build -q --message-format=short {{args}} 2>&1 | grep "^error" || echo "ok"

# Run the test suite. Case-sensitive block filter, not a case-insensitive
# grep -vi: cargo test's own harness lines ("running N tests", "test
# result: ... finished in 0.0s") are lowercase and would collide with a
# case-insensitive match meant for cargo's capitalized build-status lines,
# silently swallowing the pass/fail summary (a mistake worth avoiding
# twice). Warning blocks are dropped in full (start line through
# the next blank line), not just their first line, so code-snippet/note
# lines don't leak through. pipestatus[1] propagates cargo's real exit
# code past awk in the pipe.
test *args:
    @cargo test -q {{args}} 2>&1 | awk '/^warning:/{skip=1} skip{if($0==""){skip=0}; next} /^ *(Compiling|Checking|Finished|Running) /{next} {print}'; exit ${pipestatus[1]}

# Lint — clippy's whole point is its warnings, so those aren't filtered out,
# only the build-progress noise.
clippy *args:
    @cargo clippy --all-targets -q {{args}} 2>&1 | grep -v "^ *Compiling\|^ *Checking\|^ *Finished" || echo "ok"

# Regenerate public-api.txt, the committed snapshot of the public API
# (all features). Needs `cargo install --locked cargo-public-api --version
# {{api_tool_version}}` and `rustup toolchain install {{api_nightly}}`.
api:
    @cargo +{{api_nightly}} public-api --all-features > public-api.txt.tmp && mv public-api.txt.tmp public-api.txt && echo "ok"

# Fail if the current public API differs from the committed public-api.txt.
api-check:
    @cargo +{{api_nightly}} public-api --all-features | diff -u public-api.txt - && echo "ok"

# Apply rustfmt.
fmt:
    @cargo fmt

# Wipe build artefacts.
clean:
    @cargo clean -q && echo "cleaned"
