use super::*;

pub struct IoChannel(pub(crate) libe2fs_sys::io_channel);

impl IoChannel {
    // NOTE: I couldn't actually find evidence that this is ever implemented
    // in ext2fs.h outside of the undo io_channel. That io_channel only uses
    // actual, and returns error. :confused:
    pub fn read_error(
        &self,
        _block: u64,
        _count: i32,
        _data: &[u8],
        _size: usize,
        actual_bytes_read: i32,
        error: i64,
    ) -> Result<()> {
        self.with_io_channel(|io_channel| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let read_error_fn = io_channel.read_error.unwrap();
            unsafe {
                read_error_fn(
                    self.0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    0,
                    actual_bytes_read,
                    error,
                )
            }
        })
    }

    // TODO: look at how e2fsck implements these functions...
    pub fn write_error(
        &self,
        _block: u64,
        _count: i32,
        _data: &[u8],
        _size: usize,
        _actual_bytes_written: i32,
        _error: i64,
    ) -> Result<()> {
        unimplemented!("write_error is not yet implemented")
    }

    pub fn block_size(&self) -> i32 {
        unsafe { (*self.0).block_size }
    }

    pub fn refcount(&self) -> i32 {
        unsafe { (*self.0).refcount }
    }

    pub fn flags(&self) -> i32 {
        unsafe { (*self.0).flags }
    }

    pub fn align(&self) -> i32 {
        unsafe { (*self.0).align }
    }

    fn with_io_channel(
        &self,
        f: impl FnOnce(&mut libe2fs_sys::struct_io_channel) -> i64,
    ) -> Result<()> {
        // SAFETY: can never be None because otherwise libe2fs is broken
        unsafe {
            let io_channel = self.0.as_mut().unwrap();
            let out = f(io_channel);
            if out == 0 {
                Ok(())
            } else {
                report(out)
            }
        }
    }

    #[allow(unused)]
    fn with_io_channel_manual<T>(
        &self,
        f: impl FnOnce(&mut libe2fs_sys::struct_io_channel) -> Result<T, ExtError>,
    ) -> Result<T, ExtError> {
        // SAFETY: can never be None because otherwise libe2fs is broken
        unsafe {
            let io_channel = self.0.as_mut().unwrap();
            f(io_channel)
        }
    }
}

#[derive(Clone)]
pub struct IoManager(pub(crate) Arc<RwLock<libe2fs_sys::struct_io_manager>>);

impl IoManager {
    pub fn name(&self) -> Result<String> {
        unsafe {
            let io_manager = (*self.0).read().unwrap();
            Ok(CStr::from_ptr(io_manager.name).to_string_lossy().into())
        }
    }

    pub fn open_channel<S: Into<String>>(&self, device_name: S, flags: i32) -> Result<IoChannel> {
        let name = CString::new(device_name.into()).unwrap();
        let mut channel = std::ptr::null_mut();
        let err = unsafe {
            let io_manager = self.0.write().unwrap();
            // SAFETY: can never be None because otherwise libe2fs is broken
            let open_fn = io_manager.open.unwrap();
            open_fn(name.as_ptr(), flags, &mut channel)
        };
        if err != 0 {
            report(err)
        } else {
            Ok(IoChannel(channel))
        }
    }

    pub fn close(&self, io_channel: IoChannel) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let close_fn = io_manager.close.unwrap();
            unsafe { close_fn(io_channel.0) }
        })
    }

    pub fn set_blksize(&self, io_channel: IoChannel, blk_size: i32) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let set_blksize_fn = io_manager.set_blksize.unwrap();
            unsafe { set_blksize_fn(io_channel.0, blk_size) }
        })
    }

    pub fn read_blk(&self, io_channel: IoChannel, block: u64, count: usize) -> Result<Vec<u8>> {
        self.with_io_manager_manual(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let read_blk_fn = io_manager.read_blk.unwrap();
            let mut data = vec![0u8; count];
            let out = unsafe {
                read_blk_fn(
                    io_channel.0,
                    block,
                    count as i32,
                    data.as_mut_ptr() as *mut _,
                )
            };
            if out == 0 {
                Ok(data)
            } else {
                report(out)
            }
        })
    }

    pub fn write_blk(
        &self,
        io_channel: IoChannel,
        block: u64,
        count: i32,
        data: &[u8],
    ) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let write_blk_fn = io_manager.write_blk.unwrap();
            unsafe { write_blk_fn(io_channel.0, block, count, data.as_ptr() as *const _) }
        })
    }

    pub fn flush(&self, io_channel: IoChannel) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let flush_fn = io_manager.flush.unwrap();
            unsafe { flush_fn(io_channel.0) }
        })
    }

    pub fn write_byte(
        &self,
        io_channel: IoChannel,
        offset: u64,
        count: i32,
        data: &[u8],
    ) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let write_byte_fn = io_manager.write_byte.unwrap();
            unsafe { write_byte_fn(io_channel.0, offset, count, data.as_ptr() as *const _) }
        })
    }

    pub fn set_option<S1: Into<String>, S2: Into<String>>(
        &self,
        io_channel: IoChannel,
        option: S1,
        arg: S2,
    ) -> Result<()> {
        let option = CString::new(option.into()).unwrap();
        let arg = CString::new(arg.into()).unwrap();
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let set_option_fn = io_manager.set_option.unwrap();
            unsafe { set_option_fn(io_channel.0, option.as_ptr(), arg.as_ptr()) }
        })
    }

    pub fn get_stats(&self, io_channel: IoChannel) -> Result<IoStats> {
        self.with_io_manager_manual(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let get_stats_fn = io_manager.get_stats.unwrap();
            let io_stats = std::ptr::null_mut();
            let res = unsafe { get_stats_fn(io_channel.0, io_stats) };
            if res == 0 {
                Ok(IoStats(unsafe { **io_stats }))
            } else {
                report(res)
            }
        })
    }

    pub fn read_blk64(&self, io_channel: IoChannel, block: u64, count: i32) -> Result<Vec<u8>> {
        self.with_io_manager_manual(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let read_blk64_fn = io_manager.read_blk64.unwrap();
            let mut data = vec![0u8; count as usize];
            let res = unsafe {
                read_blk64_fn(
                    io_channel.0,
                    block,
                    count,
                    data.as_mut_ptr() as *mut std::ffi::c_void,
                )
            };
            if res == 0 {
                Ok(data)
            } else {
                report(res)
            }
        })
    }

    pub fn write_blk64(
        &self,
        io_channel: IoChannel,
        block: u64,
        count: i32,
        data: &[u8],
    ) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let write_blk64_fn = io_manager.write_blk64.unwrap();
            unsafe { write_blk64_fn(io_channel.0, block, count, data.as_ptr() as *const _) }
        })
    }

    pub fn discard(&self, io_channel: IoChannel, block: u64, count: u64) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let discard_fn = io_manager.discard.unwrap();
            unsafe { discard_fn(io_channel.0, block, count) }
        })
    }

    pub fn cache_readahead(&self, io_channel: IoChannel, block: u64, count: u64) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let cache_readahead_fn = io_manager.cache_readahead.unwrap();
            unsafe { cache_readahead_fn(io_channel.0, block, count) }
        })
    }

    pub fn zeroout(&self, io_channel: IoChannel, block: u64, count: u64) -> Result<()> {
        self.with_io_manager(|io_manager| {
            // SAFETY: can never be None because otherwise libe2fs is broken
            let zeroout_fn = io_manager.zeroout.unwrap();
            unsafe { zeroout_fn(io_channel.0, block, count) }
        })
    }

    fn with_io_manager(
        &self,
        f: impl FnOnce(&mut libe2fs_sys::struct_io_manager) -> i64,
    ) -> Result<()> {
        // SAFETY: can never be None because otherwise libe2fs is broken
        let mut io_manager = self.0.write().unwrap();
        let out = f(&mut io_manager);
        if out == 0 {
            Ok(())
        } else {
            report(out)
        }
    }

    fn with_io_manager_manual<T>(
        &self,
        f: impl FnOnce(&mut libe2fs_sys::struct_io_manager) -> Result<T>,
    ) -> Result<T> {
        // SAFETY: can never be None because otherwise libe2fs is broken
        let mut io_manager = self.0.write().unwrap();
        f(&mut io_manager)
    }
}

pub struct IoStats(libe2fs_sys::struct_io_stats);

impl IoStats {
    pub fn num_fields(&self) -> i32 {
        self.0.num_fields
    }

    pub fn reserved(&self) -> i32 {
        self.0.reserved
    }

    pub fn bytes_read(&self) -> u64 {
        self.0.bytes_read
    }

    pub fn bytes_written(&self) -> u64 {
        self.0.bytes_written
    }
}
