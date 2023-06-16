#!/usr/bin/env bash

set -eoux pipefail

cd e2fsprogs
mkdir -pv build
cd build
../configure
env CFLAGS="-DEXT2_FLAT_INCLUDES" LDFLAGS="-Wl,static" make libs
