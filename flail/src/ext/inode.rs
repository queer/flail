use super::*;

pub struct ExtInode(pub(crate) u32, pub(crate) libe2fs_sys::ext2_inode);

impl ExtInode {
    pub fn num(&self) -> u32 {
        self.0
    }

    pub fn mode(&self) -> u16 {
        self.1.i_mode
    }
}

// We don't implement Drop on the bitmaps because that fucks up a number of
// things around magic verification.
#[derive(Clone)]
pub struct ExtInodeBitmap(pub(crate) libe2fs_sys::ext2fs_inode_bitmap);

impl ExtBitmap for ExtInodeBitmap {
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
