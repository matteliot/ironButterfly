#!/bin/sh
# Build an Endless patch: cargo build --release -> target/endless/<name>.endl, then check it.
#   tools/build-endl.sh <bin> <name> [--deploy]
# Each pedal's ./build-endl.sh calls this with its binary and file names. --deploy copies the
# result to the pedal (/Volumes/ENDLESS, or $ENDLESS_VOLUME).
set -eu
BIN_NAME="$1"
NAME="$2"
shift 2
KIT="$(cd "$(dirname "$0")" && pwd)"

cargo build --release

TARGET=thumbv7em-none-eabihf
ELF="target/$TARGET/release/$BIN_NAME"
OUT="target/endless/$NAME.endl"
BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
[ -x "$BIN/llvm-objcopy" ] || { echo "llvm-objcopy missing: rustup component add llvm-tools" >&2; exit 1; }

mkdir -p target/endless
"$BIN/llvm-objcopy" -O binary "$ELF" "$OUT"

# The Endless FPU is single precision only: any f64 instruction would fault. Only instructions inside function
# symbols are checked: .patch_image also holds .rodata (e.g. embedded voice prompts), which the disassembler
# would otherwise decode as instructions and report false hits in.
"$BIN/llvm-objdump" -t "$ELF" | awk '$3 == "F" { printf "F %s %s\n", $1, $5 }' > target/endless/funcs.txt
F64=$("$BIN/llvm-objdump" -d --no-show-raw-insn --mcpu=cortex-m7 "$ELF" | cat target/endless/funcs.txt - | awk '
    function hex(s,   i, c, v) { v = 0; s = tolower(s); for (i = 1; i <= length(s); i++) { c = index("0123456789abcdef", substr(s, i, 1)) - 1; v = v * 16 + c } return v }
    $1 == "F" { n++; lo[n] = hex($2); hi[n] = lo[n] + hex($3); next }
    $1 ~ /^[0-9a-f]+:$/ && $2 ~ /\.f64$/ {
        a = hex(substr($1, 1, length($1) - 1))
        for (i = 1; i <= n; i++) if (a >= lo[i] && a < hi[i]) { print; break }
    }')
if [ -n "$F64" ]; then
    echo "error: patch contains double-precision instructions:" >&2
    echo "$F64" >&2
    exit 1
fi

# Header + image + bss must fit the firmware's 512 KB patch region.
TOTAL=$("$BIN/llvm-size" -A "$ELF" | awk '/^\.patch_(header|image|bss)/ {s += $2} END {print s}')
echo "built $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes; $TOTAL of 524288 bytes of patch RAM)"
[ "$TOTAL" -le 524288 ] || { echo "error: patch exceeds 512 KB" >&2; exit 1; }

# The header must match the SDK's PatchHeader layout.
python3 -I "$KIT/check_endl.py" "$OUT" > target/endless/check.txt || { cat target/endless/check.txt >&2; exit 1; }
echo "header check: $(tail -1 target/endless/check.txt)"

if [ "${1:-}" = "--deploy" ]; then
    VOL="${ENDLESS_VOLUME:-/Volumes/ENDLESS}"
    [ -d "$VOL" ] || { echo "Endless not mounted at $VOL" >&2; exit 1; }
    # The pedal grabs a .endl the moment it appears: write, sync, then rename.
    cp "$OUT" "$VOL/upload.tmp" && sync && mv "$VOL/upload.tmp" "$VOL/$NAME.endl"
    echo "copied to $VOL/$NAME.endl"
fi
