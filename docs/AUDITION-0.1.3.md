# RF-73 0.1.3 audition

Select **Calibrated Register** to try the frozen upper-register geometry.
Compare it with **Calibrated** at the same output gain and MIDI performance.
Both retain the normal MIDI velocity curve; the ordinal sample-fitting map is
not applied to incoming MIDI.

The new Register Aperture option shapes the base pickup gap and offset with
a smooth transition from MIDI 55 to 72, constant outside that range. At the
factory preset's C5 endpoint, gap is 0.572268 mm and offset 0.490099 mm. Pole
radius remains 2 mm. Geometry is resolved when a voice is created or its
profile is changed; there is no per-sample interpolation or allocation.

The research comparisons rendered one note at a time and used that note's
pickup level compensation. This playable version keeps the existing shared
compensation tied to the base geometry across all voices. Consequently the
spectral geometry matches, but the output level across the keyboard is not
claimed to duplicate the offline renders. Compare by ear at matched listening
levels; no new limiter or MIDI velocity remapping is added.

Existing presets, parameter indices and state-v4 fields keep their meanings.
Pickup choice 2 is appended for Register Aperture; old states continue to load.
An older plugin build cannot interpret the new pickup choice. The new preset
also survives program/state save and reload. Raw engine samples match the
frozen per-note geometry exactly at notes 28/55/64/72/100 and velocities .3/.85,
including replacement of a prepared profile.

The workspace and package versions are 0.1.3. Cargo.lock now resolves the
installed sibling RackForge SDK/program API 0.1.20, correcting the earlier
local lock mismatch. The reference runner lock follows the workspace version.

Use the standard `cargo run --locked --release -p rf-73-lab -- audition` flow.
Build/package/install receipts and startup logs are under `dist/audition/`.
Startup and automated smoke validation do not establish an audible preference;
that is the purpose of this audition.
