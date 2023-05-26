use bitflags::bitflags;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use log::*;

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use self::block::*;
use self::file::*;
use self::inode::*;
use self::io::*;
use self::messages::*;

pub mod block;
pub mod facade;
pub mod file;
pub mod inode;
pub mod io;
pub mod messages;

#[derive(Debug, Clone)]
pub struct ExtFilesystem(Arc<RwLock<libe2fs_sys::ext2_filsys>>, PathBuf);
// SAFETY: I promise I'm doing my best here :sob:
// All accesses to the ext2_filsys pointer are through an RwLock, and then
// libe2fs does its own locking internally if it's compiled w/ support.
unsafe impl Send for ExtFilesystem {}
unsafe impl Sync for ExtFilesystem {}

lazy_static! {
    static ref DEFAULT_IO_MANAGER: IoManager = {
        #[cfg(not(target_os = "windows"))]
        IoManager(Arc::new(RwLock::new(unsafe {
            *libe2fs_sys::unix_io_manager
        })))
    };
}

impl ExtFilesystem {
    pub const ROOT_INODE: u32 = libe2fs_sys::EXT2_ROOT_INO;
    pub const LPF_INODE: u32 = 11;

    pub fn open<P: Into<PathBuf> + std::fmt::Debug>(
        name: P,
        block_size: Option<u32>,
        flags: Option<ExtFilesystemOpenFlags>,
    ) -> Result<Self> {
        // assumes flags=0, superblock=0,
        // from openfs.c:
        /*
         *  Note: if superblock is non-zero, block-size must also be non-zero.
         * 	Superblock and block_size can be zero to use the default size.
         *
         * Valid flags for ext2fs_open()
         *
         * 	EXT2_FLAG_RW	- Open the filesystem for read/write.
         * 	EXT2_FLAG_FORCE - Open the filesystem even if some of the
         *				features aren't supported.
         *	EXT2_FLAG_JOURNAL_DEV_OK - Open an ext3 journal device
         *	EXT2_FLAG_SKIP_MMP - Open without multi-mount protection check.
         *	EXT2_FLAG_64BITS - Allow 64-bit bitfields (needed for large
         *				filesystems)
         */

        let mut fs = MaybeUninit::uninit();
        let name = name.into().canonicalize()?;
        let (err, fs) = unsafe {
            debug!("preparing to open ext filesystem...");
            debug!("input = {name:#?}");
            debug!("opening ext filesystem at '{name:?}'");
            let name = CString::new(name.to_string_lossy().as_bytes())?;
            let io_manager = DEFAULT_IO_MANAGER.clone().0;
            let mut io_manager = io_manager.write().unwrap();
            debug!("got io manager");
            let err = libe2fs_sys::ext2fs_open(
                name.as_ptr(),
                flags.unwrap_or(ExtFilesystemOpenFlags::OPEN_64BIT).bits(),
                0,
                block_size.unwrap_or(0),
                &mut *io_manager,
                fs.as_mut_ptr(),
            );
            (err, fs)
        };

        if err == 0 {
            let fs = unsafe { fs.assume_init() };
            let out = Self(Arc::new(RwLock::new(fs)), name);
            debug!("@ starting setup @");
            out.read_bitmaps()?;

            let lpf_inode = out.read_inode(Self::LPF_INODE);
            if lpf_inode.is_err() {
                debug!("creating missing /lost+found...");
                out.mkdir("/", "lost+found")?;
            }

            debug!("@ finished setup @");

            Ok(out)
        } else {
            report(err)
        }
    }

    pub fn iterate_dir<F, P: Into<PathBuf>>(&self, dir: P, mut f: F) -> Result<()>
    where
        F: FnMut(*mut libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
    {
        let dir = dir.into();
        debug!("iterate dir at {dir:?}");
        let inode = self.find_inode(&dir)?;
        debug!("found inode {}", inode.0);
        let fs = self.0.read().unwrap();

        debug!("creating trampoline...");
        let iterator = get_dir_iterator_trampoline(&f);
        debug!("boing!");

        let err = unsafe {
            debug!("iterating {dir:?} with user-provided iterator...");
            libe2fs_sys::ext2fs_dir_iterate(
                *fs,
                inode.num(),
                0,
                &mut [0u8; 4_096] as *mut _ as *mut i8,
                Some(iterator),
                &mut f as *mut _ as *mut ::std::ffi::c_void,
            )
        };
        if err == 0 {
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn root_inode(&self) -> Result<ExtInode> {
        self.read_inode(Self::ROOT_INODE)
    }

    pub fn read_inode(&self, inode: u32) -> Result<ExtInode> {
        debug!("reading inode {inode}...");
        let mut inode_ptr = MaybeUninit::uninit();
        let err = unsafe {
            libe2fs_sys::ext2fs_read_inode(
                self.0.read().unwrap().as_mut().unwrap(),
                inode,
                inode_ptr.as_mut_ptr(),
            )
        };
        if err == 0 {
            Ok(unsafe { ExtInode(inode, *inode_ptr.assume_init_mut()) })
        } else {
            report(err)
        }
    }

    pub fn find_inode<P: Into<PathBuf>>(&self, path: P) -> Result<ExtInode> {
        let path = path.into();
        debug!("finding inode for {path:?}...");
        let path = CString::new(path.to_str().unwrap())?;
        let mut inode = MaybeUninit::uninit();
        let err = unsafe {
            debug!("naming inode at {path:?}");
            let fs = self.0.read().unwrap();
            libe2fs_sys::ext2fs_namei(
                *fs,
                libe2fs_sys::EXT2_ROOT_INO,
                libe2fs_sys::EXT2_ROOT_INO,
                path.as_ptr(),
                inode.as_mut_ptr(),
            )
        };
        if err == 0 {
            debug!("found inode, reading...");
            self.read_inode(unsafe { *inode.assume_init_mut() })
        } else {
            report(err)
        }
    }

    pub fn lookup<P: Into<PathBuf> + Clone>(&self, dir: P, name: &str) -> Result<ExtInode> {
        {
            let dir = dir.clone();
            debug!("looking up {name} in {:?}...", dir.into());
        }
        let dir_inode_number = self.find_inode(dir)?.0;
        debug!("found dir inode: {dir_inode_number}");

        let name = match name.strip_prefix('/') {
            Some(name) => name,
            None => name,
        };
        let name = CString::new(name)?;

        let mut inode = MaybeUninit::uninit();
        let err = unsafe {
            let fs = self.0.read().unwrap();
            debug!("looking up {name:?} in {dir_inode_number} via ext2fs_lookup...");
            libe2fs_sys::ext2fs_lookup(
                *fs,
                dir_inode_number,
                name.as_ptr(),
                name.as_bytes().len().try_into()?,
                std::ptr::null_mut(),
                inode.as_mut_ptr(),
            )
        };
        if err == 0 {
            self.read_inode(unsafe { inode.assume_init() })
        } else {
            report(err)
        }
    }

    pub fn get_pathname(&self, inode: u32) -> Result<String> {
        debug!("reading pathname for inode {}", inode);
        let mut name = MaybeUninit::<&[i8]>::uninit();
        let err = unsafe {
            libe2fs_sys::ext2fs_get_pathname(
                self.0.read().unwrap().as_mut().unwrap(),
                libe2fs_sys::EXT2_ROOT_INO,
                inode,
                name.as_mut_ptr() as *mut *mut ::std::ffi::c_char,
            )
        };
        let name = unsafe { name.assume_init() };
        debug!("received {} byte(s)", name.len());
        if err == 0 {
            Ok(String::from_utf8(name.iter().map(|i| *i as u8).collect())?)
        } else {
            report(err)
        }
    }

    pub fn open_file(&self, inode: u32, flags: Option<ExtFileOpenFlags>) -> Result<ExtFile> {
        let mut file = MaybeUninit::uninit();
        let err = unsafe {
            libe2fs_sys::ext2fs_file_open2(
                self.0.read().unwrap().as_mut().unwrap(),
                inode,
                std::ptr::null_mut(),
                flags.unwrap_or(ExtFileOpenFlags::empty()).bits(),
                file.as_mut_ptr(),
            )
        };

        if err == 0 {
            Ok(ExtFile(unsafe { file.assume_init() }, ExtFileState::Open))
        } else {
            report(err)
        }
    }

    pub fn close_file(&self, file: &mut ExtFile) -> Result<()> {
        if file.1 == ExtFileState::Closed {
            return Err(eyre!("file already closed!"));
        }

        let err = unsafe { libe2fs_sys::ext2fs_file_close(file.0) };
        if err == 0 {
            file.1 = ExtFileState::Closed;
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn get_inode(&self, file: &ExtFile) -> Result<ExtInode> {
        let inode = unsafe { libe2fs_sys::ext2fs_file_get_inode(file.0) };
        let inode_num = unsafe { libe2fs_sys::ext2fs_file_get_inode_num(file.0) };
        if inode.is_null() {
            Err(ExtError::ENOENT.into())
        } else {
            Ok(ExtInode(inode_num, unsafe { *inode }))
        }
    }

    pub fn get_inode_number(&self, file: &ExtFile) -> Result<u32> {
        let inode = unsafe { libe2fs_sys::ext2fs_file_get_inode_num(file.0) };
        if inode == 0 {
            Err(ExtError::ENOENT.into())
        } else {
            Ok(inode)
        }
    }

    pub fn read_file(&self, file: &ExtFile, buf: &mut [u8]) -> Result<usize> {
        let mut got = MaybeUninit::uninit();
        let err = unsafe {
            libe2fs_sys::ext2fs_file_read(
                file.0,
                buf.as_mut_ptr() as *mut ::std::ffi::c_void,
                buf.len() as u32,
                got.as_mut_ptr(),
            )
        };
        let bytes_read = unsafe { got.assume_init() };
        if bytes_read != buf.len() as u32 {
            debug!("read {} bytes, expected {}", bytes_read, buf.len());
        }
        if err == 0 {
            Ok(bytes_read as usize)
        } else {
            report(err)
        }
    }

    pub fn write_file(&self, file: &ExtFile, buf: &[u8]) -> Result<usize> {
        let mut written = MaybeUninit::uninit();
        debug!("attempting to write {} bytes to {file:?}", buf.len());
        let file = file.0 as *mut libe2fs_sys::real_ext2_file;
        let err = unsafe {
            libe2fs_sys::ext2fs_file_write(
                file as *mut libe2fs_sys::ext2_file,
                buf.as_ptr() as *const ::std::ffi::c_void,
                buf.len() as u32,
                written.as_mut_ptr(),
            )
        };

        if err != 0 {
            return report(err);
        }

        // update the true size of the inode
        unsafe {
            let mut inode = self.read_inode((*file).ino)?;
            inode.1.i_size = buf.len() as u32;
            let err = libe2fs_sys::ext2fs_write_inode(
                self.0.read().unwrap().as_mut().unwrap(),
                (*file).ino,
                &mut inode.1,
            );

            if err != 0 {
                return report(err);
            }
        }

        let err = unsafe { libe2fs_sys::ext2fs_file_flush(file as *mut libe2fs_sys::ext2_file) };
        if err == 0 {
            self.flush()?;
            debug!("write succeeded");
            Ok(unsafe { written.assume_init() } as usize)
        } else {
            report(err)
        }
    }

    pub fn flush_file(&self, file: &ExtFile) -> Result<()> {
        let err = unsafe { libe2fs_sys::ext2fs_file_flush(file.0) };
        if err == 0 {
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn new_inode(&self, dir: u32, mode: u16) -> Result<ExtInode> {
        let mut inode = MaybeUninit::uninit();
        let fs = *self.0.read().unwrap();

        debug!("creating new inode in dir {dir} with mode {mode}");
        let err = unsafe {
            libe2fs_sys::ext2fs_new_inode(
                fs,
                dir,
                libe2fs_sys::LINUX_S_IFREG as i32 | 0o0600,
                (*fs).inode_map,
                inode.as_mut_ptr(),
            )
        };

        if err == 0 {
            let inum = unsafe { inode.assume_init() };
            // let mut inode = self.read_inode(inum)?;
            debug!("created inode: {inum}");
            // once we have the inode, set its mode to be a file
            let mut inode = libe2fs_sys::ext2_inode {
                i_mode: mode | libe2fs_sys::LINUX_S_IFREG as u16,
                i_uid: 0,
                i_size: 0,
                i_atime: 0,
                i_ctime: 0,
                i_mtime: 0,
                i_dtime: 0,
                i_gid: 0,
                i_links_count: 0,
                i_blocks: unsafe { (*fs).blocksize / 512 },
                // set extents flag, since we like modern ext4 features
                i_flags: libe2fs_sys::EXT4_EXTENTS_FL,
                osd1: libe2fs_sys::ext2_inode__bindgen_ty_1 {
                    linux1: libe2fs_sys::ext2_inode__bindgen_ty_1__bindgen_ty_1 { l_i_version: 0 },
                },
                i_block: [0; 15],
                i_generation: 0,
                i_file_acl: 0,
                i_size_high: 0,
                i_faddr: 0,
                osd2: libe2fs_sys::ext2_inode__bindgen_ty_2 {
                    linux2: libe2fs_sys::ext2_inode__bindgen_ty_2__bindgen_ty_1 {
                        l_i_blocks_hi: 0,
                        l_i_file_acl_high: 0,
                        l_i_uid_high: 0,
                        l_i_gid_high: 0,
                        l_i_checksum_lo: 0,
                        l_i_reserved: 0,
                    },
                },
            };

            unsafe {
                let err =
                    libe2fs_sys::ext2fs_iblk_set(fs, &mut inode as *mut libe2fs_sys::ext2_inode, 1);
                if err != 0 {
                    return report(err);
                }
                debug!("iblk_set");
            }

            debug!("attaching data block...");
            // find the next free block and set it on the inode. this value
            // will be written to the blocks bitmap later.
            let data_block = self.new_block(&mut ExtInode(inum, inode))?;
            debug!("data block: {data_block}");
            // TODO: support directories later with ext2fs_new_dir_block!

            // now that we know what our data block is, we need to add it to
            // the inode's extents tree.
            debug!("adding data block to extents tree...");

            unsafe {
                let mut handle = MaybeUninit::uninit();
                let err =
                    libe2fs_sys::ext2fs_extent_open2(fs, inum, &mut inode, handle.as_mut_ptr());
                if err != 0 {
                    return report(err);
                }
                let err =
                    libe2fs_sys::ext2fs_extent_set_bmap(handle.assume_init(), 0, data_block, 0);
                if err != 0 {
                    return report(err);
                }
            }

            debug!("uses {} 512b-i_blocks", inode.i_blocks);

            // flush inode to disk!
            debug!("writing new inode...");
            // update inode group unused inodes area to remove this inode
            // fs, inode, inuse, isdir
            unsafe {
                libe2fs_sys::ext2fs_inode_alloc_stats2(fs, inum, 1, 0);
                libe2fs_sys::ext2fs_block_alloc_stats2(fs, data_block, 1);
            }

            let err = unsafe { libe2fs_sys::ext2fs_write_new_inode(fs, inum, &mut inode) };
            if err == 0 {
                self.flush()?;
                Ok(ExtInode(inum, inode))
            } else {
                report(err)
            }
        } else {
            report(err)
        }
    }

    pub fn new_block(&self, inode: &mut ExtInode) -> Result<u64> {
        let mut block = MaybeUninit::uninit();
        let fs = *self.0.read().unwrap();
        let err = unsafe {
            libe2fs_sys::ext2fs_new_block2(
                fs,
                self.find_inode_goal(inode)?.0,
                std::ptr::null_mut(),
                block.as_mut_ptr(),
            )
        };
        if err == 0 {
            let block = unsafe { block.assume_init() };
            debug!("created block {block}");
            Ok(block)
        } else {
            report(err)
        }
    }

    pub fn find_inode_goal(&self, inode: &mut ExtInode) -> Result<ExtBlock> {
        let fs = *self.0.read().unwrap();
        debug!("finding goal for inode {}", inode.0);
        let block_number =
            unsafe { libe2fs_sys::ext2fs_find_inode_goal(fs, inode.0, std::ptr::null_mut(), 0) };
        Ok(ExtBlock(block_number))
    }

    pub fn next_free_block(&self) -> Result<u64> {
        let fs = *self.0.read().unwrap();
        let mut out = MaybeUninit::<u64>::uninit();
        let res = unsafe {
            // FIXME: THIS IS REALLY STUPID.
            // Just search from the first data block to the end of the fs.
            let fs = *fs;
            let superblock = *fs.super_;
            libe2fs_sys::ext2fs_find_first_zero_generic_bmap(
                fs.block_map,
                superblock.s_first_data_block as u64,
                (superblock.s_blocks_count - superblock.s_first_data_block) as u64,
                out.as_mut_ptr(),
            )
        };
        if res != 0 {
            return report(res);
        }
        let out = unsafe { out.assume_init() };
        debug!("found next free block: block #{out}");
        Ok(out)
    }

    pub fn inode_bitmap(&self) -> ExtInodeBitmap {
        let fs = *self.0.read().unwrap();
        ExtInodeBitmap(unsafe { *fs }.inode_map)
    }

    pub fn block_bitmap(&self) -> ExtBlockBitmap {
        let fs = *self.0.read().unwrap();
        ExtBlockBitmap(unsafe { *fs }.block_map)
    }

    pub fn mkdir<P: Into<PathBuf>, S: Into<String>>(&self, parent: P, name: S) -> Result<()> {
        let parent = parent.into();
        let name = name.into();
        debug!(
            "mkdir {}/{name}",
            parent.display().to_string().trim_end_matches('/')
        );
        let parent_inode = self.find_inode(&parent)?;
        let err = unsafe {
            // pass 0 to automatically allocate new inode
            // http://fs.csl.utoronto.ca/~sunk/libext2fs.html#Creating-and-expanding-directories
            libe2fs_sys::ext2fs_mkdir(
                self.0.read().unwrap().as_mut().unwrap(),
                parent_inode.0,
                0,
                CString::new(name)?.as_ptr(),
            )
        };
        if err == 0 {
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn read_bitmaps(&self) -> Result<()> {
        let err =
            unsafe { libe2fs_sys::ext2fs_read_bitmaps(self.0.read().unwrap().as_mut().unwrap()) };
        if err == 0 {
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn write_bitmaps(&self) -> Result<()> {
        let err = unsafe {
            // libe2fs_sys::ext2fs_write_bitmaps(self.0.read().unwrap().as_mut().unwrap())
            let fs = *self.0.write().unwrap();
            debug!("writing inode bitmap...");
            let err = libe2fs_sys::ext2fs_write_inode_bitmap(&mut *fs as libe2fs_sys::ext2_filsys);
            if err == 0 {
                debug!("writing block bitmap...");
                let err =
                    libe2fs_sys::ext2fs_write_block_bitmap(&mut *fs as libe2fs_sys::ext2_filsys);
                if err == 0 {
                    debug!("done writing bitmaps");
                    0
                } else {
                    err
                }
            } else {
                err
            }
        };
        if err == 0 {
            Ok(())
        } else {
            debug!("writing bitmap failed with error {err}");
            report(err)
        }
    }

    pub fn flush(&self) -> Result<()> {
        let fs = *self.0.write().unwrap();
        unsafe {
            (*fs).flags |= (libe2fs_sys::EXT2_FLAG_DIRTY | libe2fs_sys::EXT2_FLAG_CHANGED) as i32;
        };
        let err = unsafe { libe2fs_sys::ext2fs_flush(fs) };
        if err == 0 {
            Ok(())
        } else {
            report(err)
        }
    }

    pub fn write_to_file<P: Into<PathBuf>>(&self, path: P, buf: &[u8]) -> Result<usize> {
        let fs = *self.0.write().unwrap();
        let path = path.into();

        let inum = unsafe {
            let mut inum = MaybeUninit::<u32>::uninit();
            let err = libe2fs_sys::ext2fs_namei(
                fs,
                Self::ROOT_INODE,
                Self::ROOT_INODE,
                CString::new(path.to_string_lossy().as_bytes())?.as_ptr(),
                inum.as_mut_ptr(),
            );
            if err != 0 {
                debug!("could not find inum, allocating new inode");
                self.new_inode(Self::ROOT_INODE, 0)?.0
            } else {
                inum.assume_init()
            }
        };

        let file = unsafe {
            let mut file = MaybeUninit::<libe2fs_sys::ext2_file_t>::uninit();
            let err = libe2fs_sys::ext2fs_file_open2(
                fs,
                inum,
                std::ptr::null_mut(),
                (ExtFileOpenFlags::CREATE | ExtFileOpenFlags::WRITE).bits(),
                file.as_mut_ptr(),
            );
            if err != 0 {
                return report(err);
            }
            file.assume_init()
        };

        // write buf to file
        let mut written = 0;
        let err = unsafe {
            libe2fs_sys::ext2fs_file_write(
                file,
                buf.as_ptr() as *const libc::c_void,
                buf.len() as u32,
                &mut written,
            )
        };
        if err != 0 {
            return report(err);
        }

        unsafe {
            let fs = *self.0.write().unwrap();
            let mut inode = self.get_inode(&ExtFile(file, ExtFileState::Open))?;
            libe2fs_sys::ext2fs_file_close(file as *mut libe2fs_sys::ext2_file);
            debug!("closed file");
            debug!("inode size: {}", inode.1.i_size);

            inode.1.i_links_count = 1;

            // write this inode
            let err = libe2fs_sys::ext2fs_write_inode(fs, inum, &mut inode.1);
            if err != 0 {
                return report(err);
            }
            debug!("wrote inode");

            // link the inode into the fs hierarchy!
            let parent_inum = self.find_inode(path.parent().unwrap())?.0;
            let file_name = path.file_name().unwrap();
            debug!("linking {file_name:?} @ {inum} to parent inode {parent_inum}");
            let file_name = CString::new(file_name.as_bytes())?;
            let err = libe2fs_sys::ext2fs_link(
                fs,
                parent_inum,
                file_name.as_ptr(),
                inum,
                libe2fs_sys::EXT2_FT_REG_FILE.try_into()?,
            );
            if err != 0 {
                return report(err);
            }

            // update parent inode's counts
            // let mut parent_inode = self.read_inode(parent_inum)?;
            // debug!("found parent inode: {}", parent_inode.0);
            // debug!("parent has links: {}", parent_inode.1.i_links_count);
            // // TODO: FIXME: this doesn't work because ext2fs_link isn't doing the do right...
            // parent_inode.1.i_links_count += 1;
            // let err = libe2fs_sys::ext2fs_write_inode(fs, parent_inum, &mut parent_inode.1);
            // if err != 0 {
            //     return report(err);
            // }
            // debug!("wrote parent inode");
            // debug!("parent has links: {}", parent_inode.1.i_links_count);
        }

        self.flush()?;

        Ok(written as usize)
    }

    pub fn unlink<P: Into<PathBuf>>(&self, path: P) -> Result<()> {
        let fs = *self.0.write().unwrap();
        let path = path.into();
        let file_name = path
            .file_name()
            .expect("cannot unlink files without a name");

        let inode = self.find_inode(&path)?;
        let parent_inum = self
            .find_inode(path.parent().unwrap_or(PathBuf::from("/").as_path()))?
            .0;

        debug!("unlinking {file_name:?}...");
        let err = unsafe {
            libe2fs_sys::ext2fs_unlink(
                fs,
                parent_inum,
                CString::from_vec_unchecked(file_name.as_bytes().to_vec()).as_ptr(),
                inode.0,
                0,
            )
        };

        if err != 0 {
            return report(err);
        }

        Ok(())
    }

    pub fn link<P: Into<PathBuf>>(&self, path: P, new_path: P) -> Result<()> {
        let fs = *self.0.write().unwrap();
        let path = path.into();
        let new_path = new_path.into();
        let file_name = path
            .file_name()
            .expect("cannot unlink files without a name");

        let mut inode = self.find_inode(&path)?;
        let new_parent_inum = self
            .find_inode(new_path.parent().unwrap_or(PathBuf::from("/").as_path()))?
            .0;

        debug!("linking {file_name:?}...");
        let err = unsafe {
            libe2fs_sys::ext2fs_link(
                fs,
                new_parent_inum,
                CString::from_vec_unchecked(file_name.as_bytes().to_vec()).as_ptr(),
                inode.0,
                libe2fs_sys::EXT2_FT_REG_FILE.try_into()?,
            )
        };

        if err != 0 {
            return report(err);
        }

        inode.1.i_links_count += 1;
        let err = unsafe { libe2fs_sys::ext2fs_write_inode(fs, inode.0, &mut inode.1) };
        if err != 0 {
            return report(err);
        }

        Ok(())
    }

    pub fn delete<P: Into<PathBuf>>(&self, path: P) -> Result<()> {
        let fs = *self.0.write().unwrap();
        let path = path.into();
        let file_name = path
            .file_name()
            .expect("cannot unlink files without a name");

        let mut inode = self.find_inode(&path)?;
        // TODO: Is this actually the right behaviour?
        let parent_inum = self
            .find_inode(path.parent().unwrap_or(PathBuf::from("/").as_path()))?
            .0;

        debug!("unlinking {file_name:?}...");
        let err = unsafe {
            libe2fs_sys::ext2fs_unlink(
                fs,
                parent_inum,
                CString::from_vec_unchecked(file_name.as_bytes().to_vec()).as_ptr(),
                inode.0,
                0,
            )
        };
        if err != 0 {
            return report(err);
        }

        inode.1.i_links_count -= 1;
        inode.1.i_dtime = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        let err = unsafe { libe2fs_sys::ext2fs_write_inode(fs, inode.0, &mut inode.1) };
        if err != 0 {
            return report(err);
        }

        // obliterate any remaining blocks
        if unsafe { libe2fs_sys::ext2fs_inode_has_valid_blocks2(fs, &mut inode.1 as *mut _) != 0 } {
            let err = unsafe {
                libe2fs_sys::ext2fs_punch(
                    fs,
                    inode.0,
                    &mut inode.1 as *mut _,
                    std::ptr::null_mut(),
                    0,
                    u64::MAX,
                )
            };
            if err != 0 {
                return report(err);
            }
        }

        unsafe {
            // fs, ino, in_use, is_dir
            libe2fs_sys::ext2fs_inode_alloc_stats2(fs, inode.0, -1, 0);
        }

        self.flush()?;

        Ok(())
    }

    pub fn write_inode(&self, inode: &mut ExtInode) -> Result<()> {
        let err = unsafe {
            libe2fs_sys::ext2fs_write_inode(
                self.0.read().unwrap().as_mut().unwrap(),
                inode.0,
                &mut inode.1,
            )
        };

        if err != 0 {
            report(err)
        } else {
            Ok(())
        }
    }

    // #[cfg(target_os = "windows")]
    // pub fn default_io_manager() -> IoManager {
    //     unimplemented!("Windows support is not yet implemented")
    // }

    // #[cfg(not(target_os = "windows"))]
    // pub fn default_io_manager() -> IoManager {
    //     DEFAULT_IO_MANAGER.clone()
    // }
}

impl Drop for ExtFilesystem {
    fn drop(&mut self) {
        unsafe {
            debug!("drop: writing bitmaps...");
            self.write_bitmaps().unwrap();
            let fs = self.0.write().unwrap();
            debug!("closing fs...");
            let err = libe2fs_sys::ext2fs_close(fs.as_mut().unwrap());
            if err != 0 {
                Err::<(), ExtError>(ExtError::from(err as u32)).unwrap();
            }
        }
    }
}

pub trait ExtBitmap {
    fn is_32bit(&self) -> bool;
    fn is_64bit(&self) -> bool;
}

fn report<T>(error: i64) -> Result<T> {
    if error > 100_000 {
        let err: ExtEtMessage = error.into();
        Err(err.into())
    } else {
        let err: ExtError = (error as u32).into();
        Err(err.into())
    }
}

pub type DirIteratorCallback = unsafe extern "C" fn(
    *mut libe2fs_sys::ext2_dir_entry,
    i32,
    i32,
    *mut i8,
    *mut ::std::ffi::c_void,
) -> i32;

unsafe extern "C" fn dir_iterator_trampoline<F>(
    dir_entry: *mut libe2fs_sys::ext2_dir_entry,
    offset: i32,
    block_size: i32,
    buf: *mut i8,
    user_data: *mut ::std::ffi::c_void,
) -> i32
where
    F: FnMut(*mut libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
{
    let name = CStr::from_ptr(unsafe { *dir_entry }.name.as_ptr())
        .to_str()
        .unwrap();
    debug!("got dir entry: {name}");
    let buf = std::slice::from_raw_parts(buf, block_size as usize);
    debug!("built buf!");
    let user_data = &mut *(user_data as *mut F);
    debug!("invoking user fn!");
    user_data(dir_entry, offset, block_size, name, buf).unwrap()
}

fn get_dir_iterator_trampoline<F>(_closure: &F) -> DirIteratorCallback
where
    F: FnMut(*mut libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
{
    dir_iterator_trampoline::<F>
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ExtFilesystemOpenFlags: i32 {
        const OPEN_RW = libe2fs_sys::EXT2_FLAG_RW as i32;
        const FORCE = libe2fs_sys::EXT2_FLAG_FORCE as i32;
        const JOURNAL_DEV_OK = libe2fs_sys::EXT2_FLAG_JOURNAL_DEV_OK as i32;
        const SKIP_MMP = libe2fs_sys::EXT2_FLAG_SKIP_MMP as i32;
        const OPEN_64BIT = libe2fs_sys::EXT2_FLAG_64BITS as i32;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ExtFileOpenFlags: i32 {
        const WRITE = libe2fs_sys::EXT2_FILE_WRITE as i32;
        const CREATE = libe2fs_sys::EXT2_FILE_CREATE as i32;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;

    use eyre::Result;

    #[ctor::ctor]
    fn initialize() {
        pretty_env_logger::init();
    }

    struct TempImage(PathBuf, TempDir);

    impl TempImage {
        fn new<P: Into<PathBuf>>(path: P) -> Result<Self> {
            let path = path.into();
            let tmp = TempDir::new()?;
            let mut tmp_path = tmp.path_view();
            tmp_path.push(path.file_name().unwrap());
            fs::copy(&path, &tmp_path)?;

            Ok(Self(tmp_path, tmp))
        }

        #[allow(unused)]
        fn path_view(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempImage {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).unwrap();
        }
    }

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn new() -> Result<TempDir> {
            let mut path = std::env::temp_dir();
            path.push(format!("flail-workdir-{}", rand::random::<u64>()));
            fs::create_dir_all(&path)?;

            Ok(TempDir { path })
        }

        pub fn path_view(&self) -> PathBuf {
            self.path.clone()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            if self.path.exists() {
                std::fs::remove_dir_all(&self.path).unwrap();
            }
        }
    }

    #[test]
    pub fn test_reading_directories_works() -> Result<()> {
        let fs = ExtFilesystem::open(
            "./fixtures/hello-world.ext4",
            None,
            Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
        )?;

        fs.iterate_dir(
            "/",
            |dir_entry: *mut libe2fs_sys::ext2_dir_entry,
             _offset,
             _block_size,
             name: &str,
             _priv_data| {
                assert_ne!((unsafe { *dir_entry }).inode, 0);
                debug!("reading inode {}", unsafe { *dir_entry }.inode);
                debug!("got path: {name}!!!");
                assert_ne!(name.len(), 0);
                Ok(0)
            },
        )?;

        Ok(())
    }

    #[test]
    pub fn test_read_write_works() -> Result<()> {
        let img = TempImage::new("./fixtures/empty.ext4")?;

        let fs = ExtFilesystem::open(
            img.path_view(),
            None,
            Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
        )?;

        let data = "hello flail!";

        let inode = fs.new_inode(ExtFilesystem::ROOT_INODE, 0o700)?;
        let written = {
            let file = fs.open_file(
                inode.num(),
                Some(ExtFileOpenFlags::CREATE | ExtFileOpenFlags::WRITE),
            )?;
            debug!("write data: '{data}'");
            let written = fs.write_file(&file, data.as_bytes())?;
            assert_eq!(data.len(), written);
            debug!("wrote {written} bytes");
            written
        };

        {
            let file = fs.open_file(inode.num(), None)?;
            let mut out_buffer = vec![0u8; data.len()];
            let read = fs.read_file(&file, &mut out_buffer)?;
            assert_eq!(written, read);
            debug!("read {read} bytes");
            assert_eq!(data.as_bytes(), out_buffer.as_slice());
        }

        Ok(())
    }

    #[test]
    pub fn test_mkdir_works() -> Result<()> {
        let img = TempImage::new("./fixtures/empty.ext4")?;

        let fs = ExtFilesystem::open(
            img.path_view(),
            None,
            Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
        )?;

        fs.mkdir("/", "foo")?;

        let inode = fs.find_inode("/foo")?;
        assert_eq!(true, inode.0 > 0);

        Ok(())
    }

    #[test]
    pub fn test_passes_fsck() -> Result<()> {
        {
            let img = TempImage::new("./fixtures/empty.ext4")?;

            {
                let fs = ExtFilesystem::open(
                    img.path_view(),
                    None,
                    Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
                )?;

                fs.mkdir("/", "foo")?;
            }

            let fsck = std::process::Command::new("fsck.ext4")
                .arg("-f")
                .arg("-n")
                .arg(img.path_view())
                .spawn()?
                .wait()?;

            assert!(fsck.success());
        }

        {
            let img = TempImage::new("./fixtures/empty.ext4")?;

            {
                // write /test.txt
                let fs = ExtFilesystem::open(
                    img.path_view(),
                    None,
                    Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
                )?;

                let data = "hello flail";

                debug!("write data: '{data}'");
                let written = fs.write_to_file("/test.txt", data.as_bytes())?;

                assert_eq!(data.len(), written);
                debug!("wrote {written} bytes");
            }

            let fsck = std::process::Command::new("fsck.ext4")
                .arg("-f")
                .arg("-n")
                .arg(img.path_view())
                .spawn()?
                .wait()?;

            assert!(fsck.success());

            {
                // read /test.txt
                let fs = ExtFilesystem::open(
                    img.path_view(),
                    None,
                    Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
                )?;

                let mut out_buffer = vec![0u8; 11];

                let inode = fs.lookup("/", "/test.txt")?;
                let file = fs.open_file(inode.0, None)?;
                let read = fs.read_file(&file, &mut out_buffer)?;

                assert_eq!(11, read);
                debug!("read {read} bytes");
                assert_eq!("hello flail", std::str::from_utf8(&out_buffer)?);
            }

            let fsck = std::process::Command::new("fsck.ext4")
                .arg("-f")
                .arg("-n")
                .arg(img.path_view())
                .spawn()?
                .wait()?;

            assert!(fsck.success());

            {
                // unlink /test.txt
                let fs = ExtFilesystem::open(
                    img.path_view(),
                    None,
                    Some(ExtFilesystemOpenFlags::OPEN_64BIT | ExtFilesystemOpenFlags::OPEN_RW),
                )?;

                fs.delete("/test.txt")?;
            }

            let fsck = std::process::Command::new("fsck.ext4")
                .arg("-f")
                .arg("-n")
                .arg(img.path_view())
                .spawn()?
                .wait()?;

            assert!(fsck.success());
        }

        Ok(())
    }
}
