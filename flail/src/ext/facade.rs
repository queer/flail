use std::ffi::{CString, OsString};
use std::future::Future;
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

use super::file::ExtFile;
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
    type DirBuilder = ExtFacadeDirBuilder<'a>;
    type DirEntry = ExtFacadeDirEntry;
    type File = ExtFacadeFile<'a>;
    type FileType = ExtFacadeFileType;
    type Metadata = ExtFacadeMetadata;
    type OpenOptions = ExtFacadeOpenOptions;
    type Permissions = ExtFacadePermissions;
    type ReadDir = ExtFacadeReadDir;

    async fn canonicalize<P: AsRef<Path> + Send>(&self, _path: P) -> Result<PathBuf> {
        unimplemented!(
            "canonicalize does not have any meaning as everything is already relative to root"
        )
    }

    async fn copy<P: AsRef<Path> + Send>(&self, from: P, to: P) -> Result<u64> {
        let from = from.as_ref();
        let to = to.as_ref();
        let (data, permissions) = {
            let fs = self.fs.read().await;
            let inode = fs.find_inode(from).map_err(wrap_report)?;
            let file = fs.open_file(inode.0, None).map_err(wrap_report)?;
            let mut buf = vec![0; inode.size() as usize];
            fs.read_file(&file, &mut buf).map_err(wrap_report)?;

            (buf, inode.mode() & 0o777)
        };

        self.write(to, &data).await?;
        {
            let fs = self.fs.write().await;
            let mut inode = fs.find_inode(to).map_err(wrap_report)?;
            inode.1.i_mode = (inode.1.i_mode & 0o70000) | permissions;
            fs.write_inode(&mut inode).map_err(wrap_report)?;
        }

        Ok(data.len() as u64)
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

    async fn hard_link<P: AsRef<Path> + Send>(&self, _src: P, _dst: P) -> Result<()> {
        unimplemented!("please open an issue if you need hard-link functionality.")
    }

    async fn metadata<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::Metadata> {
        let fs = self.fs.read().await;
        match fs.find_inode_follow(path.as_ref()) {
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

    async fn read_dir<P: AsRef<Path> + Send>(
        &self,
        path: P,
    ) -> Result<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::ReadDir> {
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
        let path = path.as_ref();
        let mut read_dir = self.read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let inode = entry.inode;
            let path = entry.path();
            if inode.is_dir() {
                self.remove_dir_all(path).await?;
            } else {
                self.remove_file(path).await?;
            }
        }

        self.remove_dir(path).await
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
        let fs = self.fs.write().await;
        let src = src.as_ref();
        let dst = dst.as_ref();
        let parent_inode = fs
            .find_inode(src.parent().unwrap_or(Path::new("/")))
            .map_err(wrap_report)?;

        fs.symlink(&parent_inode, None, src, dst)
            .map_err(wrap_report)
    }

    async fn symlink_metadata<P: AsRef<Path> + Send>(&self, path: P) -> Result<Self::Metadata> {
        let fs = self.fs.read().await;
        match fs.find_inode(path.as_ref()) {
            Ok(inode) => Ok(ExtFacadeMetadata {
                inode: DebugIgnore(inode),
            }),
            Err(err) => Err(wrap_report(err)),
        }
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
        let fs = self.fs.write().await;
        fs.write_to_file(path.as_ref(), contents.as_ref())
            .map(|_| ())
            .map_err(wrap_report)
    }

    fn new_dir_builder(&'a self) -> <ExtFacadeFloppyDisk as FloppyDisk<'a>>::DirBuilder {
        ExtFacadeDirBuilder {
            facade: self,
            recursive: false,
            mode: None,
        }
    }
}

#[async_trait::async_trait]
impl FloppyDiskUnixExt for ExtFacadeFloppyDisk {
    async fn chown<P: Into<PathBuf> + Send>(&self, path: P, uid: u32, gid: u32) -> Result<()> {
        let fs = self.fs.write().await;
        let mut inode = fs.find_inode(path.into()).map_err(wrap_report)?;
        inode.1.i_uid = uid as u16;
        inode.1.i_gid = gid as u16;
        fs.write_inode(&mut inode).map_err(wrap_report)
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ExtFacadeMetadata {
    inode: DebugIgnore<ExtInode>,
}

#[async_trait::async_trait]
impl<'a> FloppyMetadata<'a, ExtFacadeFloppyDisk> for ExtFacadeMetadata {
    async fn file_type(&self) -> <ExtFacadeFloppyDisk as FloppyDisk<'a>>::FileType {
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

    async fn permissions(&self) -> <ExtFacadeFloppyDisk as FloppyDisk<'a>>::Permissions {
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

impl FloppyUnixMetadata for ExtFacadeMetadata {
    fn uid(&self) -> Result<u32> {
        Ok(self.inode.0 .1.i_uid as u32)
    }

    fn gid(&self) -> Result<u32> {
        Ok(self.inode.0 .1.i_gid as u32)
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
impl<'a> FloppyReadDir<'a, ExtFacadeFloppyDisk> for ExtFacadeReadDir {
    async fn next_entry(
        &mut self,
    ) -> Result<Option<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::DirEntry>> {
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

impl FloppyUnixPermissions for ExtFacadePermissions {
    fn mode(&self) -> u32 {
        self.0 as u32
    }

    fn set_mode(&mut self, mode: u32) {
        self.0 = mode as u16;
    }

    fn from_mode(mode: u32) -> Self {
        Self(mode as u16)
    }
}

#[derive(Debug)]
pub struct ExtFacadeDirBuilder<'a> {
    facade: &'a ExtFacadeFloppyDisk,
    recursive: bool,
    mode: Option<u32>,
}

#[async_trait::async_trait]
impl FloppyDirBuilder for ExtFacadeDirBuilder<'_> {
    fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.recursive = recursive;
        self
    }

    async fn create<P: AsRef<Path> + Send>(&self, path: P) -> Result<()> {
        let fs = self.facade.fs.read().await;
        let path = path.as_ref();
        fs.mkdir(
            path.parent().unwrap_or(&PathBuf::from("/")),
            path.file_name()
                .expect("paths must have file names")
                .to_string_lossy()
                .to_string(),
        )
        .map_err(wrap_report)?;

        if let Some(mode) = self.mode {
            let mut inode = fs.find_inode(path).unwrap();
            inode.1.i_mode |= mode as u16;
            fs.write_inode(&mut inode).map_err(wrap_report)?;
        }

        Ok(())
    }

    fn mode(&mut self, mode: u32) -> &mut Self {
        self.mode = if mode == 0 { None } else { Some(mode) };
        self
    }
}

#[derive(Debug)]
pub struct ExtFacadeDirEntry {
    inode: DebugIgnore<ExtInode>,
    entry: libe2fs_sys::ext2_dir_entry,
    parent_path: PathBuf,
}

#[async_trait::async_trait]
impl<'a> FloppyDirEntry<'a, ExtFacadeFloppyDisk> for ExtFacadeDirEntry {
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

    async fn file_type(&self) -> Result<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::FileType> {
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
pub struct ExtFacadeOpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

#[async_trait::async_trait]
impl<'a> FloppyOpenOptions<'a, ExtFacadeFloppyDisk> for ExtFacadeOpenOptions {
    fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    async fn open<P: AsRef<Path> + Send>(
        &self,
        facade: &'a mut ExtFacadeFloppyDisk,
        _path: P,
    ) -> Result<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::File> {
        let path = _path.as_ref();
        // TODO: FIXME: THIS DOESN'T HANDLE FLAGS RIGHT AAAAAAAAAAAAAAAAAAAAAAAAA
        let fs = facade.fs.write().await;
        let file = match fs.find_inode(path) {
            Ok(inode) => {
                let file = fs.open_file(inode.0, None).map_err(wrap_report)?;
                ExtFacadeFile {
                    facade,
                    file,
                    seek_position: std::io::SeekFrom::Start(0),
                }
            }
            Err(err) => {
                if self.create {
                    let file = fs.touch(path).map_err(wrap_report)?;
                    ExtFacadeFile {
                        facade,
                        file,
                        seek_position: std::io::SeekFrom::Start(0),
                    }
                } else {
                    return Err(wrap_report(err));
                }
            }
        };

        Ok(file)
    }
}

#[derive(Debug)]
pub struct ExtFacadeFile<'a> {
    facade: &'a ExtFacadeFloppyDisk,
    file: ExtFile,
    seek_position: std::io::SeekFrom,
}
unsafe impl Send for ExtFacadeFile<'_> {}
unsafe impl Sync for ExtFacadeFile<'_> {}

#[async_trait::async_trait]
impl<'a> FloppyFile<'a, ExtFacadeFloppyDisk> for ExtFacadeFile<'a> {
    async fn sync_all(&mut self) -> Result<()> {
        Ok(())
    }

    async fn sync_data(&mut self) -> Result<()> {
        Ok(())
    }

    async fn set_len(&mut self, size: u64) -> Result<()> {
        let fs = self.facade.fs.write().await;
        let mut inode = fs.get_inode(&self.file).map_err(wrap_report)?;
        // TODO: Support 64-bit inodes properly!
        inode.1.i_size = size as u32;
        fs.write_inode(&mut inode).map_err(wrap_report)?;
        Ok(())
    }

    async fn metadata(&self) -> Result<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::Metadata> {
        let fs = self.facade.fs.read().await;
        let inode = fs.get_inode(&self.file).map_err(wrap_report)?;
        Ok(ExtFacadeMetadata {
            inode: DebugIgnore(inode),
        })
    }

    async fn try_clone(&'a self) -> Result<Box<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::File>> {
        unimplemented!("try_clone requires smarter pointer management for ExtFile to implement.")
    }

    async fn set_permissions(
        &self,
        perm: <ExtFacadeFloppyDisk as FloppyDisk<'a>>::Permissions,
    ) -> Result<()> {
        let fs = self.facade.fs.write().await;
        let mut inode = fs.get_inode(&self.file).map_err(wrap_report)?;
        inode.1.i_mode = (inode.1.i_mode & 0o70000) | perm.0;
        fs.write_inode(&mut inode).map_err(wrap_report)?;
        Ok(())
    }

    async fn permissions(&self) -> Result<<ExtFacadeFloppyDisk as FloppyDisk<'a>>::Permissions> {
        let fs = self.facade.fs.read().await;
        let inode = fs.get_inode(&self.file).map_err(wrap_report)?;
        Ok(ExtFacadePermissions(inode.1.i_mode))
    }
}

impl AsyncRead for ExtFacadeFile<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        // TODO: Respect seek position
        let out_buf = run_here(async {
            let fs = self.facade.fs.read().await;
            let mut buf = vec![];
            fs.read_file(&self.file, &mut buf)
                .map_err(wrap_report)
                .unwrap();
            buf
        });
        // copy out_buf to buf
        let len = buf.remaining().min(out_buf.len());
        buf.put_slice(&out_buf[..len]);
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for ExtFacadeFile<'_> {
    fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        let mut this = self.get_mut();
        this.seek_position = position;
        Ok(())
    }

    fn poll_complete(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        let position = match self.seek_position {
            std::io::SeekFrom::Start(pos) => pos as i64,
            std::io::SeekFrom::End(pos) => run_here(async {
                let fs = self.facade.fs.read().await;
                let inode = fs.get_inode(&self.file).unwrap();
                inode.1.i_size as i64 + pos
            }),
            std::io::SeekFrom::Current(pos) => run_here(async {
                let fs = self.facade.fs.read().await;
                let inode = fs.get_inode(&self.file).unwrap();
                inode.1.i_size as i64 + pos
            }),
        };

        Poll::Ready(Ok(position as u64))
    }
}

impl AsyncWrite for ExtFacadeFile<'_> {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let res = run_here(async {
            let fs = self.facade.fs.write().await;
            fs.write_file(&self.file, buf).map_err(wrap_report)
        });
        Poll::Ready(res)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn wrap_report(report: eyre::Report) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, report)
}

fn wrap_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

fn run_here<F: Future>(fut: F) -> F::Output {
    // TODO: This is evil
    // Adapted from https://stackoverflow.com/questions/66035290
    let handle = tokio::runtime::Handle::try_current().unwrap();
    let _guard = handle.enter();
    futures::executor::block_on(fut)
}

#[allow(unused)]
fn run_here_outside_of_tokio_context<F: Future>(fut: F) -> F::Output {
    // TODO: This is slightly less-evil than the previous one but still pretty bad
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(fut)
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

//     fn deref(&self) -> &<ExtFacadeFloppyDisk as FloppyDisk<'a>>::Target {
//         &self.path
//     }
// }

#[cfg(test)]
mod tests {}
