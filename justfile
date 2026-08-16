# cycling-ble — justfile
# Targets for efficient AI-assisted development: only errors reach stdout.
# Usage: just check | just build | just test | just clippy | just fmt | just clean

set shell := ["/bin/zsh", "-l", "-c"]

export CARGO_TERM_COLOR := "never"

# Type-check only — fast, errors only.
check:
    @cargo check -q --message-format=short 2>&1 | grep "^error" || echo "ok"

# Build only — errors only.
build:
    @cargo build -q --message-format=short 2>&1 | grep "^error" || echo "ok"

# Run the test suite. Case-sensitive block filter, not a case-insensitive
# grep -vi: cargo test's own harness lines ("running N tests", "test
# result: ... finished in 0.0s") are lowercase and would collide with a
# case-insensitive match meant for cargo's capitalized build-status lines,
# silently swallowing the pass/fail summary (a mistake worth avoiding
# twice). Warning blocks are dropped in full (start line through
# the next blank line), not just their first line, so code-snippet/note
# lines don't leak through. pipestatus[1] propagates cargo's real exit
# code past awk in the pipe.
test:
    @cargo test -q 2>&1 | awk '/^warning:/{skip=1} skip{if($0==""){skip=0}; next} /^ *(Compiling|Checking|Finished|Running) /{next} {print}'; exit ${pipestatus[1]}

# Lint — clippy's whole point is its warnings, so those aren't filtered out,
# only the build-progress noise.
clippy:
    @cargo clippy --all-targets -q 2>&1 | grep -v "^ *Compiling\|^ *Checking\|^ *Finished" || echo "ok"

# Apply rustfmt.
fmt:
    @cargo fmt

# Wipe build artefacts.
clean:
    @cargo clean -q && echo "cleaned"
