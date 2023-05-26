use std::time::SystemTime;

use super::*;

#[derive(Copy, Clone)]
pub struct ExtInode(pub(crate) u32, pub(crate) libe2fs_sys::ext2_inode);

impl ExtInode {
    pub fn num(&self) -> u32 {
        self.0
    }

    pub fn mode(&self) -> u16 {
        self.1.i_mode
    }

    pub fn is_dir(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFDIR == libe2fs_sys::LINUX_S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFREG == libe2fs_sys::LINUX_S_IFREG
    }

    pub fn is_symlink(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFLNK == libe2fs_sys::LINUX_S_IFLNK
    }

    pub fn is_block_device(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFBLK == libe2fs_sys::LINUX_S_IFBLK
    }

    pub fn is_char_device(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFCHR == libe2fs_sys::LINUX_S_IFCHR
    }

    pub fn is_fifo(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFIFO == libe2fs_sys::LINUX_S_IFIFO
    }

    pub fn is_socket(&self) -> bool {
        (self.1.i_mode as u32) & libe2fs_sys::LINUX_S_IFSOCK == libe2fs_sys::LINUX_S_IFSOCK
    }

    pub fn size(&self) -> u64 {
        // TODO: This is wrong for 64-bit inodes...? What's the right containing struct? large inode?
        self.1.i_size as u64
    }

    pub fn atime(&self) -> Result<SystemTime> {
        let time = self.1.i_atime;
        Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time as u64))
    }

    pub fn ctime(&self) -> Result<SystemTime> {
        let time = self.1.i_ctime;
        Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time as u64))
    }

    pub fn mtime(&self) -> Result<SystemTime> {
        let time = self.1.i_mtime;
        Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time as u64))
    }

    pub fn dtime(&self) -> Result<SystemTime> {
        let time = self.1.i_dtime;
        Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time as u64))
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
