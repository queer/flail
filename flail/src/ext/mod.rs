use bitflags::bitflags;
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use log::*;

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use self::block::*;
use self::file::*;
use self::inode::*;
use self::io::*;

pub mod block;
pub mod file;
pub mod inode;
pub mod io;

pub struct ExtFilesystem(Arc<RwLock<libe2fs_sys::ext2_filsys>>, PathBuf);

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

        // errcode_t ext2fs_open(const char *name, int flags, int superblock,
        //       unsigned int block_size, io_manager manager,
        //       ext2_filsys *ret_fs)
        //
        let mut fs: libe2fs_sys::ext2_filsys = std::ptr::null_mut();
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
                &mut fs,
            );
            debug!("opened fs with err {err}");
            (err, fs)
        };

        if err == 0 {
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
        F: Fn(&libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
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

            // // in the inode's inode.i_block array, we need to add the data
            // // block as a proper extent entry.
            // // this is the 12-byte header{depth=0}, the 12-byte leaf node, and
            // // the 4-byte checksum.
            // let header = libe2fs_sys::ext3_extent_header {
            //     eh_magic: 0xF30A,
            //     eh_entries: 1,
            //     eh_max: 2,
            //     eh_depth: 0,
            //     eh_generation: 0,
            // };
            // let leaf_node = libe2fs_sys::ext3_extent {
            //     ee_block: data_block as u32,
            //     ee_len: 1,
            //     ee_start_hi: (data_block >> 32) as u16,
            //     ee_start: data_block as u32,
            // };

            // // build it as a byte array for simplicity, then transmute it up
            // // into a [__u32; 15].
            // const I_BLOCK_SIZE: usize = 15 * ::core::mem::size_of::<libe2fs_sys::__u32>();
            // let mut i_block_buffer: [u8; I_BLOCK_SIZE] = [0; I_BLOCK_SIZE];
            // unsafe {
            //     // write header.eh_magic at 0x0
            //     ::core::ptr::copy_nonoverlapping(
            //         header.eh_magic.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr(),
            //         2,
            //     );
            //     // write header.eh_entries at 0x2
            //     ::core::ptr::copy_nonoverlapping(
            //         header.eh_entries.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x2),
            //         2,
            //     );
            //     // write header.eh_max at 0x4
            //     ::core::ptr::copy_nonoverlapping(
            //         header.eh_max.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x4),
            //         2,
            //     );
            //     // write header.eh_depth at 0x6
            //     ::core::ptr::copy_nonoverlapping(
            //         header.eh_depth.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x6),
            //         2,
            //     );
            //     // write header.eh_generation at 0x8
            //     ::core::ptr::copy_nonoverlapping(
            //         header.eh_generation.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x8),
            //         4,
            //     );

            //     // write leaf_node.ee_block at 0xC
            //     ::core::ptr::copy_nonoverlapping(
            //         leaf_node.ee_block.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0xC),
            //         4,
            //     );
            //     // write leaf_node.ee_len at 0x10
            //     ::core::ptr::copy_nonoverlapping(
            //         leaf_node.ee_len.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x10),
            //         2,
            //     );
            //     // write leaf_node.ee_start_hi at 0x12
            //     ::core::ptr::copy_nonoverlapping(
            //         leaf_node.ee_start_hi.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x12),
            //         2,
            //     );
            //     // write leaf_node.ee_start at 0x14
            //     ::core::ptr::copy_nonoverlapping(
            //         leaf_node.ee_start.to_le_bytes().as_mut_ptr(),
            //         i_block_buffer.as_mut_ptr().add(0x14),
            //         4,
            //     );
            // }
            // debug!("wrote extent header + entries");

            // {
            //     // these are the tests that ext2fs_extent_block_csum_set does.
            //     // we do them here so thaat errors can bubble up fast.
            //     assert!(header.eh_magic == 0xF30A);
            //     assert!(header.eh_entries < header.eh_max);
            //     let safe_eh_max =
            //         ((i_block_buffer.len() - 0x18) / 12/* sizeof(ext3_extent) */) as u16;
            //     assert!(header.eh_max <= safe_eh_max);
            //     assert!(header.eh_max >= safe_eh_max - 2);
            // }

            // let err = unsafe {
            //     libe2fs_sys::ext2fs_extent_block_csum_set(
            //         fs,
            //         inum,
            //         i_block_buffer.as_mut_ptr() as *mut _ as *mut libe2fs_sys::ext3_extent_header,
            //     )
            // };
            // debug!("set extent block checksum!");
            // if err != 0 {
            //     return report(err);
            // }

            // debug!("final extent buffer: {:X?}", i_block_buffer);

            // // do final transmutation into i_block shape
            // let final_i_block = unsafe {
            //     ::core::mem::transmute::<[u8; I_BLOCK_SIZE], [libe2fs_sys::__u32; 15]>(
            //         i_block_buffer,
            //     )
            // };
            // inode.i_block = final_i_block;

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

#[derive(thiserror::Error, Debug)]
enum ExtEtMessage {
    #[error("EXT2FS Library version @E2FSPROGS_VERSION@")]
    Base,
    #[error("Wrong magic number for ext2_filsys structure")]
    MagicExt2fsFilsys,
    #[error("Wrong magic number for badblocks_list structure")]
    MagicBadblocksList,
    #[error("Wrong magic number for badblocks_iterate structure")]
    MagicBadblocksIterate,
    #[error("Wrong magic number for inode_scan structure")]
    MagicInodeScan,
    #[error("Wrong magic number for io_channel structure")]
    MagicIoChannel,
    #[error("Wrong magic number for unix io_channel structure")]
    MagicUnixIoChannel,
    #[error("Wrong magic number for io_manager structure")]
    MagicIoManager,
    #[error("Wrong magic number for block_bitmap structure")]
    MagicBlockBitmap,
    #[error("Wrong magic number for inode_bitmap structure")]
    MagicInodeBitmap,
    #[error("Wrong magic number for generic_bitmap structure")]
    MagicGenericBitmap,
    #[error("Wrong magic number for test io_channel structure")]
    MagicTestIoChannel,
    #[error("Wrong magic number for directory block list structure")]
    MagicDbList,
    #[error("Wrong magic number for icount structure")]
    MagicIcount,
    #[error("Wrong magic number for Powerquest io_channel structure")]
    MagicPqIoChannel,
    #[error("Wrong magic number for ext2 file structure")]
    MagicExt2File,
    #[error("Wrong magic number for Ext2 Image Header")]
    MagicE2Image,
    #[error("Wrong magic number for inode io_channel structure")]
    MagicInodeIoChannel,
    #[error("Wrong magic number for ext4 extent handle")]
    MagicExtentHandle,
    #[error("Bad magic number in super-block")]
    BadMagic,
    #[error("Filesystem revision too high")]
    RevTooHigh,
    #[error("Attempt to write to filesystem opened read-only")]
    RoFilsys,
    #[error("Can't read group descriptors")]
    GdescRead,
    #[error("Can't write group descriptors")]
    GdescWrite,
    #[error("Corrupt group descriptor: bad block for block bitmap")]
    GdescBadBlockMap,
    #[error("Corrupt group descriptor: bad block for inode bitmap")]
    GdescBadInodeMap,
    #[error("Corrupt group descriptor: bad block for inode table")]
    GdescBadInodeTable,
    #[error("Can't write an inode bitmap")]
    InodeBitmapWrite,
    #[error("Can't read an inode bitmap")]
    InodeBitmapRead,
    #[error("Can't write a block bitmap")]
    BlockBitmapWrite,
    #[error("Can't read a block bitmap")]
    BlockBitmapRead,
    #[error("Can't write an inode table")]
    InodeTableWrite,
    #[error("Can't read an inode table")]
    InodeTableRead,
    #[error("Can't read next inode")]
    NextInodeRead,
    #[error("Filesystem has unexpected block size")]
    UnexpectedBlockSize,
    #[error("EXT2 directory corrupted")]
    DirCorrupted,
    #[error("Attempt to read block from filesystem resulted in short read")]
    ShortRead,
    #[error("Attempt to write block to filesystem resulted in short write")]
    ShortWrite,
    #[error("No free space in the directory")]
    DirNoSpace,
    #[error("Inode bitmap not loaded")]
    NoInodeBitmap,
    #[error("Block bitmap not loaded")]
    NoBlockBitmap,
    #[error("Illegal inode number")]
    BadInodeNumber,
    #[error("Illegal block number")]
    BadBlockNumber,
    #[error("Internal error in ext2fs_expand_dir")]
    ExpandDirError,
    #[error("Not enough space to build proposed filesystem")]
    TooSmall,
    #[error("Illegal block number passed to ext2fs_mark_block_bitmap")]
    BadBlockMark,
    #[error("Illegal block number passed to ext2fs_unmark_block_bitmap")]
    BadBlockUnmark,
    #[error("Illegal block number passed to ext2fs_test_block_bitmap")]
    BadBlockTest,
    #[error("Illegal inode number passed to ext2fs_mark_inode_bitmap")]
    BadInodeMark,
    #[error("Illegal inode number passed to ext2fs_unmark_inode_bitmap")]
    BadInodeUnmark,
    #[error("Illegal inode number passed to ext2fs_test_inode_bitmap")]
    BadInodeTest,
    #[error("Attempt to fudge end of block bitmap past the real end")]
    FudgeBlockBitmapEnd,
    #[error("Attempt to fudge end of inode bitmap past the real end")]
    FudgeInodeBitmapEnd,
    #[error("Illegal indirect block found")]
    BadIndBlock,
    #[error("Illegal doubly indirect block found")]
    BadDindBlock,
    #[error("Illegal triply indirect block found")]
    BadTindBlock,
    #[error("Block bitmaps are not the same")]
    NeqBlockBitmap,
    #[error("Inode bitmaps are not the same")]
    NeqInodeBitmap,
    #[error("Illegal or malformed device name")]
    BadDeviceName,
    #[error("A block group is missing an inode table")]
    MissingInodeTable,
    #[error("The ext2 superblock is corrupt")]
    CorruptSuperblock,
    #[error("Illegal generic bit number passed to ext2fs_mark_generic_bitmap")]
    BadGenericMark,
    #[error("Illegal generic bit number passed to ext2fs_unmark_generic_bitmap")]
    BadGenericUnmark,
    #[error("Illegal generic bit number passed to ext2fs_test_generic_bitmap")]
    BadGenericTest,
    #[error("Too many symbolic links encountered.")]
    SymlinkLoop,
    #[error("The callback function will not handle this case")]
    CallbackNotHandled,
    #[error("The inode is from a bad block in the inode table")]
    BadBlockInInodeTable,
    #[error("Filesystem has unsupported feature(s)")]
    UnsupportedFeature,
    #[error("Filesystem has unsupported read-only feature(s)")]
    ReadOnlyUnsupportedFeature,
    #[error("IO Channel failed to seek on read or write")]
    LlseekFailed,
    #[error("Memory allocation failed")]
    NoMemory,
    #[error("Invalid argument passed to ext2 library")]
    InvalidArgument,
    #[error("Could not allocate block in ext2 filesystem")]
    BlockAllocFail,
    #[error("Could not allocate inode in ext2 filesystem")]
    InodeAllocFail,
    #[error("Ext2 inode is not a directory")]
    NoDirectory,
    #[error("Too many references in table")]
    TooManyRefs,
    #[error("File not found by ext2_lookup")]
    FileNotFound,
    #[error("File open read-only")]
    FileReadOnly,
    #[error("Ext2 directory block not found")]
    DbNotFound,
    #[error("Ext2 directory already exists")]
    DirExists,
    #[error("Unimplemented ext2 library function")]
    Unimplemented,
    #[error("User cancel requested")]
    CancelRequested,
    #[error("Ext2 file too big")]
    FileTooBig,
    #[error("Supplied journal device not a block device")]
    JournalNotBlock,
    #[error("Journal superblock not found")]
    NoJournalSuperblock,
    #[error("Journal must be at least 1024 blocks")]
    JournalTooSmall,
    #[error("Unsupported journal version")]
    UnsupportedJournalVersion,
    #[error("Error loading external journal")]
    LoadExtJournal,
    #[error("Journal not found")]
    NoJournal,
    #[error("Directory hash unsupported")]
    DirhashUnsupp,
    #[error("Illegal extended attribute block number")]
    BadEABlockNum,
    #[error("Cannot create filesystem with requested number of inodes")]
    TooManyInodes,
    #[error("E2image snapshot not in use")]
    NotImageFile,
    #[error("Too many reserved group descriptor blocks")]
    ResGDTBlocks,
    #[error("Resize inode is corrupt")]
    ResizeInodeCorrupt,
    #[error("Tried to set block bmap with missing indirect block")]
    SetBmapNoInd,
    #[error("TDB: Success")]
    TDBSuccess,
    #[error("TDB: Corrupt database")]
    TDBErrCorrupt,
    #[error("TDB: IO Error")]
    TDBErrIO,
    #[error("TDB: Locking error")]
    TDBErrLock,
    #[error("TDB: Out of memory")]
    TDBErrOOM,
    #[error("TDB: Record exists")]
    TDBErrExists,
    #[error("TDB: Lock exists on other keys")]
    TDBErrNoLock,
    #[error("TDB: Invalid parameter")]
    TDBErrEINVAL,
    #[error("TDB: Record does not exist")]
    TDBErrNoExist,
    #[error("TDB: Write not permitted")]
    TDBErrRDONLY,
    #[error("Ext2fs directory block list is empty")]
    DBListEmpty,
    #[error("Attempt to modify a block mapping via a read-only block iterator")]
    ROBlockIterate,
    #[error("Wrong magic number for ext4 extent saved path")]
    MagicExtentPath,
    #[error("Wrong magic number for 64-bit generic bitmap")]
    MagicGenericBitmap64,
    #[error("Wrong magic number for 64-bit block bitmap")]
    MagicBlockBitmap64,
    #[error("Wrong magic number for 64-bit inode bitmap")]
    MagicInodeBitmap64,
    #[error("Wrong magic number --- RESERVED_13")]
    MagicReserved13,
    #[error("Wrong magic number --- RESERVED_14")]
    MagicReserved14,
    #[error("Wrong magic number --- RESERVED_15")]
    MagicReserved15,
    #[error("Wrong magic number --- RESERVED_16")]
    MagicReserved16,
    #[error("Wrong magic number --- RESERVED_17")]
    MagicReserved17,
    #[error("Wrong magic number --- RESERVED_18")]
    MagicReserved18,
    #[error("Wrong magic number --- RESERVED_19")]
    MagicReserved19,
    #[error("Corrupt extent header")]
    ExtentHeaderBad,
    #[error("Corrupt extent index")]
    ExtentIndexBad,
    #[error("Corrupt extent")]
    ExtentLeafBad,
    #[error("No free space in extent map")]
    ExtentNoSpace,
    #[error("Inode does not use extents")]
    InodeNotExtent,
    #[error("No 'next' extent")]
    ExtentNoNext,
    #[error("No 'previous' extent")]
    ExtentNoPrev,
    #[error("No 'up' extent")]
    ExtentNoUp,
    #[error("No 'down' extent")]
    ExtentNoDown,
    #[error("No current node")]
    NoCurrentNode,
    #[error("Ext2fs operation not supported")]
    OpNotSupported,
    #[error("No room to insert extent in node")]
    CantInsertExtent,
    #[error("Splitting would result in empty node")]
    CantSplitExtent,
    #[error("Extent not found")]
    ExtentNotFound,
    #[error("Operation not supported for inodes containing extents")]
    ExtentNotSupported,
    #[error("Extent length is invalid")]
    ExtentInvalidLength,
    #[error("I/O Channel does not support 64-bit block numbers")]
    IoChannelNoSupport64,
    #[error("Can't check if filesystem is mounted due to missing mtab file")]
    NoMtabFile,
    #[error("Filesystem too large to use legacy bitmaps")]
    CantUseLegacyBitmaps,
    #[error("MMP: invalid magic number")]
    MmpMagicInvalid,
    #[error("MMP: device currently active")]
    MmpFailed,
    #[error("MMP: e2fsck being run")]
    MmpFsckOn,
    #[error("MMP: block number beyond filesystem range")]
    MmpBadBlock,
    #[error("MMP: undergoing an unknown operation")]
    MmpUnknownSeq,
    #[error("MMP: filesystem still in use")]
    MmpChangeAbort,
    #[error("MMP: open with O_DIRECT failed")]
    MmpOpenDirect,
    #[error("Block group descriptor size incorrect")]
    BadDescSize,
    #[error("Inode checksum does not match inode")]
    InodeCsumInvalid,
    #[error("Inode bitmap checksum does not match bitmap")]
    InodeBitmapCsumInvalid,
    #[error("Extent block checksum does not match extent block")]
    ExtentCsumInvalid,
    #[error("Directory block does not have space for checksum")]
    DirNoSpaceForCsum,
    #[error("Directory block checksum does not match directory block")]
    DirCsumInvalid,
    #[error("Extended attribute block checksum does not match block")]
    ExtAttrCsumInvalid,
    #[error("Superblock checksum does not match superblock")]
    SbCsumInvalid,
    #[error("Unknown checksum algorithm")]
    UnknownCsum,
    #[error("MMP block checksum does not match")]
    MmpCsumInvalid,
    #[error("Ext2 file already exists")]
    FileExists,
    #[error("Block bitmap checksum does not match bitmap")]
    BlockBitmapCsumInvalid,
    #[error("Cannot iterate data blocks of an inode containing inline data")]
    InlineDataCantIterate,
    #[error("Extended attribute has an invalid name length")]
    EaBadNameLen,
    #[error("Extended attribute has an invalid value length")]
    EaBadValueSize,
    #[error("Extended attribute has an incorrect hash")]
    BadEaHash,
    #[error("Extended attribute block has a bad header")]
    BadEAHeader,
    #[error("Extended attribute key not found")]
    EAKeyNotFound,
    #[error("Insufficient space to store extended attribute data")]
    EANoSpace,
    #[error("Filesystem is missing ext_attr or inline_data feature")]
    MissingEAFeature,
    #[error("Inode doesn't have inline data")]
    NoInlineData,
    #[error("No block for an inode with inline data")]
    InlineDataNoBlock,
    #[error("No free space in inline data")]
    InlineDataNoSpace,
    #[error("Wrong magic number for extended attribute structure")]
    MagicEAHandle,
    #[error("Inode seems to contain garbage")]
    InodeIsGarbage,
    #[error("Extended attribute has an invalid value offset")]
    EABadValueOffset,
    #[error("Journal flags inconsistent")]
    JournalFlagsWrong,
    #[error("Undo file corrupt")]
    UndoFileCorrupt,
    #[error("Wrong undo file for this filesystem")]
    UndoFileWrong,
    #[error("File system is corrupted")]
    FileSystemCorrupted,
    #[error("Bad CRC detected in file system")]
    BadCRC,
    #[error("The journal superblock is corrupt")]
    CorruptJournalSB,
    #[error("Inode is corrupted")]
    InodeCorrupted,
    #[error("Inode containing extended attribute value is corrupted")]
    EAInodeCorrupted,
    #[error("Group descriptors not loaded")]
    NoGdesc,
    #[error("The internal ext2_filsys data structure appears to be corrupted")]
    FilsysCorrupted,
    #[error("Found cyclic loop in extent tree")]
    ExtentCycle,
    #[error("Operation not supported on an external journal")]
    ExternalJournalNoSupport,
}

impl From<i64> for ExtEtMessage {
    fn from(value: i64) -> Self {
        match value as u32 {
            libe2fs_sys::EXT2_ET_BASE => ExtEtMessage::Base,
            libe2fs_sys::EXT2_ET_MAGIC_EXT2FS_FILSYS => ExtEtMessage::MagicExt2fsFilsys,
            libe2fs_sys::EXT2_ET_MAGIC_BADBLOCKS_LIST => ExtEtMessage::MagicBadblocksList,
            libe2fs_sys::EXT2_ET_MAGIC_BADBLOCKS_ITERATE => ExtEtMessage::MagicBadblocksIterate,
            libe2fs_sys::EXT2_ET_MAGIC_INODE_SCAN => ExtEtMessage::MagicInodeScan,
            libe2fs_sys::EXT2_ET_MAGIC_IO_CHANNEL => ExtEtMessage::MagicIoChannel,
            libe2fs_sys::EXT2_ET_MAGIC_UNIX_IO_CHANNEL => ExtEtMessage::MagicUnixIoChannel,
            libe2fs_sys::EXT2_ET_MAGIC_IO_MANAGER => ExtEtMessage::MagicIoManager,
            libe2fs_sys::EXT2_ET_MAGIC_BLOCK_BITMAP => ExtEtMessage::MagicBlockBitmap,
            libe2fs_sys::EXT2_ET_MAGIC_INODE_BITMAP => ExtEtMessage::MagicInodeBitmap,
            libe2fs_sys::EXT2_ET_MAGIC_GENERIC_BITMAP => ExtEtMessage::MagicGenericBitmap,
            libe2fs_sys::EXT2_ET_MAGIC_TEST_IO_CHANNEL => ExtEtMessage::MagicTestIoChannel,
            libe2fs_sys::EXT2_ET_MAGIC_DBLIST => ExtEtMessage::MagicDbList,
            libe2fs_sys::EXT2_ET_MAGIC_ICOUNT => ExtEtMessage::MagicIcount,
            libe2fs_sys::EXT2_ET_MAGIC_PQ_IO_CHANNEL => ExtEtMessage::MagicPqIoChannel,
            libe2fs_sys::EXT2_ET_MAGIC_EXT2_FILE => ExtEtMessage::MagicExt2File,
            libe2fs_sys::EXT2_ET_MAGIC_E2IMAGE => ExtEtMessage::MagicE2Image,
            libe2fs_sys::EXT2_ET_MAGIC_INODE_IO_CHANNEL => ExtEtMessage::MagicInodeIoChannel,
            libe2fs_sys::EXT2_ET_MAGIC_EXTENT_HANDLE => ExtEtMessage::MagicExtentHandle,
            libe2fs_sys::EXT2_ET_BAD_MAGIC => ExtEtMessage::BadMagic,
            libe2fs_sys::EXT2_ET_REV_TOO_HIGH => ExtEtMessage::RevTooHigh,
            libe2fs_sys::EXT2_ET_RO_FILSYS => ExtEtMessage::RoFilsys,
            libe2fs_sys::EXT2_ET_GDESC_READ => ExtEtMessage::GdescRead,
            libe2fs_sys::EXT2_ET_GDESC_WRITE => ExtEtMessage::GdescWrite,
            libe2fs_sys::EXT2_ET_GDESC_BAD_BLOCK_MAP => ExtEtMessage::GdescBadBlockMap,
            libe2fs_sys::EXT2_ET_GDESC_BAD_INODE_MAP => ExtEtMessage::GdescBadInodeMap,
            libe2fs_sys::EXT2_ET_GDESC_BAD_INODE_TABLE => ExtEtMessage::GdescBadInodeTable,
            libe2fs_sys::EXT2_ET_INODE_BITMAP_WRITE => ExtEtMessage::InodeBitmapWrite,
            libe2fs_sys::EXT2_ET_INODE_BITMAP_READ => ExtEtMessage::InodeBitmapRead,
            libe2fs_sys::EXT2_ET_BLOCK_BITMAP_WRITE => ExtEtMessage::BlockBitmapWrite,
            libe2fs_sys::EXT2_ET_BLOCK_BITMAP_READ => ExtEtMessage::BlockBitmapRead,
            libe2fs_sys::EXT2_ET_INODE_TABLE_WRITE => ExtEtMessage::InodeTableWrite,
            libe2fs_sys::EXT2_ET_INODE_TABLE_READ => ExtEtMessage::InodeTableRead,
            libe2fs_sys::EXT2_ET_NEXT_INODE_READ => ExtEtMessage::NextInodeRead,
            libe2fs_sys::EXT2_ET_UNEXPECTED_BLOCK_SIZE => ExtEtMessage::UnexpectedBlockSize,
            libe2fs_sys::EXT2_ET_DIR_CORRUPTED => ExtEtMessage::DirCorrupted,
            libe2fs_sys::EXT2_ET_SHORT_READ => ExtEtMessage::ShortRead,
            libe2fs_sys::EXT2_ET_SHORT_WRITE => ExtEtMessage::ShortWrite,
            libe2fs_sys::EXT2_ET_DIR_NO_SPACE => ExtEtMessage::DirNoSpace,
            libe2fs_sys::EXT2_ET_NO_INODE_BITMAP => ExtEtMessage::NoInodeBitmap,
            libe2fs_sys::EXT2_ET_NO_BLOCK_BITMAP => ExtEtMessage::NoBlockBitmap,
            libe2fs_sys::EXT2_ET_BAD_INODE_NUM => ExtEtMessage::BadInodeNumber,
            libe2fs_sys::EXT2_ET_BAD_BLOCK_NUM => ExtEtMessage::BadBlockNumber,
            libe2fs_sys::EXT2_ET_EXPAND_DIR_ERR => ExtEtMessage::ExpandDirError,
            libe2fs_sys::EXT2_ET_TOOSMALL => ExtEtMessage::TooSmall,
            libe2fs_sys::EXT2_ET_BAD_BLOCK_MARK => ExtEtMessage::BadBlockMark,
            libe2fs_sys::EXT2_ET_BAD_BLOCK_UNMARK => ExtEtMessage::BadBlockUnmark,
            libe2fs_sys::EXT2_ET_BAD_BLOCK_TEST => ExtEtMessage::BadBlockTest,
            libe2fs_sys::EXT2_ET_BAD_INODE_MARK => ExtEtMessage::BadInodeMark,
            libe2fs_sys::EXT2_ET_BAD_INODE_UNMARK => ExtEtMessage::BadInodeUnmark,
            libe2fs_sys::EXT2_ET_BAD_INODE_TEST => ExtEtMessage::BadInodeTest,
            libe2fs_sys::EXT2_ET_FUDGE_BLOCK_BITMAP_END => ExtEtMessage::FudgeBlockBitmapEnd,
            libe2fs_sys::EXT2_ET_FUDGE_INODE_BITMAP_END => ExtEtMessage::FudgeInodeBitmapEnd,
            libe2fs_sys::EXT2_ET_BAD_IND_BLOCK => ExtEtMessage::BadIndBlock,
            libe2fs_sys::EXT2_ET_BAD_DIND_BLOCK => ExtEtMessage::BadDindBlock,
            libe2fs_sys::EXT2_ET_BAD_TIND_BLOCK => ExtEtMessage::BadTindBlock,
            libe2fs_sys::EXT2_ET_NEQ_BLOCK_BITMAP => ExtEtMessage::NeqBlockBitmap,
            libe2fs_sys::EXT2_ET_NEQ_INODE_BITMAP => ExtEtMessage::NeqInodeBitmap,
            libe2fs_sys::EXT2_ET_BAD_DEVICE_NAME => ExtEtMessage::BadDeviceName,
            libe2fs_sys::EXT2_ET_MISSING_INODE_TABLE => ExtEtMessage::MissingInodeTable,
            libe2fs_sys::EXT2_ET_CORRUPT_SUPERBLOCK => ExtEtMessage::CorruptSuperblock,
            libe2fs_sys::EXT2_ET_BAD_GENERIC_MARK => ExtEtMessage::BadGenericMark,
            libe2fs_sys::EXT2_ET_BAD_GENERIC_UNMARK => ExtEtMessage::BadGenericUnmark,
            libe2fs_sys::EXT2_ET_BAD_GENERIC_TEST => ExtEtMessage::BadGenericTest,
            libe2fs_sys::EXT2_ET_SYMLINK_LOOP => ExtEtMessage::SymlinkLoop,
            libe2fs_sys::EXT2_ET_CALLBACK_NOTHANDLED => ExtEtMessage::CallbackNotHandled,
            libe2fs_sys::EXT2_ET_BAD_BLOCK_IN_INODE_TABLE => ExtEtMessage::BadBlockInInodeTable,
            libe2fs_sys::EXT2_ET_UNSUPP_FEATURE => ExtEtMessage::UnsupportedFeature,
            libe2fs_sys::EXT2_ET_RO_UNSUPP_FEATURE => ExtEtMessage::ReadOnlyUnsupportedFeature,
            libe2fs_sys::EXT2_ET_LLSEEK_FAILED => ExtEtMessage::LlseekFailed,
            libe2fs_sys::EXT2_ET_NO_MEMORY => ExtEtMessage::NoMemory,
            libe2fs_sys::EXT2_ET_INVALID_ARGUMENT => ExtEtMessage::InvalidArgument,
            libe2fs_sys::EXT2_ET_BLOCK_ALLOC_FAIL => ExtEtMessage::BlockAllocFail,
            libe2fs_sys::EXT2_ET_INODE_ALLOC_FAIL => ExtEtMessage::InodeAllocFail,
            libe2fs_sys::EXT2_ET_NO_DIRECTORY => ExtEtMessage::NoDirectory,
            libe2fs_sys::EXT2_ET_TOO_MANY_REFS => ExtEtMessage::TooManyRefs,
            libe2fs_sys::EXT2_ET_FILE_NOT_FOUND => ExtEtMessage::FileNotFound,
            libe2fs_sys::EXT2_ET_FILE_RO => ExtEtMessage::FileReadOnly,
            libe2fs_sys::EXT2_ET_DB_NOT_FOUND => ExtEtMessage::DbNotFound,
            libe2fs_sys::EXT2_ET_DIR_EXISTS => ExtEtMessage::DirExists,
            libe2fs_sys::EXT2_ET_UNIMPLEMENTED => ExtEtMessage::Unimplemented,
            libe2fs_sys::EXT2_ET_CANCEL_REQUESTED => ExtEtMessage::CancelRequested,
            libe2fs_sys::EXT2_ET_FILE_TOO_BIG => ExtEtMessage::FileTooBig,
            libe2fs_sys::EXT2_ET_JOURNAL_NOT_BLOCK => ExtEtMessage::JournalNotBlock,
            libe2fs_sys::EXT2_ET_NO_JOURNAL_SB => ExtEtMessage::NoJournalSuperblock,
            libe2fs_sys::EXT2_ET_JOURNAL_TOO_SMALL => ExtEtMessage::JournalTooSmall,
            libe2fs_sys::EXT2_ET_JOURNAL_UNSUPP_VERSION => ExtEtMessage::UnsupportedJournalVersion,
            libe2fs_sys::EXT2_ET_LOAD_EXT_JOURNAL => ExtEtMessage::LoadExtJournal,
            libe2fs_sys::EXT2_ET_NO_JOURNAL => ExtEtMessage::NoJournal,
            libe2fs_sys::EXT2_ET_DIRHASH_UNSUPP => ExtEtMessage::DirhashUnsupp,
            libe2fs_sys::EXT2_ET_BAD_EA_BLOCK_NUM => ExtEtMessage::BadEABlockNum,
            libe2fs_sys::EXT2_ET_TOO_MANY_INODES => ExtEtMessage::TooManyInodes,
            libe2fs_sys::EXT2_ET_NOT_IMAGE_FILE => ExtEtMessage::NotImageFile,
            libe2fs_sys::EXT2_ET_RES_GDT_BLOCKS => ExtEtMessage::ResGDTBlocks,
            libe2fs_sys::EXT2_ET_RESIZE_INODE_CORRUPT => ExtEtMessage::ResizeInodeCorrupt,
            libe2fs_sys::EXT2_ET_SET_BMAP_NO_IND => ExtEtMessage::SetBmapNoInd,
            libe2fs_sys::EXT2_ET_TDB_SUCCESS => ExtEtMessage::TDBSuccess,
            libe2fs_sys::EXT2_ET_TDB_ERR_CORRUPT => ExtEtMessage::TDBErrCorrupt,
            libe2fs_sys::EXT2_ET_TDB_ERR_IO => ExtEtMessage::TDBErrIO,
            libe2fs_sys::EXT2_ET_TDB_ERR_LOCK => ExtEtMessage::TDBErrLock,
            libe2fs_sys::EXT2_ET_TDB_ERR_OOM => ExtEtMessage::TDBErrOOM,
            libe2fs_sys::EXT2_ET_TDB_ERR_EXISTS => ExtEtMessage::TDBErrExists,
            libe2fs_sys::EXT2_ET_TDB_ERR_NOLOCK => ExtEtMessage::TDBErrNoLock,
            libe2fs_sys::EXT2_ET_TDB_ERR_EINVAL => ExtEtMessage::TDBErrEINVAL,
            libe2fs_sys::EXT2_ET_TDB_ERR_NOEXIST => ExtEtMessage::TDBErrNoExist,
            libe2fs_sys::EXT2_ET_TDB_ERR_RDONLY => ExtEtMessage::TDBErrRDONLY,
            libe2fs_sys::EXT2_ET_DBLIST_EMPTY => ExtEtMessage::DBListEmpty,
            libe2fs_sys::EXT2_ET_RO_BLOCK_ITERATE => ExtEtMessage::ROBlockIterate,
            libe2fs_sys::EXT2_ET_MAGIC_EXTENT_PATH => ExtEtMessage::MagicExtentPath,
            libe2fs_sys::EXT2_ET_MAGIC_GENERIC_BITMAP64 => ExtEtMessage::MagicGenericBitmap64,
            libe2fs_sys::EXT2_ET_MAGIC_BLOCK_BITMAP64 => ExtEtMessage::MagicBlockBitmap64,
            libe2fs_sys::EXT2_ET_MAGIC_INODE_BITMAP64 => ExtEtMessage::MagicInodeBitmap64,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_13 => ExtEtMessage::MagicReserved13,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_14 => ExtEtMessage::MagicReserved14,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_15 => ExtEtMessage::MagicReserved15,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_16 => ExtEtMessage::MagicReserved16,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_17 => ExtEtMessage::MagicReserved17,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_18 => ExtEtMessage::MagicReserved18,
            libe2fs_sys::EXT2_ET_MAGIC_RESERVED_19 => ExtEtMessage::MagicReserved19,
            libe2fs_sys::EXT2_ET_EXTENT_HEADER_BAD => ExtEtMessage::ExtentHeaderBad,
            libe2fs_sys::EXT2_ET_EXTENT_INDEX_BAD => ExtEtMessage::ExtentIndexBad,
            libe2fs_sys::EXT2_ET_EXTENT_LEAF_BAD => ExtEtMessage::ExtentLeafBad,
            libe2fs_sys::EXT2_ET_EXTENT_NO_SPACE => ExtEtMessage::ExtentNoSpace,
            libe2fs_sys::EXT2_ET_INODE_NOT_EXTENT => ExtEtMessage::InodeNotExtent,
            libe2fs_sys::EXT2_ET_EXTENT_NO_NEXT => ExtEtMessage::ExtentNoNext,
            libe2fs_sys::EXT2_ET_EXTENT_NO_PREV => ExtEtMessage::ExtentNoPrev,
            libe2fs_sys::EXT2_ET_EXTENT_NO_UP => ExtEtMessage::ExtentNoUp,
            libe2fs_sys::EXT2_ET_EXTENT_NO_DOWN => ExtEtMessage::ExtentNoDown,
            libe2fs_sys::EXT2_ET_NO_CURRENT_NODE => ExtEtMessage::NoCurrentNode,
            libe2fs_sys::EXT2_ET_OP_NOT_SUPPORTED => ExtEtMessage::OpNotSupported,
            libe2fs_sys::EXT2_ET_CANT_INSERT_EXTENT => ExtEtMessage::CantInsertExtent,
            libe2fs_sys::EXT2_ET_CANT_SPLIT_EXTENT => ExtEtMessage::CantSplitExtent,
            libe2fs_sys::EXT2_ET_EXTENT_NOT_FOUND => ExtEtMessage::ExtentNotFound,
            libe2fs_sys::EXT2_ET_EXTENT_NOT_SUPPORTED => ExtEtMessage::ExtentNotSupported,
            libe2fs_sys::EXT2_ET_EXTENT_INVALID_LENGTH => ExtEtMessage::ExtentInvalidLength,
            libe2fs_sys::EXT2_ET_IO_CHANNEL_NO_SUPPORT_64 => ExtEtMessage::IoChannelNoSupport64,
            libe2fs_sys::EXT2_ET_NO_MTAB_FILE => ExtEtMessage::NoMtabFile,
            libe2fs_sys::EXT2_ET_CANT_USE_LEGACY_BITMAPS => ExtEtMessage::CantUseLegacyBitmaps,
            libe2fs_sys::EXT2_ET_MMP_MAGIC_INVALID => ExtEtMessage::MmpMagicInvalid,
            libe2fs_sys::EXT2_ET_MMP_FAILED => ExtEtMessage::MmpFailed,
            libe2fs_sys::EXT2_ET_MMP_FSCK_ON => ExtEtMessage::MmpFsckOn,
            libe2fs_sys::EXT2_ET_MMP_BAD_BLOCK => ExtEtMessage::MmpBadBlock,
            libe2fs_sys::EXT2_ET_MMP_UNKNOWN_SEQ => ExtEtMessage::MmpUnknownSeq,
            libe2fs_sys::EXT2_ET_MMP_CHANGE_ABORT => ExtEtMessage::MmpChangeAbort,
            libe2fs_sys::EXT2_ET_MMP_OPEN_DIRECT => ExtEtMessage::MmpOpenDirect,
            libe2fs_sys::EXT2_ET_BAD_DESC_SIZE => ExtEtMessage::BadDescSize,
            libe2fs_sys::EXT2_ET_INODE_CSUM_INVALID => ExtEtMessage::InodeCsumInvalid,
            libe2fs_sys::EXT2_ET_INODE_BITMAP_CSUM_INVALID => ExtEtMessage::InodeBitmapCsumInvalid,
            libe2fs_sys::EXT2_ET_EXTENT_CSUM_INVALID => ExtEtMessage::ExtentCsumInvalid,
            libe2fs_sys::EXT2_ET_DIR_NO_SPACE_FOR_CSUM => ExtEtMessage::DirNoSpaceForCsum,
            libe2fs_sys::EXT2_ET_DIR_CSUM_INVALID => ExtEtMessage::DirCsumInvalid,
            libe2fs_sys::EXT2_ET_EXT_ATTR_CSUM_INVALID => ExtEtMessage::ExtAttrCsumInvalid,
            libe2fs_sys::EXT2_ET_SB_CSUM_INVALID => ExtEtMessage::SbCsumInvalid,
            libe2fs_sys::EXT2_ET_UNKNOWN_CSUM => ExtEtMessage::UnknownCsum,
            libe2fs_sys::EXT2_ET_MMP_CSUM_INVALID => ExtEtMessage::MmpCsumInvalid,
            libe2fs_sys::EXT2_ET_FILE_EXISTS => ExtEtMessage::FileExists,
            libe2fs_sys::EXT2_ET_BLOCK_BITMAP_CSUM_INVALID => ExtEtMessage::BlockBitmapCsumInvalid,
            libe2fs_sys::EXT2_ET_INLINE_DATA_CANT_ITERATE => ExtEtMessage::InlineDataCantIterate,
            libe2fs_sys::EXT2_ET_EA_BAD_NAME_LEN => ExtEtMessage::EaBadNameLen,
            libe2fs_sys::EXT2_ET_EA_BAD_VALUE_SIZE => ExtEtMessage::EaBadValueSize,
            libe2fs_sys::EXT2_ET_BAD_EA_HASH => ExtEtMessage::BadEaHash,
            libe2fs_sys::EXT2_ET_BAD_EA_HEADER => ExtEtMessage::BadEAHeader,
            libe2fs_sys::EXT2_ET_EA_KEY_NOT_FOUND => ExtEtMessage::EAKeyNotFound,
            libe2fs_sys::EXT2_ET_EA_NO_SPACE => ExtEtMessage::EANoSpace,
            libe2fs_sys::EXT2_ET_MISSING_EA_FEATURE => ExtEtMessage::MissingEAFeature,
            libe2fs_sys::EXT2_ET_NO_INLINE_DATA => ExtEtMessage::NoInlineData,
            libe2fs_sys::EXT2_ET_INLINE_DATA_NO_BLOCK => ExtEtMessage::InlineDataNoBlock,
            libe2fs_sys::EXT2_ET_INLINE_DATA_NO_SPACE => ExtEtMessage::InlineDataNoSpace,
            libe2fs_sys::EXT2_ET_MAGIC_EA_HANDLE => ExtEtMessage::MagicEAHandle,
            libe2fs_sys::EXT2_ET_INODE_IS_GARBAGE => ExtEtMessage::InodeIsGarbage,
            libe2fs_sys::EXT2_ET_EA_BAD_VALUE_OFFSET => ExtEtMessage::EABadValueOffset,
            libe2fs_sys::EXT2_ET_JOURNAL_FLAGS_WRONG => ExtEtMessage::JournalFlagsWrong,
            libe2fs_sys::EXT2_ET_UNDO_FILE_CORRUPT => ExtEtMessage::UndoFileCorrupt,
            libe2fs_sys::EXT2_ET_UNDO_FILE_WRONG => ExtEtMessage::UndoFileWrong,
            libe2fs_sys::EXT2_ET_FILESYSTEM_CORRUPTED => ExtEtMessage::FileSystemCorrupted,
            libe2fs_sys::EXT2_ET_BAD_CRC => ExtEtMessage::BadCRC,
            libe2fs_sys::EXT2_ET_CORRUPT_JOURNAL_SB => ExtEtMessage::CorruptJournalSB,
            libe2fs_sys::EXT2_ET_INODE_CORRUPTED => ExtEtMessage::InodeCorrupted,
            libe2fs_sys::EXT2_ET_EA_INODE_CORRUPTED => ExtEtMessage::EAInodeCorrupted,
            libe2fs_sys::EXT2_ET_NO_GDESC => ExtEtMessage::NoGdesc,
            libe2fs_sys::EXT2_FILSYS_CORRUPTED => ExtEtMessage::FilsysCorrupted,
            libe2fs_sys::EXT2_ET_EXTENT_CYCLE => ExtEtMessage::ExtentCycle,
            libe2fs_sys::EXT2_ET_EXTERNAL_JOURNAL_NOSUPP => ExtEtMessage::ExternalJournalNoSupport,
            other => unreachable!("unreachable libr2fs error code: {other}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtError {
    #[error("Operation not permitted")]
    EPERM,
    #[error("No such file or directory")]
    ENOENT,
    #[error("No such process")]
    ESRCH,
    #[error("Interrupted system call")]
    EINTR,
    #[error("Input/output error")]
    EIO,
    #[error("Device not configured")]
    ENXIO,
    #[error("Argument list too long")]
    E2BIG,
    #[error("Exec format error")]
    ENOEXEC,
    #[error("Bad file descriptor")]
    EBADF,
    #[error("No child processes")]
    ECHILD,
    #[error("Resource temporarily unavailable")]
    EAGAIN,
    #[error("Cannot allocate memory")]
    ENOMEM,
    #[error("Permission denied")]
    EACCES,
    #[error("Bad address")]
    EFAULT,
    #[error("Block device required")]
    ENOTBLK,
    #[error("Device busy")]
    EBUSY,
    #[error("File exists")]
    EEXIST,
    #[error("Cross-device link")]
    EXDEV,
    #[error("No such device")]
    ENODEV,
    #[error("Not a directory")]
    ENOTDIR,
    #[error("Is a directory")]
    EISDIR,
    #[error("Invalid argument")]
    EINVAL,
    #[error("Too many open files in system")]
    ENFILE,
    #[error("Too many open files")]
    EMFILE,
    #[error("Inappropriate ioctl for device")]
    ENOTTY,
    #[error("Text file busy")]
    ETXTBSY,
    #[error("File too large")]
    EFBIG,
    #[error("No space left on device")]
    ENOSPC,
    #[error("Illegal seek")]
    ESPIPE,
    #[error("Read-only file system")]
    EROFS,
    #[error("Too many links")]
    EMLINK,
    #[error("Broken pipe")]
    EPIPE,
    #[error("Numerical argument out of domain")]
    EDOM,
    #[error("Result too large")]
    ERANGE,
    #[error("Resource deadlock avoided")]
    EDEADLK,
    #[error("File name too long")]
    ENAMETOOLONG,
    #[error("No locks available")]
    ENOLCK,
    #[error("Function not implemented")]
    ENOSYS,
    #[error("Directory not empty")]
    ENOTEMPTY,
    #[error("Too many symbolic links encountered")]
    ELOOP,
    #[error("Operation would block")]
    EWOULDBLOCK,
    #[error("No message of desired type")]
    ENOMSG,
    #[error("Identifier removed")]
    EIDRM,
    #[error("Channel number out of range")]
    ECHRNG,
    #[error("Level 2 not synchronized")]
    EL2NSYNC,
    #[error("Level 3 halted")]
    EL3HLT,
    #[error("Level 3 reset")]
    EL3RST,
    #[error("Link number out of range")]
    ELNRNG,
    #[error("Device not allocated")]
    EUNATCH,
    #[error("No CSI structure available")]
    ENOCSI,
    #[error("Level 2 halted")]
    EL2HLT,
    #[error("Invalid exchange")]
    EBADE,
    #[error("Invalid request descriptor")]
    EBADR,
    #[error("Exchange full")]
    EXFULL,
    #[error("No anode")]
    ENOANO,
    #[error("Invalid request code")]
    EBADRQC,
    #[error("Invalid slot")]
    EBADSLT,
    #[error("Resource deadlock would occur")]
    EDEADLOCK,
    #[error("Bad font file format")]
    EBFONT,
    #[error("Device not a stream")]
    ENOSTR,
    #[error("No data available")]
    ENODATA,
    #[error("Timer expired")]
    ETIME,
    #[error("No message of desired type")]
    ENOSR,
    #[error("Machine is not on the network")]
    ENONET,
    #[error("Package not installed")]
    ENOPKG,
    #[error("Object is remote")]
    EREMOTE,
    #[error("Link has been severed")]
    ENOLINK,
    #[error("Advertise error")]
    EADV,
    #[error("Srmount error")]
    ESRMNT,
    #[error("Communication error on send")]
    ECOMM,
    #[error("Protocol error")]
    EPROTO,
    #[error("Multihop attempted")]
    EMULTIHOP,
    #[error("RFS specific error")]
    EDOTDOT,
    #[error("Bad message")]
    EBADMSG,
    #[error("Value too large for defined data type")]
    EOVERFLOW,
    #[error("Name not unique on network")]
    ENOTUNIQ,
    #[error("File descriptor in bad state")]
    EBADFD,
    #[error("Remote address changed")]
    EREMCHG,
    #[error("Can not access a needed shared library")]
    ELIBACC,
    #[error("Accessing a corrupted shared library")]
    ELIBBAD,
    #[error(".lib section in a.out corrupted")]
    ELIBSCN,
    #[error("Attempting to link in too many shared libraries")]
    ELIBMAX,
    #[error("Cannot exec a shared library directly")]
    ELIBEXEC,
    #[error("Illegal byte sequence")]
    EILSEQ,
    #[error("Interrupted system call should be restarted")]
    ERESTART,
    #[error("Streams pipe error")]
    ESTRPIPE,
    #[error("Too many users")]
    EUSERS,
    #[error("Socket operation on non-socket")]
    ENOTSOCK,
    #[error("Destination address required")]
    EDESTADDRREQ,
    #[error("Message too long")]
    EMSGSIZE,
    #[error("Protocol wrong type for socket")]
    EPROTOTYPE,
    #[error("Protocol not available")]
    ENOPROTOOPT,
    #[error("Protocol not supported")]
    EPROTONOSUPPORT,
    #[error("Socket type not supported")]
    ESOCKTNOSUPPORT,
    #[error("Operation not supported on transport endpoint")]
    EOPNOTSUPP,
    #[error("Protocol family not supported")]
    EPFNOSUPPORT,
    #[error("Address family not supported")]
    EAFNOSUPPORT,
    #[error("Address already in use")]
    EADDRINUSE,
    #[error("Cannot assign requested address")]
    EADDRNOTAVAIL,
    #[error("Network is down")]
    ENETDOWN,
    #[error("Network is unreachable")]
    ENETUNREACH,
    #[error("Network dropped connection because of reset")]
    ENETRESET,
    #[error("Software caused connection abort")]
    ECONNABORTED,
    #[error("Connection reset by peer")]
    ECONNRESET,
    #[error("No buffer space available")]
    ENOBUFS,
    #[error("Transport endpoint is already connected")]
    EISCONN,
    #[error("Transport endpoint is not connected")]
    ENOTCONN,
    #[error("Cannot send after transport endpoint shutdown")]
    ESHUTDOWN,
    #[error("Too many references: cannot splice")]
    ETOOMANYREFS,
    #[error("Connection timed out")]
    ETIMEDOUT,
    #[error("Connection refused")]
    ECONNREFUSED,
    #[error("Host is down")]
    EHOSTDOWN,
    #[error("No route to host")]
    EHOSTUNREACH,
    #[error("Operation already in progress")]
    EALREADY,
    #[error("Operation now in progress")]
    EINPROGRESS,
    #[error("Stale file handle")]
    ESTALE,
    #[error("Structure needs cleaning")]
    EUCLEAN,
    #[error("Not a XENIX named type file")]
    ENOTNAM,
    #[error("No XENIX semaphores available")]
    ENAVAIL,
    #[error("Is a named type file")]
    EISNAM,
    #[error("Remote I/O error")]
    EREMOTEIO,
    #[error("Disk quota exceeded")]
    EDQUOT,
    #[error("No medium found")]
    ENOMEDIUM,
    #[error("Wrong medium type")]
    EMEDIUMTYPE,
    #[error("Operation canceled")]
    ECANCELED,
    #[error("Required key not available")]
    ENOKEY,
    #[error("Key has expired")]
    EKEYEXPIRED,
    #[error("Key has been revoked")]
    EKEYREVOKED,
    #[error("Key was rejected by service")]
    EKEYREJECTED,
    #[error("Owner died")]
    EOWNERDEAD,
    #[error("State not recoverable")]
    ENOTRECOVERABLE,
    #[error("Operation not possible due to RF-kill")]
    ERFKILL,
    #[error("Memory page has hardware error")]
    EHWPOISON,
    #[error("Operation not supported")]
    ENOTSUP,
    #[error("Unknown error code: {0}")]
    Unknown(u32),
}

impl From<ExtError> for u32 {
    fn from(value: ExtError) -> Self {
        match value {
            ExtError::EPERM => 1,
            ExtError::ENOENT => 2,
            ExtError::ESRCH => 3,
            ExtError::EINTR => 4,
            ExtError::EIO => 5,
            ExtError::ENXIO => 6,
            ExtError::E2BIG => 7,
            ExtError::ENOEXEC => 8,
            ExtError::EBADF => 9,
            ExtError::ECHILD => 10,
            ExtError::EAGAIN => 11,
            ExtError::ENOMEM => 12,
            ExtError::EACCES => 13,
            ExtError::EFAULT => 14,
            ExtError::ENOTBLK => 15,
            ExtError::EBUSY => 16,
            ExtError::EEXIST => 17,
            ExtError::EXDEV => 18,
            ExtError::ENODEV => 19,
            ExtError::ENOTDIR => 20,
            ExtError::EISDIR => 21,
            ExtError::EINVAL => 22,
            ExtError::ENFILE => 23,
            ExtError::EMFILE => 24,
            ExtError::ENOTTY => 25,
            ExtError::ETXTBSY => 26,
            ExtError::EFBIG => 27,
            ExtError::ENOSPC => 28,
            ExtError::ESPIPE => 29,
            ExtError::EROFS => 30,
            ExtError::EMLINK => 31,
            ExtError::EPIPE => 32,
            ExtError::EDOM => 33,
            ExtError::ERANGE => 34,
            ExtError::EDEADLK => 35,
            ExtError::ENAMETOOLONG => 36,
            ExtError::ENOLCK => 37,
            ExtError::ENOSYS => 38,
            ExtError::ENOTEMPTY => 39,
            ExtError::ELOOP => 40,
            ExtError::EWOULDBLOCK => 11,
            ExtError::ENOMSG => 42,
            ExtError::EIDRM => 43,
            ExtError::ECHRNG => 44,
            ExtError::EL2NSYNC => 45,
            ExtError::EL3HLT => 46,
            ExtError::EL3RST => 47,
            ExtError::ELNRNG => 48,
            ExtError::EUNATCH => 49,
            ExtError::ENOCSI => 50,
            ExtError::EL2HLT => 51,
            ExtError::EBADE => 52,
            ExtError::EBADR => 53,
            ExtError::EXFULL => 54,
            ExtError::ENOANO => 55,
            ExtError::EBADRQC => 56,
            ExtError::EBADSLT => 57,
            ExtError::EDEADLOCK => 35,
            ExtError::EBFONT => 59,
            ExtError::ENOSTR => 60,
            ExtError::ENODATA => 61,
            ExtError::ETIME => 62,
            ExtError::ENOSR => 63,
            ExtError::ENONET => 64,
            ExtError::ENOPKG => 65,
            ExtError::EREMOTE => 66,
            ExtError::ENOLINK => 67,
            ExtError::EADV => 68,
            ExtError::ESRMNT => 69,
            ExtError::ECOMM => 70,
            ExtError::EPROTO => 71,
            ExtError::EMULTIHOP => 72,
            ExtError::EDOTDOT => 73,
            ExtError::EBADMSG => 74,
            ExtError::EOVERFLOW => 75,
            ExtError::ENOTUNIQ => 76,
            ExtError::EBADFD => 77,
            ExtError::EREMCHG => 78,
            ExtError::ELIBACC => 79,
            ExtError::ELIBBAD => 80,
            ExtError::ELIBSCN => 81,
            ExtError::ELIBMAX => 82,
            ExtError::ELIBEXEC => 83,
            ExtError::EILSEQ => 84,
            ExtError::ERESTART => 85,
            ExtError::ESTRPIPE => 86,
            ExtError::EUSERS => 87,
            ExtError::ENOTSOCK => 88,
            ExtError::EDESTADDRREQ => 89,
            ExtError::EMSGSIZE => 90,
            ExtError::EPROTOTYPE => 91,
            ExtError::ENOPROTOOPT => 92,
            ExtError::EPROTONOSUPPORT => 93,
            ExtError::ESOCKTNOSUPPORT => 94,
            ExtError::EOPNOTSUPP => 95,
            ExtError::EPFNOSUPPORT => 96,
            ExtError::EAFNOSUPPORT => 97,
            ExtError::EADDRINUSE => 98,
            ExtError::EADDRNOTAVAIL => 99,
            ExtError::ENETDOWN => 100,
            ExtError::ENETUNREACH => 101,
            ExtError::ENETRESET => 102,
            ExtError::ECONNABORTED => 103,
            ExtError::ECONNRESET => 104,
            ExtError::ENOBUFS => 105,
            ExtError::EISCONN => 106,
            ExtError::ENOTCONN => 107,
            ExtError::ESHUTDOWN => 108,
            ExtError::ETOOMANYREFS => 109,
            ExtError::ETIMEDOUT => 110,
            ExtError::ECONNREFUSED => 111,
            ExtError::EHOSTDOWN => 112,
            ExtError::EHOSTUNREACH => 113,
            ExtError::EALREADY => 114,
            ExtError::EINPROGRESS => 115,
            ExtError::ESTALE => 116,
            ExtError::EUCLEAN => 117,
            ExtError::ENOTNAM => 118,
            ExtError::ENAVAIL => 119,
            ExtError::EISNAM => 120,
            ExtError::EREMOTEIO => 121,
            ExtError::EDQUOT => 122,
            ExtError::ENOMEDIUM => 123,
            ExtError::EMEDIUMTYPE => 124,
            ExtError::ECANCELED => 125,
            ExtError::ENOKEY => 126,
            ExtError::EKEYEXPIRED => 127,
            ExtError::EKEYREVOKED => 128,
            ExtError::EKEYREJECTED => 129,
            ExtError::EOWNERDEAD => 130,
            ExtError::ENOTRECOVERABLE => 131,
            ExtError::ERFKILL => 132,
            ExtError::EHWPOISON => 133,
            ExtError::ENOTSUP => 95,
            ExtError::Unknown(other) => other,
        }
    }
}

impl From<u32> for ExtError {
    fn from(value: u32) -> ExtError {
        match value {
            1 => ExtError::EPERM,
            2 => ExtError::ENOENT,
            3 => ExtError::ESRCH,
            4 => ExtError::EINTR,
            5 => ExtError::EIO,
            6 => ExtError::ENXIO,
            7 => ExtError::E2BIG,
            8 => ExtError::ENOEXEC,
            9 => ExtError::EBADF,
            10 => ExtError::ECHILD,
            11 => ExtError::EAGAIN,
            12 => ExtError::ENOMEM,
            13 => ExtError::EACCES,
            14 => ExtError::EFAULT,
            15 => ExtError::ENOTBLK,
            16 => ExtError::EBUSY,
            17 => ExtError::EEXIST,
            18 => ExtError::EXDEV,
            19 => ExtError::ENODEV,
            20 => ExtError::ENOTDIR,
            21 => ExtError::EISDIR,
            22 => ExtError::EINVAL,
            23 => ExtError::ENFILE,
            24 => ExtError::EMFILE,
            25 => ExtError::ENOTTY,
            26 => ExtError::ETXTBSY,
            27 => ExtError::EFBIG,
            28 => ExtError::ENOSPC,
            29 => ExtError::ESPIPE,
            30 => ExtError::EROFS,
            31 => ExtError::EMLINK,
            32 => ExtError::EPIPE,
            33 => ExtError::EDOM,
            34 => ExtError::ERANGE,
            35 => ExtError::EDEADLK,
            36 => ExtError::ENAMETOOLONG,
            37 => ExtError::ENOLCK,
            38 => ExtError::ENOSYS,
            39 => ExtError::ENOTEMPTY,
            40 => ExtError::ELOOP,
            // 11 => ExtError::EWOULDBLOCK,
            42 => ExtError::ENOMSG,
            43 => ExtError::EIDRM,
            44 => ExtError::ECHRNG,
            45 => ExtError::EL2NSYNC,
            46 => ExtError::EL3HLT,
            47 => ExtError::EL3RST,
            48 => ExtError::ELNRNG,
            49 => ExtError::EUNATCH,
            50 => ExtError::ENOCSI,
            51 => ExtError::EL2HLT,
            52 => ExtError::EBADE,
            53 => ExtError::EBADR,
            54 => ExtError::EXFULL,
            55 => ExtError::ENOANO,
            56 => ExtError::EBADRQC,
            57 => ExtError::EBADSLT,
            // 35 => ExtError::EDEADLOCK,
            59 => ExtError::EBFONT,
            60 => ExtError::ENOSTR,
            61 => ExtError::ENODATA,
            62 => ExtError::ETIME,
            63 => ExtError::ENOSR,
            64 => ExtError::ENONET,
            65 => ExtError::ENOPKG,
            66 => ExtError::EREMOTE,
            67 => ExtError::ENOLINK,
            68 => ExtError::EADV,
            69 => ExtError::ESRMNT,
            70 => ExtError::ECOMM,
            71 => ExtError::EPROTO,
            72 => ExtError::EMULTIHOP,
            73 => ExtError::EDOTDOT,
            74 => ExtError::EBADMSG,
            75 => ExtError::EOVERFLOW,
            76 => ExtError::ENOTUNIQ,
            77 => ExtError::EBADFD,
            78 => ExtError::EREMCHG,
            79 => ExtError::ELIBACC,
            80 => ExtError::ELIBBAD,
            81 => ExtError::ELIBSCN,
            82 => ExtError::ELIBMAX,
            83 => ExtError::ELIBEXEC,
            84 => ExtError::EILSEQ,
            85 => ExtError::ERESTART,
            86 => ExtError::ESTRPIPE,
            87 => ExtError::EUSERS,
            88 => ExtError::ENOTSOCK,
            89 => ExtError::EDESTADDRREQ,
            90 => ExtError::EMSGSIZE,
            91 => ExtError::EPROTOTYPE,
            92 => ExtError::ENOPROTOOPT,
            93 => ExtError::EPROTONOSUPPORT,
            94 => ExtError::ESOCKTNOSUPPORT,
            95 => ExtError::EOPNOTSUPP,
            96 => ExtError::EPFNOSUPPORT,
            97 => ExtError::EAFNOSUPPORT,
            98 => ExtError::EADDRINUSE,
            99 => ExtError::EADDRNOTAVAIL,
            100 => ExtError::ENETDOWN,
            101 => ExtError::ENETUNREACH,
            102 => ExtError::ENETRESET,
            103 => ExtError::ECONNABORTED,
            104 => ExtError::ECONNRESET,
            105 => ExtError::ENOBUFS,
            106 => ExtError::EISCONN,
            107 => ExtError::ENOTCONN,
            108 => ExtError::ESHUTDOWN,
            109 => ExtError::ETOOMANYREFS,
            110 => ExtError::ETIMEDOUT,
            111 => ExtError::ECONNREFUSED,
            112 => ExtError::EHOSTDOWN,
            113 => ExtError::EHOSTUNREACH,
            114 => ExtError::EALREADY,
            115 => ExtError::EINPROGRESS,
            116 => ExtError::ESTALE,
            117 => ExtError::EUCLEAN,
            118 => ExtError::ENOTNAM,
            119 => ExtError::ENAVAIL,
            120 => ExtError::EISNAM,
            121 => ExtError::EREMOTEIO,
            122 => ExtError::EDQUOT,
            123 => ExtError::ENOMEDIUM,
            124 => ExtError::EMEDIUMTYPE,
            125 => ExtError::ECANCELED,
            126 => ExtError::ENOKEY,
            127 => ExtError::EKEYEXPIRED,
            128 => ExtError::EKEYREVOKED,
            129 => ExtError::EKEYREJECTED,
            130 => ExtError::EOWNERDEAD,
            131 => ExtError::ENOTRECOVERABLE,
            132 => ExtError::ERFKILL,
            133 => ExtError::EHWPOISON,
            // 95 => ExtError::ENOTSUP,
            other => ExtError::Unknown(other),
        }
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
    F: Fn(&libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
{
    let dir_entry: &libe2fs_sys::ext2_dir_entry = &*dir_entry;
    let name = CStr::from_ptr(dir_entry.name.as_ptr()).to_str().unwrap();
    debug!("got dir entry: {name}");
    let buf = std::slice::from_raw_parts(buf, block_size as usize);
    debug!("built buf!");
    let user_data = &mut *(user_data as *mut F);
    debug!("invoking user fn!");
    user_data(dir_entry, offset, block_size, name, buf).unwrap()
}

fn get_dir_iterator_trampoline<F>(_closure: &F) -> DirIteratorCallback
where
    F: Fn(&libe2fs_sys::ext2_dir_entry, i32, i32, &str, &[i8]) -> Result<i32>,
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
            |dir_entry: &libe2fs_sys::ext2_dir_entry, _offset, _block_size, name, _priv_data| {
                assert_ne!(dir_entry.inode, 0);
                debug!("reading inode {}", dir_entry.inode);
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
        }

        Ok(())
    }
}
