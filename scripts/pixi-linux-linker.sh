#!/bin/sh
set -eu

exec "$CONDA_PREFIX/bin/x86_64-conda-linux-gnu-cc" -B "$(dirname "$0")/gcc-specs/" "$@"
