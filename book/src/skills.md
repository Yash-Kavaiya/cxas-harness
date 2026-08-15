# Agent Skills

Versioned Agent Skills ship under `share/cxas/skills/` in the release tarball. They are not unversioned hatch `shared-data` copies of `.agents` / `.claude` / `.gemini` trees.

This checkout does not yet emit that tarball. The CLI is still consumed via `cargo run -p cxas-cli`. When cargo-dist packaging lands (`dist-workspace.toml`), skills travel next to the binary at a pinned version.

Until then, treat the Superpowers specs and this documentation website as the agent-readable surface:

- Website: <https://yash-kavaiya.github.io/cxas-harness/>
- Coverage map: `docs/superpowers/coverage-map.md`
- CLI JSON envelope: designed for coding agents (issue #55)
