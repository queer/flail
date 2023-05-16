use super::*;

pub struct ExtBlock(pub(crate) libe2fs_sys::blk64_t);

impl ExtBlock {
    pub fn num(&self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
pub struct ExtBlockBitmap(pub(crate) libe2fs_sys::ext2fs_block_bitmap);

impl ExtBitmap for ExtBlockBitmap {
    fn is_32bit(&self) -> bool {
        let bitmap = unsafe { *self.0 };
        bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_GENERIC_BITMAP.into()
            || bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_INODE_BITMAP.into()
            || bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_BLOCK_BITMAP.into()
    }

    fn is_64bit(&self) -> bool {
        let bitmap = unsafe { *self.0 };
        bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_GENERIC_BITMAP64.into()
            || bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_INODE_BITMAP64.into()
            || bitmap.magic == libe2fs_sys::EXT2_ET_MAGIC_BLOCK_BITMAP64.into()
    }
}
