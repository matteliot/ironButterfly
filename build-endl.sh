#!/bin/sh
# Build the Endless patch: cargo build --release -> target/endless/iron_butterfly.endl
#   ./build-endl.sh            build
#   ./build-endl.sh --deploy   build and copy to the pedal (/Volumes/ENDLESS, or $ENDLESS_VOLUME)
set -eu
cd "$(dirname "$0")"

cargo build --release

TARGET=thumbv7em-none-eabihf
ELF="target/$TARGET/release/iron_butterfly"
OUT=target/endless/iron_butterfly.endl
BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
[ -x "$BIN/llvm-objcopy" ] || { echo "llvm-objcopy missing: rustup component add llvm-tools" >&2; exit 1; }

mkdir -p target/endless
"$BIN/llvm-objcopy" -O binary "$ELF" "$OUT"

# The Endless FPU is single precision only: any f64 instruction would fault.
if "$BIN/llvm-objdump" -d --no-show-raw-insn --mcpu=cortex-m7 "$ELF" | awk '{print $2}' | grep -q '\.f64$'; then
    echo "error: patch contains double-precision instructions" >&2
    exit 1
fi

# Header + image + bss must fit the firmware's 512 KB patch region.
TOTAL=$("$BIN/llvm-size" -A "$ELF" | awk '/^\.patch_(header|image|bss)/ {s += $2} END {print s}')
echo "built $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes; $TOTAL of 524288 bytes of patch RAM)"
[ "$TOTAL" -le 524288 ] || { echo "error: patch exceeds 512 KB" >&2; exit 1; }

if [ "${1:-}" = "--deploy" ]; then
    VOL="${ENDLESS_VOLUME:-/Volumes/ENDLESS}"
    [ -d "$VOL" ] || { echo "Endless not mounted at $VOL" >&2; exit 1; }
    # The pedal grabs a .endl the moment it appears: write, sync, then rename.
    cp "$OUT" "$VOL/upload.tmp" && sync && mv "$VOL/upload.tmp" "$VOL/iron_butterfly.endl"
    echo "copied to $VOL/iron_butterfly.endl"
fi
