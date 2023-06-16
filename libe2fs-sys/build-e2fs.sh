#!/usr/bin/env bash

set -eoux pipefail

cd ../e2fsprogs
mkdir -pv build
cd build
../configure
env LDFLAGS="-Wl,static" make libs