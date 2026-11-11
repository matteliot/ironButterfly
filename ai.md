# ai.md — Project Requirements Log

This file records every requirement given for the **ironButterfly** project, so the whole project can be recreated from scratch by handing this file to an AI assistant as the prompt.

It is kept up to date as new requirements are given. Requirements are listed in the order they were given; later entries override earlier ones where they conflict (and the earlier entry is marked as
superseded).

---

## Meta / Process Requirements

1. Maintain this `ai.md` file at the repo root. Its purpose is to keep a list of all requirements so the entire project can be recreated from this file alone. Update it as work progresses, whenever a
   new requirement is given.

## Project Requirements

2. **Build a Polyend Endless patch binary, the same way as the sibling projects `../soundGarden` and `../nirvana`, for "the most Iron Butterfly 'In-A-Gadda-Da-Vida' sound ever."** Build it with `cargo
   build --release`.
   - The target sound is 1968 heavy psych: a raspy, thin Fuzzrite-style fuzz into a bright Vox-style amp.
   - The riff is doubled by a combo organ, and the whole thing sits in period reverb.
3. Push the project to its GitHub remote (`origin`).

4. On `master`, add the demo render (`cargo sim` output) as `samples/iron_butterfly_demo.wav`. Commit with the message "ai" and push.

5. Keep `ai.md` lines unwrapped: only wrap where needed, or at a natural word break around 200 characters.

6. Fix: the fuzz got noisy (intermodulation hash) when several notes overlapped, as in ringing notes and chords. It was not the reverb (adds ~0.01 spectral flatness), the organ tracker (stays clean on chords) or aliasing (oversampling didn't help). Turning the Fuzz knob down must actually clean it up, like rolling back a real fuzz. The default setting must keep its sound, and the volume curve across the knob must stay about where it was. Then build and push.

### Implementation decisions (needed to recreate requirement 2)

- **Toolchain and packaging (copied from `../soundGarden`).**
  - Language and target: `no_std` Rust, edition 2024, nightly toolchain, `thumbv7em-none-eabihf`.
  - Copied files: `src/abi.rs` (FxPatchSDK ABI 0x000B header), `patch.ld`, `build.rs`, `.cargo/config.toml` (pedal is the default target, `-fp64`), and `src/dsp.rs` (f32-only sin/cos/exp2/tanh,
    biquads, one-pole, delay lines).
  - `build-endl.sh` makes `target/endless/iron_butterfly.endl`. It checks for f64 instructions and the 512 KB limit; `--deploy` copies the file to `/Volumes/ENDLESS`.
- **Guitar path.** Input HP 70 Hz, then a gate (opens at 0.0012, closes at 0.0006).
  - **Fuzzrite fuzz:**
    1. HP 160 Hz.
    2. Gain 6–40 dB (Fuzz knob; requirement 6 widened it from 18–40 dB).
    3. Stage 1, with 2x oversampling: tanh for positive input, `max(1.4u, -0.7)` for negative input (starved, asymmetric).
    4. Stage 2: `clamp((1.5 + 4.5·depth)·a)`. Its gain follows Depth, so it is no longer a fixed 6×.
    5. Depth blend `b·depth - 0.5·a·(1-depth)`, where depth = 0.45–0.95, set by the same knob.
    6. LP 6.5 kHz, then +4 dB at 1.8 kHz.
    7. Makeup gain below the default: `+7 dB × max(0, 0.7 - fuzz)`. This keeps the volume curve within 1 dB of the old range.
    - Measured on ringing notes and chords (spectral flatness): at Fuzz 0.2, 0.231 → 0.095 and 0.182 → 0.056. At the default 0.7 it is about the same (0.234 → 0.226).
  - **Clean:** `tanh(2.5x)` with +36 dB makeup and ×0.1. The footswitch press crossfades between fuzz and clean over 12 ms.
  - **Vox-style amp EQ:** HP 90 Hz, -2 dB at 250 Hz, +4 dB at 2.8 kHz.
  - **Cab:** HP 95 Hz, +2 dB at 1.2 kHz, +3 dB at 3.5 kHz, LP 6.5 kHz, LP 9 kHz.
- **Organ double.**
  1. Tracking: two LP 320 Hz biquads, then a Schmitt trigger at ±0.25 × the tracked envelope. Its state gives the 8' square; toggling on each rising edge gives the 16' sub-octave square.
  2. Mix: 0.55 × sub + 0.45 × fundamental.
  3. Amplitude follows the input envelope (4 ms attack, 120 ms release), × 19 gain, with a 6.2 Hz ±8% amplitude vibrato.
  4. LP 2.5 kHz, HP 30 Hz, then gated.
  - Hold toggles the organ, crossfaded over 12 ms.
- **Mix and reverb.**
  - Panning: guitar (L 1.0, R 0.6), organ (L 0.6, R 1.0).
  - Reverb: Freeverb-style, fed the mono sum × 0.25. Four LP-feedback combs per channel (L 1215/1293/1390/1476 samples; R is +25 on each), feedback 0.83, damping 0.35. Then two allpasses (L 605/480, R
    630/505) with gain 0.5.
  - The Reverb knob sets the wet mix from 0 to 0.6. Reverb buffers are in a separate all-zero static, so they're in .bss, not the file.
  - Output: Level (square law, unity at 0.75) × 0.3 trim, then `tanh` × 0.98.
- **Controls.**
  - Knobs: Fuzz (default 0.7), Reverb (default 0.35), Level (default 0.75).
  - Press: fuzz on/off. Hold: organ on/off. Both start on.
  - LED: fuzz+organ magenta, fuzz red, clean+organ light blue, clean light green. Red also means the working buffer is missing.
- **Host audition.** `cargo sim` renders an *original* D-minor single-note riff at 112 bpm, not the song's actual riff, using Karplus-Strong. It plays fuzz+organ → fuzz only → clean dyads with reverb
  → fuzz+organ, writes `target/render/iron_butterfly_demo.wav`, and prints per-section stats.
