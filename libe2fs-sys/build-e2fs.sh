#!/usr/bin/env bash

set -eoux pipefail

cd e2fsprogs

# clean up previous state. this is mostly for dev.
rm -rf ./build/

# git reset --hard HEAD
# set up build dir
mkdir -pv build

# subs
# yes, two passes are necessary. the first pass generates the subs files, and
# the second ./configure actually configures for build.
./configure
make subs
make lib/ext2fs/ext2_types.h

# build!
cd build

# fix includes for flat includes
# TODO: file bug against e2fsprogs
# TODO: use real patches
sed -i -e 's/#include "e2_types.h"/#include "ext2_types.h"/' ../lib/ext2fs/ext2fs.h
sed -i -e 's/#include "e2_bitops.h"/#include "bitops.h"/' ../lib/ext2fs/ext2fs.h
sed -i -e 's|#include <ext2fs/ext2_types.h>|#ifdef EXT2_FLAT_INCLUDES\n#include "ext2_types.h"\n#else\n#include <ext2fs/ext2_types.h>\n#endif|g' ../lib/ext2fs/crc16.c ../lib/ext2fs/dosio.c ../lib/ext2fs/ext2_fs.h ../lib/ext2fs/ext2_io.h ../lib/ext2fs/ext2fs.h

# configure! autotools! pain! :D
env CFLAGS="-DEXT2_FLAT_INCLUDES=1" ../configure

# patch makefile to only build libext2fs
perl -i -pe 's/LIB_SUBDIRS=.*\n.*$/LIB_SUBDIRS=lib\/et \$\(EXT2FS_LIB_SUBDIR\) #/igs' Makefile

# subst doesn't get copied in, presumably because we're butchering the build
# process. copy it in from below
cp ../util/subst* ./util/

# do the meme!
make subs
env CFLAGS="-DEXT2_FLAT_INCLUDES=1" LDFLAGS="-Wl,static" make libs
