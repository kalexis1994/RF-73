# RF-Rhodes working conventions

- Keep project code, documentation and CLI messages in English. Speak to the user in their preferred language.
- After producing a version for the user to test, run `cargo run --locked --release -p rf-rhodes-lab -- audition` from this workspace. This is the standard build, package, install and Desktop launch workflow.
- The audition command owns only `dist/audition/RackForgeData`; keep the user's regular RackForge library separate. Preserve test-library audio/MIDI settings between runs.
- If RackForge is already open, report that its window must be closed before rerunning. Do not force-terminate it or replace a package behind a running host.
- Use `audition --prepare-only` for noninteractive preparation. Never launch Desktop in CI. Do not label a prepare-only run as a successful GUI/audio test.
- Validation receipts and Desktop logs are under `dist/audition/<version>-<run-id>/`. Report failures accurately and use those logs for diagnosis.
