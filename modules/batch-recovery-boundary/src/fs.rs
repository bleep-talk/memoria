//! Directory-relative filesystem access for managed memory and private recovery data.
use crate::repo::RepoError;
use batch_recovery_decisions::{MemoryPath, Policy, SnapshotEntry};
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Component, Path};

fn io(error: rustix::io::Errno) -> RepoError {
    RepoError::Sys(error)
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn regular_flags() -> OFlags {
    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK
}

fn valid_repository_id(raw: &str) -> bool {
    raw.len() == 36
        && raw.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

/// Opens every component independently. A symlink cannot become an ancestor.
pub(crate) fn open_directory(path: &Path, create_last: bool) -> Result<OwnedFd, RepoError> {
    let requested = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(RepoError::Io)?.join(path)
    };
    // The caller supplies the repository anchor. Resolve its parent once, then
    // require the repository directory itself and all managed descendants not
    // to be symlinks. macOS temp paths commonly traverse the /var symlink.
    let parent = requested
        .parent()
        .ok_or_else(|| RepoError::UnsafePath(requested.display().to_string()))?;
    let leaf = requested
        .file_name()
        .ok_or_else(|| RepoError::UnsafePath(requested.display().to_string()))?;
    let absolute = parent.canonicalize().map_err(RepoError::Io)?.join(leaf);
    let mut fd = fs::open("/", dir_flags(), Mode::empty()).map_err(io)?;
    let components: Vec<_> = absolute
        .components()
        .filter_map(|component| match component {
            Component::RootDir | Component::CurDir => None,
            Component::Normal(name) => Some(Ok(name.to_os_string())),
            _ => Some(Err(RepoError::UnsafePath(absolute.display().to_string()))),
        })
        .collect::<Result<_, _>>()?;
    for (index, component) in components.iter().enumerate() {
        if create_last && index + 1 == components.len() {
            match fs::mkdirat(&fd, component, Mode::RUSR | Mode::WUSR | Mode::XUSR) {
                Ok(()) | Err(rustix::io::Errno::EXIST) => {}
                Err(error) => return Err(io(error)),
            }
        }
        fd = fs::openat(&fd, component, dir_flags(), Mode::empty()).map_err(io)?;
    }
    Ok(fd)
}

pub(crate) struct Confined {
    root: OwnedFd,
}

impl Confined {
    pub(crate) fn open(path: &Path) -> Result<Self, RepoError> {
        Ok(Self {
            root: open_directory(path, false)?,
        })
    }

    pub(crate) fn control(&self) -> Result<Self, RepoError> {
        let git = match fs::openat(&self.root, ".git", dir_flags(), Mode::empty()) {
            Ok(git) => git,
            Err(rustix::io::Errno::NOENT | rustix::io::Errno::NOTDIR) => {
                return Err(RepoError::UnsupportedRepository)
            }
            Err(error) => return Err(io(error)),
        };
        match fs::mkdirat(&git, "memoria", Mode::RUSR | Mode::WUSR | Mode::XUSR) {
            Ok(()) => fs::fsync(&git).map_err(io)?,
            Err(rustix::io::Errno::EXIST) => {}
            Err(error) => return Err(io(error)),
        }
        let root = fs::openat(&git, "memoria", dir_flags(), Mode::empty()).map_err(io)?;
        Ok(Self { root })
    }

    pub(crate) fn lock(&self) -> Result<File, RepoError> {
        let fd = fs::openat(
            &self.root,
            "lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(io)?;
        let stat = fs::fstat(&fd).map_err(io)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(RepoError::UnsafePath(".git/memoria/lock".into()));
        }
        let file = File::from(fd);
        fs::flock(&file, fs::FlockOperation::LockExclusive).map_err(io)?;
        Ok(file)
    }

    pub(crate) fn repository_id(&self) -> Result<String, RepoError> {
        let bytes = self
            .read("repository-id", 128)?
            .ok_or(RepoError::InvalidJournal)?;
        let raw = std::str::from_utf8(&bytes).map_err(|_| RepoError::InvalidJournal)?;
        if !valid_repository_id(raw) {
            return Err(RepoError::InvalidJournal);
        }
        Ok(raw.to_owned())
    }

    /// Called only after the repository lock is held.
    pub(crate) fn ensure_repository_id(&self) -> Result<String, RepoError> {
        if self.read("repository-id", 128)?.is_some() {
            return self.repository_id();
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.replace("repository-id", id.as_bytes())?;
        Ok(id)
    }

    fn directory(&self, relative: &str, create: bool) -> Result<OwnedFd, RepoError> {
        let mut fd = fs::openat(&self.root, ".", dir_flags(), Mode::empty()).map_err(io)?;
        if relative.is_empty() {
            return Ok(fd);
        }
        for component in relative.split('/') {
            if component.is_empty() || component == "." || component == ".." || component == ".git"
            {
                return Err(RepoError::UnsafePath(relative.to_owned()));
            }
            if create {
                match fs::mkdirat(&fd, component, Mode::RUSR | Mode::WUSR | Mode::XUSR) {
                    Ok(()) | Err(rustix::io::Errno::EXIST) => {}
                    Err(error) => return Err(io(error)),
                }
            }
            fd = fs::openat(&fd, component, dir_flags(), Mode::empty()).map_err(io)?;
        }
        Ok(fd)
    }

    fn parent(&self, relative: &str, create: bool) -> Result<(OwnedFd, String), RepoError> {
        let (parent, name) = relative.rsplit_once('/').unwrap_or(("", relative));
        if name.is_empty() || name == "." || name == ".." {
            return Err(RepoError::UnsafePath(relative.to_owned()));
        }
        Ok((self.directory(parent, create)?, name.to_owned()))
    }

    pub(crate) fn ensure_dir(&self, relative: &str) -> Result<(), RepoError> {
        self.directory(relative, true)?;
        Ok(())
    }

    pub(crate) fn read(&self, relative: &str, max: usize) -> Result<Option<Vec<u8>>, RepoError> {
        let (parent, name) = match self.parent(relative, false) {
            Ok(value) => value,
            Err(RepoError::Sys(rustix::io::Errno::NOENT)) => return Ok(None),
            Err(error) => return Err(error),
        };
        let fd = match fs::openat(&parent, name.as_str(), regular_flags(), Mode::empty()) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(io(error)),
        };
        let stat = fs::fstat(&fd).map_err(io)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(RepoError::UnsafePath(relative.to_owned()));
        }
        if stat.st_size < 0 || u64::try_from(stat.st_size).unwrap_or(u64::MAX) > max as u64 {
            return Err(RepoError::LimitExceeded);
        }
        let mut data = Vec::new();
        File::from(fd)
            .take((max as u64).saturating_add(1))
            .read_to_end(&mut data)
            .map_err(RepoError::Io)?;
        if data.len() > max {
            return Err(RepoError::LimitExceeded);
        }
        Ok(Some(data))
    }

    pub(crate) fn replace(&self, relative: &str, bytes: &[u8]) -> Result<(), RepoError> {
        let (parent, name) = self.parent(relative, true)?;
        if let Ok(stat) = fs::statat(&parent, name.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
            if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                return Err(RepoError::UnsafePath(relative.to_owned()));
            }
        }
        let temporary = format!(".memoria-{}", uuid::Uuid::new_v4());
        let fd = fs::openat(
            &parent,
            temporary.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(io)?;
        let mut file = File::from(fd);
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            let _ = fs::unlinkat(&parent, temporary.as_str(), AtFlags::empty());
            return Err(RepoError::Io(error));
        }
        drop(file);
        if let Err(error) = fs::renameat(&parent, temporary.as_str(), &parent, name.as_str()) {
            let _ = fs::unlinkat(&parent, temporary.as_str(), AtFlags::empty());
            return Err(io(error));
        }
        if relative.ends_with(".md") {
            crate::journal::crash("replace_after_rename");
        }
        fs::fsync(&parent).map_err(io)?;
        Ok(())
    }

    pub(crate) fn delete(&self, relative: &str) -> Result<(), RepoError> {
        let (parent, name) = self.parent(relative, false)?;
        let stat = fs::statat(&parent, name.as_str(), AtFlags::SYMLINK_NOFOLLOW).map_err(io)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(RepoError::UnsafePath(relative.to_owned()));
        }
        fs::unlinkat(&parent, name.as_str(), AtFlags::empty()).map_err(io)?;
        if relative.ends_with(".md") {
            crate::journal::crash("delete_after_unlink");
        }
        fs::fsync(&parent).map_err(io)?;
        Ok(())
    }

    pub(crate) fn list(&self, policy: &Policy) -> Result<Vec<SnapshotEntry>, RepoError> {
        let mut result = Vec::new();
        for root in policy.roots() {
            let directory = self.directory(root, false)?;
            self.walk(root, directory, policy, &mut result)?;
        }
        Ok(result)
    }

    fn walk(
        &self,
        prefix: &str,
        fd: OwnedFd,
        policy: &Policy,
        result: &mut Vec<SnapshotEntry>,
    ) -> Result<(), RepoError> {
        if result.len() >= policy.max_entries() {
            return Err(RepoError::LimitExceeded);
        }
        result.push(SnapshotEntry::Directory(MemoryPath::new(prefix)?));
        let iterator = Dir::read_from(&fd).map_err(io)?;
        for entry in iterator {
            let entry = entry.map_err(io)?;
            let name = entry
                .file_name()
                .to_str()
                .map_err(|_| RepoError::UnsafePath(prefix.to_owned()))?;
            if name == "." || name == ".." {
                continue;
            }
            let relative = format!("{prefix}/{name}");
            let path = MemoryPath::new(&relative)?;
            let stat = fs::statat(&fd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io)?;
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::Directory => {
                    let child = fs::openat(&fd, name, dir_flags(), Mode::empty()).map_err(io)?;
                    self.walk(&relative, child, policy, result)?;
                }
                FileType::RegularFile if path.is_markdown() => {
                    let bytes = self
                        .read(&relative, policy.max_file_bytes())?
                        .ok_or(RepoError::NotFound(relative))?;
                    let content = String::from_utf8(bytes)
                        .map_err(|_| RepoError::UnsafePath(path.to_string()))?;
                    result.push(SnapshotEntry::File(path, content));
                }
                FileType::RegularFile => {}
                _ => return Err(RepoError::UnsafePath(relative)),
            }
            if result.len() > policy.max_entries() {
                return Err(RepoError::LimitExceeded);
            }
        }
        Ok(())
    }
}

/// The effect executor entrypoint declared by RMS. All managed filesystem effects use `Confined`.
pub(crate) fn execute_confined(root: &Path) -> Result<Confined, RepoError> {
    Confined::open(root)
}
