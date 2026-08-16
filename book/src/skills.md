# Agent Skills

Versioned Agent Skills ship under `share/cxas/skills/` in the release tarball. They are not unversioned hatch `shared-data` copies of `.agents` / `.claude` / `.gemini` trees.

This checkout does not emit that tarball, and no longer carries a `cargo-dist`
configuration: config for a release job that does not exist describes an
intention rather than a fact. The CLI is consumed via `cargo run -p cxas-cli`.
When packaging lands, the config returns with the release workflow, and skills
travel next to the binary at a pinned version.

Until then, treat the Superpowers specs and this documentation website as the agent-readable surface:

- Website: <https://yash-kavaiya.github.io/cxas-harness/>
- Coverage map: `docs/superpowers/coverage-map.md`
- CLI JSON envelope: designed for coding agents (issue #55)
