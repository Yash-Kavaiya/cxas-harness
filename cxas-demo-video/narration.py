"""Per-scene narration for the cxas-harness demo, ordered by scene index.

Word budgets are roughly duration-seconds x 2.3 words (comfortable pace).
"""

NARRATION = {
    "TitleScene": (
        "cxas harness. A machine-first CLI and library harness for "
        "CX Agent Studio, written in Rust. Ten crates, JSON output, "
        "stable exit codes."
    ),
    "PitchScene": (
        "One hundred and seventy of one hundred and seventy CES methods "
        "addressable. Thirty seven hand modelled with real types."
    ),
    "DefectScene": (
        "A green test suite once missed a real bug. The evaluation run state "
        "declared pending, succeeded, failed. CES declares queued, completed, "
        "error. Seventy eight passing tests could not see it."
    ),
    "ArchitectureScene": (
        "The architecture: Google's own discovery documents, pinned and "
        "checksummed, generate the methods table. cxas core builds requests "
        "from it, and cxas parity keeps the contract honest in both "
        "directions. Requests flow to the CES REST API, authenticated with "
        "cached, refreshed tokens."
    ),
    "CoverageScene": (
        "Coverage is reported, never gated. v one: sixty six of sixty six. "
        "v one beta: one hundred and four of one hundred and four. Total, "
        "one hundred and seventy. Thirty seven modelled."
    ),
    "InstallScene": (
        "Install and verify: cargo build. Two hundred and twenty one tests "
        "across the workspace. Clippy clean. Fifty eight pytest tests, "
        "including the ones that keep the Gauntlet Loop honest. No Google "
        "Cloud project, no credentials, no network. Fully offline and "
        "deterministic."
    ),
    "CliScene": (
        "The CLI is machine first. JSON by default, no prompts, stable exit "
        "codes. Init scaffolds an app: app dot yaml, plus an agents folder "
        "with a main instruction. Lint runs over the directory and reports "
        "zero errors. Api list answers offline, straight from the generated "
        "method table. Api stream delivers session messages one by one, "
        "exactly as they arrive."
    ),
    "AuthStreamingScene": (
        "Credentials resolve the way Google's tools resolve them: an OAuth "
        "token, then environment variables, then the ADC file, then the "
        "metadata server, then gcloud. First usable wins. Tokens are cached "
        "and refreshed before expiry. Streaming delivers each message as it "
        "arrives, and a stream that ends mid message is an error, not a "
        "short result."
    ),
    "GauntletScene": (
        "The Gauntlet Loop: a builder edits one crate, evidence is collected "
        "by deterministic code, and a blind critic judges only that evidence. "
        "Stop conditions are enforced in code. Reaching a cap is a fail, "
        "never a pass."
    ),
    "OutroScene": (
        "Build for CX Agent Studio, in Rust. Find the code and docs at the "
        "links below. Thanks for watching."
    ),
}
