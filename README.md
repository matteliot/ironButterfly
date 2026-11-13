# ironButterfly

A [Polyend Endless](https://polyend.com/endless/) patch for the most
*In-A-Gadda-Da-Vida* guitar sound ever: 1968 heavy psych. It's written in
`no_std` Rust against the patch ABI of Polyend's
[FxPatchSDK](https://github.com/polyend/FxPatchSDK) (ABI 0x000B), so no ARM GCC is needed.

Signal chain (mono in from the left input, stereo out):

- **Guitar**: a Fuzzrite-style fuzz with two transistor stages blended out of phase
  by Depth, which makes it thin, raspy and nasal. With the fuzz off it's a chimey clean.
  Either way it goes through a Vox-style top-boost amp EQ and an alnico-style cab.
- **Organ double**: the guitar is pitch-tracked into combo-organ square waves at the
  played pitch and an octave below (16' + 8' stops). They follow the guitar's envelope
  and have a vibrato wobble, so every riff note is doubled by an "organ", like the
  organ and bass doubling the riff on the record. Play single notes; chords confuse
  the tracker.
- **Reverb**: a stereo Freeverb-style room for the 1968 wash.
- **Stereo placement**: guitar left of centre, organ right of centre.

| Control | Function |
|---|---|
| Left knob | **Fuzz**: fuzz gain and Depth together. Back it off to clean up chords and ringing notes |
| Mid knob | **Reverb**: dry to drenched |
| Right knob | **Level**: output, unity at 3 o'clock |
| Footswitch press | **Fuzz** on/off (off = chimey clean) |
| Footswitch hold | **Organ double** on/off |

| LED | Organ on | Organ off |
|---|---|---|
| Fuzz | magenta | red |
| Clean | light blue | light green |

## Build

```sh
rustup target add thumbv7em-none-eabihf --toolchain nightly
rustup component add llvm-tools --toolchain nightly
cargo build --release        # ELF for the pedal (default target is thumbv7em-none-eabihf)
./build-endl.sh              # build + make target/endless/iron_butterfly.endl, check f64 use and size
./build-endl.sh --deploy     # ...and copy to /Volumes/ENDLESS
```

## Audition on your computer

```sh
cargo sim                    # -> target/render/iron_butterfly_demo.wav
```

This renders an original D-minor single-note riff at 112 bpm on a modelled electric guitar (`src/guitar.rs`: exact tuning, pick and humbucker, palm mutes, strums, vibrato and human timing) through the same
patch code. It goes fuzz + organ → fuzz only → clean + reverb → fuzz + organ,
then prints the level of each section.
