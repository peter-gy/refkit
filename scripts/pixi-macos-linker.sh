#!/bin/sh
set -eu
sdk=${SDKROOT:-$(xcrun --sdk macosx --show-sdk-path)}
exec "$CONDA_PREFIX/bin/clang" --no-default-config -isysroot "$sdk" "$@"
