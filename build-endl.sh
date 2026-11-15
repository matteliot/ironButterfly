#!/bin/sh
# Build the Endless patch: target/endless/iron_butterfly.endl (see src/pedalkit/tools/build-endl.sh).
#   ./build-endl.sh            build and check
#   ./build-endl.sh --deploy   ...and copy to the pedal (/Volumes/ENDLESS, or $ENDLESS_VOLUME)
set -eu
cd "$(dirname "$0")"
exec src/pedalkit/tools/build-endl.sh iron_butterfly iron_butterfly "$@"
