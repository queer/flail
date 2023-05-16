use super::*;
#[allow(unused)]
use log::*;

/// Files ***MUST*** be closed by their respective filesystem for writes to
/// apply!!!
#[derive(Debug)]
pub struct ExtFile(pub(crate) libe2fs_sys::ext2_file_t, pub(crate) ExtFileState);

impl Drop for ExtFile {
    fn drop(&mut self) {
        if self.1 == ExtFileState::Open {
            debug!("file open, closing on drop!");
            let file = self.0 as *mut libe2fs_sys::ext2_file_64;
            let res =
                unsafe { libe2fs_sys::ext2fs_file_close(file as *mut libe2fs_sys::ext2_file) };
            if res != 0 {
                panic!("{:#?}", Err::<(), ExtError>((res as u32).into()));
            }
            debug!("dropped!");
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum ExtFileState {
    Open,
    Closed,
}
