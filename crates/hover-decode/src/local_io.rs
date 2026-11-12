//! Shared read-only source policy for local previews. No network or cloud recall.
use crate::{CancellationToken, DecodeError};
use std::fs::{File, Metadata, OpenOptions};
use std::io::Read;
use std::path::Path;

pub fn local_metadata(path: &Path) -> std::io::Result<Metadata> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use std::path::{Component, Prefix};
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        use windows::core::PCWSTR;
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => {
                    return Err(unavailable(
                        "Network and device paths require explicit access; hover will not open them.",
                    ));
                }
            },
            _ => return Err(unavailable("Hover requires an absolute local file path.")),
        };
        let root = [drive as u16, b':' as u16, b'\\' as u16, 0];
        if matches!(unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) }, 0 | 1 | 4) {
            return Err(unavailable(
                "This drive is unavailable or remote; hover will not access it.",
            ));
        }
        // Also reject linked parents so an apparently local path cannot redirect
        // the automatic preview into a network share or cloud provider.
        for ancestor in path.ancestors() {
            let metadata = std::fs::symlink_metadata(ancestor)?;
            if blocked_attributes(metadata.file_attributes()) {
                return Err(unavailable(
                    "This item is online-only or linked. Hover will not download it or follow the link automatically.",
                ));
            }
        }
    }
    std::fs::symlink_metadata(path)
}

pub fn open_preview_file(path: &Path) -> std::io::Result<File> {
    local_metadata(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ | WRITE | DELETE; OPEN_NO_RECALL | OPEN_REPARSE_POINT.
        options
            .share_mode(7)
            .custom_flags(0x0010_0000 | 0x0020_0000);
    }
    let file = options.open(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if blocked_attributes(file.metadata()?.file_attributes()) {
            return Err(unavailable(
                "This item changed into an online-only or linked file.",
            ));
        }
    }
    Ok(file)
}

/// Closes the source handle before a native codec receives the bounded bytes.
pub fn read_local_snapshot(
    path: &Path,
    max_bytes: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, DecodeError> {
    let mut file =
        open_preview_file(path).map_err(|error| DecodeError::Source(error.to_string()))?;
    let before = file.metadata().map_err(|_| DecodeError::NotReadable)?;
    if before.len() > max_bytes {
        return Err(DecodeError::FileTooLarge {
            actual: before.len(),
            limit: max_bytes,
        });
    }
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let read = file
            .read(&mut buffer)
            .map_err(|_| DecodeError::NotReadable)?;
        if read == 0 {
            break;
        }
        let next = (bytes.len() as u64).saturating_add(read as u64);
        if next > max_bytes {
            return Err(DecodeError::FileTooLarge {
                actual: next,
                limit: max_bytes,
            });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let after = file.metadata().map_err(|_| DecodeError::NotReadable)?;
    let current = local_metadata(path).map_err(|_| DecodeError::SourceChanged)?;
    if before.len() != bytes.len() as u64
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || after.len() != current.len()
        || after.modified().ok() != current.modified().ok()
    {
        return Err(DecodeError::SourceChanged);
    }
    Ok(bytes)
}

fn unavailable(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, message)
}

#[cfg(any(windows, test))]
fn blocked_attributes(attributes: u32) -> bool {
    // REPARSE_POINT | OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS.
    attributes & (0x400 | 0x1000 | 0x40000 | 0x400000) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_cloud_recall_and_links_but_allows_normal_local_files() {
        for flag in [0x400, 0x1000, 0x40000, 0x400000] {
            assert!(blocked_attributes(flag | 0x20));
        }
        assert!(!blocked_attributes(0x20));
        assert!(!blocked_attributes(0x10));
    }
}
