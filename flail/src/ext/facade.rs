use std::ffi::{CString, OsString};
use std::io::Result;
use std::os::unix::prelude::OsStringExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;

use debug_ignore::DebugIgnore;
use floppy_disk::prelude::*;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};
use tokio::sync::RwLock;

use super::inode::ExtInode;

#[derive(Debug, Clone)]
pub struct ExtFacadeFloppyDisk {
    fs: Arc<RwLock<super::ExtFilesystem>>,
}

unsafe impl Send for ExtFacadeFloppyDisk {}
unsafe impl Sync for ExtFacadeFloppyDisk {}

impl ExtFacadeFloppyDisk {
    pub fn new<P: Into<PathBuf> + std::fmt::Debug>(path: P) -> Result<Self> {
        Ok(Self {
            fs: Arc::new(RwLock::new(
                super::ExtFilesystem::open(path, None, None).map_err(wrap_report)?,
            )),
        })
    }
}

#[async_trait::async_trait]
impl<'a> FloppyDisk<'a> for ExtFacadeFloppyDisk {
    type Metadata = ExtFacadeMetadata;
    type ReadDir = ExtFacadeReadDir;
    type Permissions = ExtFacadePermissions;
    type DirBuilder = ExtFacadeDirBuilder;

    async fn canonicalize<P: AsRef<Path> + Send>(&self, path: P) -> Result<PathBuf> {
        unimplemented!(
            "canonicalize does not have any meaning as everything is already relative to root"
        )
    }

    async fn copy<P: AsRef<Path> + Send>(&self, from: P, to: P) -> Result<u64> {
        todo!()
    }

    async fn create_dir<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        let fs = self.fs.write().await;
        match fs.find_inode(path.as_ref()) {
            Ok(_) => fs
                .mkdir(
                    path.as_ref().parent().unwrap_or(&PathBuf::from("/")),
                    path.as_ref()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                )
                .map_err(wrap_report),
            Err(err) => {
                // rewrap and throw
                Err(wrap_report(err))
            }
        }
    }

    async fn create_dir_all<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        let mut parent_paths = vec![];
        let fs = self.fs.write().await;

        let path = path.as_ref().to_path_buf();
        let mut parent = path.parent();
        while let Some(real_parent) = parent {
            if let Ok(inode) = fs.find_inode(real_parent) {
                if inode.is_dir() {
                    break;
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("{} is not a directory", real_parent.display()),
                    ));
                }
            }

            parent_paths.push(real_parent.to_path_buf());
            parent = real_parent.parent();
        }

        parent_paths.reverse();

        // TODO: This might break somehow, right?
        let mut path_to = parent.unwrap().to_path_buf();
        for path in parent_paths {
            fs.mkdir(
                &path_to,
                path.file_name().unwrap().to_string_lossy().to_string(),
            )
            .map_err(wrap_report)?;
            path_to.push(path.file_name().unwrap());
        }

        Ok(())
    }

    async fn hard_link<P: AsRef<Path> + Send>(&self, src: P, dst: P) -> Result<()> {
        unimplemented!("please open an issue if you need hard-link functionality.")
    }

    async fn metadata<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::Metadata> {
        let fs = self.fs.read().await;
        match fs.find_inode(path.as_ref()) {
            Ok(inode) => Ok(ExtFacadeMetadata {
                inode: DebugIgnore(inode),
            }),
            Err(err) => Err(wrap_report(err)),
        }
    }

    async fn read<P: AsRef<Path> + Send>(&self, path: P) -> Result<Vec<u8>> {
        let fs = self.fs.read().await;
        match fs.find_inode(path.as_ref()) {
            Ok(inode) => {
                if !inode.is_file() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "not a file",
                    ));
                }

                let file = fs.open_file(inode.0, None).map_err(wrap_report)?;
                let mut buf = vec![0; inode.size() as usize];
                fs.read_file(&file, &mut buf).map_err(wrap_report)?;

                Ok(buf)
            }
            Err(err) => Err(wrap_report(err)),
        }
    }

    async fn read_dir<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::ReadDir> {
        let fs = self.fs.read().await;
        let mut inodes = vec![];
        let path = path.as_ref();
        fs.iterate_dir(path, |dir_entry, _offset, _blocksize, _buf, _priv_data| {
            inodes.push((unsafe { *dir_entry }, unsafe { *dir_entry }.inode));
            Ok(0)
        })
        .map_err(wrap_report)?;

        let inodes: Vec<(ExtInode, _)> = inodes
            .iter()
            .map(|(entry, inum)| {
                let inode = fs
                    .read_inode(*inum)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                    .unwrap();
                (inode, *entry)
            })
            .collect();

        Ok(ExtFacadeReadDir::new(path, inodes))
    }

    async fn read_link<P: AsRef<Path> + Send>(&self, path: P) -> Result<PathBuf> {
        let fs = self.fs.read().await;
        match fs.find_inode(path.as_ref()) {
            Ok(inode) => {
                if !inode.is_symlink() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "not a symlink",
                    ));
                }

                let file = fs.open_file(inode.0, None).map_err(wrap_report)?;
                let mut buf = vec![0; inode.size() as usize];
                fs.read_file(&file, &mut buf).map_err(wrap_report)?;

                Ok(PathBuf::from(std::str::from_utf8(&buf).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, err)
                })?))
            }
            Err(err) => Err(wrap_report(err)),
        }
    }

    async fn read_to_string<P: AsRef<Path> + Send>(&self, path: P) -> Result<String> {
        let bytes = self.read(path).await?;
        Ok(String::from_utf8(bytes).map_err(wrap_err)?)
    }

    async fn remove_dir<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let read_dir = self.read_dir(path).await?;
        if !read_dir.inodes.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "directory not empty",
            ));
        }

        let fs = self.fs.write().await;
        fs.delete(path).map_err(wrap_report)
    }

    async fn remove_dir_all<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        todo!()
    }

    async fn remove_file<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        let fs = self.fs.write().await;
        fs.delete(path.as_ref()).map_err(wrap_report)
    }

    async fn rename<P: AsRef<Path> + Send>(&self, from: P, to: P) -> Result<()> {
        let fs = self.fs.write().await;
        let from = from.as_ref();
        fs.link(from, to.as_ref()).map_err(wrap_report)?;
        fs.unlink(from).map_err(wrap_report)?;
        Ok(())
    }

    async fn set_permissions<P: AsRef<Path> + Send>(
        &mut self,
        path: P,
        perm: Self::Permissions,
    ) -> Result<()> {
        let fs = self.fs.write().await;
        match fs.find_inode(path.as_ref()) {
            Ok(mut inode) => {
                // We only want to write the lower bits of perm.0 to inode.1.i_mode
                let mut mode = inode.mode();
                mode &= !0o777;
                mode |= perm.0 & 0o777;
                inode.1.i_mode = mode;
                fs.write_inode(&mut inode).map_err(wrap_report)
            }
            Err(err) => Err(wrap_report(err)),
        }
    }

    async fn symlink<P: AsRef<Path> + Send>(&self, src: P, dst: P) -> Result<()> {
        todo!()
    }

    async fn symlink_metadata<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::Metadata> {
        todo!()
    }

    async fn try_exists<P: AsRef<Path> + Send>(&self, path: P) -> Result<bool> {
        let fs = self.fs.read().await;
        fs.find_inode(path.as_ref())
            .map(|_| true)
            .map_err(wrap_report)
    }

    async fn write<P: AsRef<Path> + Send>(
        &self,
        path: P,
        contents: impl AsRef<[u8]> + Send,
    ) -> Result<()> {
        todo!()
    }

    fn new_dir_builder(&'a self) -> Self::DirBuilder {
        todo!()
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ExtFacadeMetadata {
    inode: DebugIgnore<ExtInode>,
}

#[async_trait::async_trait]
impl FloppyMetadata for ExtFacadeMetadata {
    type FileType = ExtFacadeFileType;

    type Permissions = ExtFacadePermissions;

    async fn file_type(&self) -> Self::FileType {
        ExtFacadeFileType { inode: self.inode }
    }

    async fn is_dir(&self) -> bool {
        self.inode.is_dir()
    }

    async fn is_file(&self) -> bool {
        self.inode.is_file()
    }

    async fn is_symlink(&self) -> bool {
        self.inode.is_symlink()
    }

    async fn len(&self) -> u64 {
        self.inode.size()
    }

    async fn permissions(&self) -> Self::Permissions {
        ExtFacadePermissions(self.inode.mode())
    }

    async fn modified(&self) -> Result<SystemTime> {
        self.inode.mtime().map_err(wrap_report)
    }

    async fn accessed(&self) -> Result<SystemTime> {
        self.inode.atime().map_err(wrap_report)
    }

    async fn created(&self) -> Result<SystemTime> {
        self.inode.ctime().map_err(wrap_report)
    }
}

#[derive(Debug)]
pub struct ExtFacadeReadDir {
    idx: usize,
    inodes: DebugIgnore<Vec<(ExtInode, libe2fs_sys::ext2_dir_entry)>>,
    path: PathBuf,
}

impl ExtFacadeReadDir {
    fn new(path: &Path, inodes: Vec<(ExtInode, libe2fs_sys::ext2_dir_entry)>) -> Self {
        Self {
            idx: 0,
            inodes: DebugIgnore(inodes),
            path: path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl FloppyReadDir for ExtFacadeReadDir {
    type DirEntry = ExtFacadeDirEntry;

    async fn next_entry(&mut self) -> Result<Option<Self::DirEntry>> {
        if self.idx < self.inodes.len() {
            let (inode, dir_entry) = self.inodes[0];
            self.idx += 1;
            Ok(Some(ExtFacadeDirEntry {
                inode: DebugIgnore(inode),
                entry: dir_entry,
                parent_path: self.path.clone(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ExtFacadePermissions(u16);

impl FloppyPermissions for ExtFacadePermissions {
    fn readonly(&self) -> bool {
        self.0 & 0o200 == 0
    }

    fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.0 &= !0o200;
        } else {
            self.0 |= 0o200;
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ExtFacadeDirBuilder();

#[async_trait::async_trait]
impl FloppyDirBuilder for ExtFacadeDirBuilder {
    fn recursive(&mut self, recursive: bool) -> &mut Self {
        todo!()
    }

    async fn create<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        todo!()
    }

    fn mode(&mut self, mode: u32) -> &mut Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct ExtFacadeDirEntry {
    inode: DebugIgnore<ExtInode>,
    entry: libe2fs_sys::ext2_dir_entry,
    parent_path: PathBuf,
}

#[async_trait::async_trait]
impl FloppyDirEntry for ExtFacadeDirEntry {
    type FileType = ExtFacadeFileType;
    type Metadata = ExtFacadeMetadata;

    fn file_name(&self) -> OsString {
        // SAFETY: We just got this struct (and therefore pointer) from e2fs.
        unsafe {
            OsString::from_vec(
                CString::from_raw(self.entry.name.as_ptr() as *mut i8)
                    .as_bytes()
                    .to_vec(),
            )
        }
    }

    async fn file_type(&self) -> Result<Self::FileType> {
        Ok(ExtFacadeFileType { inode: self.inode })
    }

    async fn metadata(&self) -> Result<ExtFacadeMetadata> {
        Ok(ExtFacadeMetadata { inode: self.inode })
    }

    fn path(&self) -> PathBuf {
        self.parent_path.clone()
    }

    #[cfg(unix)]
    fn ino(&self) -> u64 {
        self.inode.0 .0 as u64
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ExtFacadeFileType {
    inode: DebugIgnore<ExtInode>,
}

impl FloppyFileType for ExtFacadeFileType {
    fn is_dir(&self) -> bool {
        self.inode.is_dir()
    }

    fn is_file(&self) -> bool {
        self.inode.is_file()
    }

    fn is_symlink(&self) -> bool {
        self.inode.is_symlink()
    }
}

#[derive(Debug)]
pub struct ExtFacadeOpenOptions();

#[async_trait::async_trait]
impl FloppyOpenOptions for ExtFacadeOpenOptions {
    type File = ExtFacadeFile;

    fn new() -> Self {
        Self()
    }

    fn read(&mut self, read: bool) -> &mut Self {
        todo!()
    }

    fn write(&mut self, write: bool) -> &mut Self {
        todo!()
    }

    fn append(&mut self, append: bool) -> &mut Self {
        todo!()
    }

    fn truncate(&mut self, truncate: bool) -> &mut Self {
        todo!()
    }

    fn create(&mut self, create: bool) -> &mut Self {
        todo!()
    }

    fn create_new(&mut self, create_new: bool) -> &mut Self {
        todo!()
    }

    async fn open<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::File> {
        todo!()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ExtFacadeFile();

#[async_trait::async_trait]
impl FloppyFile for ExtFacadeFile {
    type Metadata = ExtFacadeMetadata;
    type Permissions = ExtFacadePermissions;

    async fn sync_all(&mut self) -> Result<()> {
        todo!()
    }

    async fn sync_data(&mut self) -> Result<()> {
        todo!()
    }

    async fn set_len(&mut self, size: u64) -> Result<()> {
        todo!()
    }

    async fn metadata(&self) -> Result<Self::Metadata> {
        todo!()
    }

    async fn try_clone(&self) -> Result<Box<Self>> {
        todo!()
    }

    async fn set_permissions(&self, perm: Self::Permissions) -> Result<()> {
        todo!()
    }

    async fn permissions(&self) -> Result<Self::Permissions> {
        todo!()
    }
}

impl AsyncRead for ExtFacadeFile {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        todo!()
    }
}

impl AsyncSeek for ExtFacadeFile {
    fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        todo!()
    }

    fn poll_complete(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        todo!()
    }
}

impl AsyncWrite for ExtFacadeFile {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }
}

fn wrap_report(report: eyre::Report) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, report)
}

fn wrap_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

// #[derive(Debug)]
// pub struct ExtFacadeTempDir {
//     path: PathBuf,
// }

// impl ExtFacadeTempDir {
//     async fn new() -> Result<Self> {
//         let mut path = std::env::temp_dir();
//         path.push(format!("peckish-workdir-{}", rand::random::<u64>()));
//         tokio::fs::create_dir_all(&path).await?;

//         Ok(Self { path })
//     }
// }

// impl FloppyTempDir for ExtFacadeTempDir {
//     fn path(&self) -> &Path {
//         &self.path
//     }
// }

// impl Drop for ExtFacadeTempDir {
//     fn drop(&mut self) {
//         if self.path.exists() {
//             std::fs::remove_dir_all(&self.path).unwrap();
//         }
//     }
// }

// impl AsRef<Path> for ExtFacadeTempDir {
//     fn as_ref(&self) -> &Path {
//         &self.path
//     }
// }

// impl AsRef<PathBuf> for ExtFacadeTempDir {
//     fn as_ref(&self) -> &PathBuf {
//         &self.path
//     }
// }

// impl std::ops::Deref for ExtFacadeTempDir {
//     type Target = Path;

//     fn deref(&self) -> &Self::Target {
//         &self.path
//     }
// }
