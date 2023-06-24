#!/usr/bin/env bash

set -eoux pipefail

cd e2fsprogs

# clean up previous state. this is mostly for dev.
rm -rf ./build/

# git clean -fd
# git reset --hard HEAD

# set up build dir
mkdir -pv build
cd build

# fix includes for flat includes
# TODO: file bug against e2fsprogs
# TODO: use real patches
sed -i -e 's/#include "e2_types.h"/#include "ext2_types.h"/' ../lib/ext2fs/ext2fs.h
sed -i -e 's/#include "e2_bitops.h"/#include "bitops.h"/' ../lib/ext2fs/ext2fs.h
sed -i -e 's|#include <ext2fs/ext2_types.h>|#ifdef EXT2_FLAT_INCLUDES\n#include "ext2_types.h"\n#else\n#include <ext2fs/ext2_types.h>\n#endif|g' ../lib/ext2fs/crc16.c ../lib/ext2fs/dosio.c ../lib/ext2fs/ext2_fs.h ../lib/ext2fs/ext2_io.h ../lib/ext2fs/ext2fs.h
# i swear, musl-gcc causes this to break.
sed -i -e 's|#include "com_err.h"|#include "../et/com_err.h"|g' ../lib/ext2fs/ext2fs.h

# configure! autotools! pain! :D
env CC="musl-gcc" CFLAGS="-DEXT2_FLAT_INCLUDES=1" ../configure

# generate subs and ext2 types header
make subs -j
make lib/ext2fs/ext2_types.h

# patch makefile to only build libext2fs
perl -i -pe 's/LIB_SUBDIRS=.*\n.*$/LIB_SUBDIRS=lib\/et \$\(EXT2FS_LIB_SUBDIR\) #/igs' Makefile

# subst doesn't get copied in, presumably because we're butchering the build
# process. copy it in from below
cp ../util/subst* ./util/

# do the meme!
env CC="musl-gcc" CFLAGS="-DEXT2_FLAT_INCLUDES=1" LDFLAGS="-Wl,static" make -j libs
