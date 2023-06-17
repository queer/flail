#include "./e2fsprogs/lib/ext2fs/ext2fs.h"

struct ext2fs_struct_generic_bitmap_32
{
  errcode_t magic;
  ext2_filsys fs;
  __u32 start, end;
  __u32 real_end;
  char *description;
  char *bitmap;
  errcode_t base_error_code;
  __u32 reserved[7];
};

struct ext2fs_struct_generic_bitmap_64
{
  errcode_t magic;
  ext2_filsys fs;
  struct ext2_bitmap_ops *bitmap_ops;
  int flags;
  __u64 start, end;
  __u64 real_end;
  int cluster_bits;
  char *description;
  void *private_;
  errcode_t base_error_code;
#ifdef ENABLE_BMAP_STATS
  struct ext2_bmap_statistics stats;
#endif
};

struct ext2_file_64
{
  errcode_t magic;
  ext2_filsys fs;
  ext2_ino_t ino;
  struct ext2_inode inode;
  int flags;
  __u64 pos;
  blk64_t blockno;
  blk64_t physblock;
  char *buf;
};

typedef struct ext2fs_struct_generic_bitmap_64 *ext2fs_generic_bitmap_64;

struct ext2_bitmap_ops
{
  int type;
  /* Generic bmap operators */
  errcode_t (*new_bmap)(ext2_filsys fs, ext2fs_generic_bitmap_64 bmap);
  void (*free_bmap)(ext2fs_generic_bitmap_64 bitmap);
  errcode_t (*copy_bmap)(ext2fs_generic_bitmap_64 src,
                         ext2fs_generic_bitmap_64 dest);
  errcode_t (*resize_bmap)(ext2fs_generic_bitmap_64 bitmap,
                           __u64 new_end,
                           __u64 new_real_end);
  /* bit set/test operators */
  int (*mark_bmap)(ext2fs_generic_bitmap_64 bitmap, __u64 arg);
  int (*unmark_bmap)(ext2fs_generic_bitmap_64 bitmap, __u64 arg);
  int (*test_bmap)(ext2fs_generic_bitmap_64 bitmap, __u64 arg);
  void (*mark_bmap_extent)(ext2fs_generic_bitmap_64 bitmap, __u64 arg,
                           unsigned int num);
  void (*unmark_bmap_extent)(ext2fs_generic_bitmap_64 bitmap, __u64 arg,
                             unsigned int num);
  int (*test_clear_bmap_extent)(ext2fs_generic_bitmap_64 bitmap,
                                __u64 arg, unsigned int num);
  errcode_t (*set_bmap_range)(ext2fs_generic_bitmap_64 bitmap,
                              __u64 start, size_t num, void *in);
  errcode_t (*get_bmap_range)(ext2fs_generic_bitmap_64 bitmap,
                              __u64 start, size_t num, void *out);
  void (*clear_bmap)(ext2fs_generic_bitmap_64 bitmap);
  void (*print_stats)(ext2fs_generic_bitmap_64);

  /* Find the first zero bit between start and end, inclusive.
   * May be NULL, in which case a generic function is used. */
  errcode_t (*find_first_zero)(ext2fs_generic_bitmap_64 bitmap,
                               __u64 start, __u64 end, __u64 *out);
  /* Find the first set bit between start and end, inclusive.
   * May be NULL, in which case a generic function is used. */
  errcode_t (*find_first_set)(ext2fs_generic_bitmap_64 bitmap,
                              __u64 start, __u64 end, __u64 *out);
};

struct real_ext2_file
{
  errcode_t magic;
  ext2_filsys fs;
  ext2_ino_t ino;
  struct ext2_inode inode;
  int flags;
  __u64 pos;
  blk64_t blockno;
  blk64_t physblock;
  char *buf;
};