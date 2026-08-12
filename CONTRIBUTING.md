# Contributing to cycling-ble

Thanks for considering a contribution. This is a small, focused crate, so
the process is intentionally lightweight.

## Reporting issues

Open a [GitHub issue](https://github.com/friedrichwilken/cycling-ble/issues).
For bug reports, please use the bug report template — for this crate the
single most useful piece of information is usually the raw bytes of the
notification payload that failed to parse (and which characteristic they
came from), since that's enough to turn into a regression test.

For a security-relevant issue (e.g. a payload that causes a panic rather
than a clean `ParseError`), see `SECURITY.md` instead of opening a public
issue.

## Submitting a pull request

1. Fork the repo and create a branch for your change.
2. Make your change. If you're adding or fixing parsing behavior, add a
   test alongside it — see existing tests in each `src/*.rs` module for the
   pattern (hand-built byte sequences covering the relevant flag-bit
   combinations). If the change touches parsing of untrusted input, include
   a truncated/malformed-payload test asserting a clean `Err(ParseError)`
   rather than a panic — see `AGENTS.md` for why this matters.
3. Before opening the PR, run:
   ```bash
   just test
   just clippy
   just fmt
   ```
   All three should be clean (`just test` and `just clippy` produce no
   output beyond `ok`/nothing on success; `just fmt` applies formatting
   directly).
4. Open the PR against `main` and fill in the PR template.

Pre-1.0 (crate is currently `0.1.0`), breaking API changes are acceptable
when justified, but call them out explicitly in the PR description.

## Commit messages

Nothing elaborate — just:

- A clear, imperative, present-tense summary line (e.g. "Add CSC wheel
  revolution parsing", not "Added" or "Adding").
- One logical change per commit. Split unrelated changes into separate
  commits/PRs rather than bundling them.
