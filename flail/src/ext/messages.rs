#[derive(thiserror::Error, Debug)]
pub enum ExtEtMessage {
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
