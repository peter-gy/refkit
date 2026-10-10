#!/bin/sh
set -eu
export DYLD_FALLBACK_LIBRARY_PATH="$CONDA_PREFIX/lib:$(dirname "$1")"
exec "$@"
