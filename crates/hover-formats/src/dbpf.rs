//! DBPF 2.1 index metadata, not decoded resources or CC reconstruction.
//! Format comparison: LlamaLogic 4a3d576f0afb776aba783db561760afb5b5de8b7,
//! DataBasePackedFile.ParseHeader/ParseIndex. No game code is executed.
use hover_core::CancellationToken;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_index_bytes: u64,
    pub max_preview_entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_index_bytes: 4 * 1024 * 1024,
            max_preview_entries: 5_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub type_id: u32,
    pub group_id: u32,
    pub instance_id: u64,
    pub position: u32,
    pub size: u32,
    pub decompressed_size: u32,
    pub compression: Option<u16>,
    pub committed: Option<u16>,
}

#[derive(Debug)]
pub struct Index {
    pub declared_count: u32,
    pub entries: Vec<Entry>,
    pub truncated: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot read DBPF: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not a DBPF package.")]
    Signature,
    #[error("Unsupported DBPF version {major}.{minor}; this reader supports 2.1 metadata only.")]
    Version { major: u32, minor: u32 },
    #[error("Invalid DBPF index: {0}")]
    Invalid(&'static str),
    #[error("DBPF index exceeds the automatic preview byte budget.")]
    Budget,
    #[error("Preview was cancelled.")]
    Cancelled,
}

pub fn read_index(
    reader: &mut (impl Read + Seek),
    limits: Limits,
    token: &CancellationToken,
) -> Result<Index, Error> {
    cancelled(token)?;
    reader.seek(SeekFrom::Start(0))?;
    let mut header = [0; 96];
    reader.read_exact(&mut header)?;
    if &header[..4] != b"DBPF" {
        return Err(Error::Signature);
    }
    let major = u32_at(&header, 4);
    let minor = u32_at(&header, 8);
    if (major, minor) != (2, 1) {
        return Err(Error::Version { major, minor });
    }
    let count = u32_at(&header, 36);
    let index_size = u32_at(&header, 44) as u64;
    let wide_offset = u64::from_le_bytes(header[64..72].try_into().unwrap());
    let index_offset = if wide_offset != 0 {
        wide_offset
    } else {
        u32_at(&header, 40) as u64
    };
    let file_size = reader.seek(SeekFrom::End(0))?;
    if count == 0 && index_size == 0 {
        return Ok(Index {
            declared_count: 0,
            entries: Vec::new(),
            truncated: false,
        });
    }
    if index_size > limits.max_index_bytes {
        return Err(Error::Budget);
    }
    let index_end = index_offset
        .checked_add(index_size)
        .filter(|&end| end <= file_size)
        .ok_or(Error::Invalid("index is outside the file"))?;
    if index_offset < 96 || index_size < 4 {
        return Err(Error::Invalid("index overlaps the header or lacks flags"));
    }
    // Even with all three constant keys, each entry needs at least 16 bytes.
    if count as u64 > (index_size - 4) / 16 {
        return Err(Error::Invalid("entry count cannot fit in index"));
    }
    reader.seek(SeekFrom::Start(index_offset))?;
    let size = usize::try_from(index_size).map_err(|_| Error::Budget)?;
    let mut bytes = vec![0; size];
    for chunk in bytes.chunks_mut(64 * 1024) {
        cancelled(token)?;
        reader.read_exact(chunk)?;
    }
    let mut view = View {
        bytes: &bytes,
        position: 0,
    };
    let flags = view.u32()?;
    if flags & !7 != 0 {
        return Err(Error::Invalid("unknown index flag bits"));
    }
    let constant_type = if flags & 1 != 0 {
        Some(view.u32()?)
    } else {
        None
    };
    let constant_group = if flags & 2 != 0 {
        Some(view.u32()?)
    } else {
        None
    };
    let constant_high = if flags & 4 != 0 {
        Some(view.u32()?)
    } else {
        None
    };
    let preview_count = (count as usize).min(limits.max_preview_entries);
    let mut entries = Vec::with_capacity(preview_count);
    for _ in 0..preview_count {
        cancelled(token)?;
        let type_id = match constant_type {
            Some(v) => v,
            None => view.u32()?,
        };
        let group_id = match constant_group {
            Some(v) => v,
            None => view.u32()?,
        };
        let high = match constant_high {
            Some(v) => v,
            None => view.u32()?,
        };
        let low = view.u32()?;
        let position = view.u32()?;
        let size_flags = view.u32()?;
        let size = size_flags & 0x7fff_ffff;
        let decompressed_size = view.u32()?;
        let (compression, committed) = if size_flags & 0x8000_0000 != 0 {
            (Some(view.u16()?), Some(view.u16()?))
        } else {
            (None, None)
        };
        if size > 0 {
            let end = u64::from(position)
                .checked_add(size as u64)
                .filter(|&end| end <= file_size)
                .ok_or(Error::Invalid("resource is outside the file"))?;
            if position < 96 || ((position as u64) < index_end && end > index_offset) {
                return Err(Error::Invalid("resource overlaps header/index"));
            }
        }
        entries.push(Entry {
            type_id,
            group_id,
            instance_id: ((high as u64) << 32) | low as u64,
            position,
            size,
            decompressed_size,
            compression,
            committed,
        });
    }
    Ok(Index {
        declared_count: count,
        entries,
        truncated: preview_count < count as usize,
    })
}

fn cancelled(token: &CancellationToken) -> Result<(), Error> {
    if token.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
struct View<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl View<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(Error::Invalid("offset overflow"))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(Error::Invalid("truncated entry"))?;
        self.position = end;
        Ok(value.try_into().unwrap())
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn fixture(flags: u32, extended: bool) -> Vec<u8> {
        let mut bytes = vec![0; 100]; // 96-byte header + four resource bytes.
        bytes[..4].copy_from_slice(b"DBPF");
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&2u32.to_le_bytes());
        bytes[64..72].copy_from_slice(&100u64.to_le_bytes());
        // A stale legacy pointer must not override the nonzero 64-bit pointer.
        bytes[40..44].copy_from_slice(&999_999u32.to_le_bytes());
        let mut index = flags.to_le_bytes().to_vec();
        for (bit, value) in [(1, 0x034a_eecbu32), (2, 1), (4, 2)] {
            if flags & bit != 0 {
                index.extend(value.to_le_bytes());
            }
        }
        for low in [3u32, 4] {
            for (bit, value) in [(1, 0x034a_eecbu32), (2, 1), (4, 2)] {
                if flags & bit == 0 {
                    index.extend(value.to_le_bytes());
                }
            }
            for value in [low, 96, 4 | if extended { 0x8000_0000 } else { 0 }, 4] {
                index.extend(value.to_le_bytes());
            }
            if extended {
                index.extend(0x5a42u16.to_le_bytes());
                index.extend(1u16.to_le_bytes());
            }
        }
        bytes[44..48].copy_from_slice(&(index.len() as u32).to_le_bytes());
        bytes.extend(index);
        bytes
    }
    #[test]
    fn all_constant_key_layouts_and_optional_compression_fields_stay_aligned() {
        for flags in 0..=7 {
            for extended in [false, true] {
                let index = read_index(
                    &mut Cursor::new(fixture(flags, extended)),
                    Limits::default(),
                    &CancellationToken::new(),
                )
                .unwrap();
                assert_eq!(index.declared_count, 2);
                assert!(!index.truncated);
                assert_eq!(index.entries[1].instance_id, (2u64 << 32) | 4);
                assert_eq!(index.entries[1].type_id, 0x034a_eecb);
                assert_eq!(index.entries[1].committed, extended.then_some(1));
            }
        }
    }
    #[test]
    fn rejects_bad_versions_offsets_truncation_flags_and_budgets() {
        let good = fixture(0, false);
        for length in 0..good.len() {
            assert!(
                read_index(
                    &mut Cursor::new(&good[..length]),
                    Limits::default(),
                    &CancellationToken::new()
                )
                .is_err()
            );
        }
        for (offset, bytes) in [
            (4, 3u32.to_le_bytes()),
            (36, u32::MAX.to_le_bytes()),
            (100, 8u32.to_le_bytes()),
        ] {
            let mut bad = good.clone();
            bad[offset..offset + 4].copy_from_slice(&bytes);
            assert!(
                read_index(
                    &mut Cursor::new(bad),
                    Limits::default(),
                    &CancellationToken::new()
                )
                .is_err()
            );
        }
        assert!(matches!(
            read_index(
                &mut Cursor::new(good),
                Limits {
                    max_index_bytes: 1,
                    ..Limits::default()
                },
                &CancellationToken::new()
            ),
            Err(Error::Budget)
        ));
    }
    #[test]
    fn reports_partial_prefix_and_cancels_without_claiming_full_validation() {
        let bytes = fixture(7, false);
        let index = read_index(
            &mut Cursor::new(&bytes),
            Limits {
                max_preview_entries: 1,
                ..Limits::default()
            },
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(index.entries.len(), 1);
        assert!(index.truncated);
        let token = CancellationToken::new();
        token.cancel();
        assert!(matches!(
            read_index(&mut Cursor::new(bytes), Limits::default(), &token),
            Err(Error::Cancelled)
        ));
    }

    #[test]
    fn deterministic_malformed_corpus_never_panics_or_exceeds_the_entry_cap() {
        let original = fixture(3, true);
        let mut seed = 0x48_4f_56_45_52u64;
        for _ in 0..4_096 {
            let mut bytes = original.clone();
            for _ in 0..8 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let index = seed as usize % bytes.len();
                bytes[index] ^= (seed >> 32) as u8;
            }
            let limits = Limits {
                max_preview_entries: 1,
                ..Limits::default()
            };
            if let Ok(index) =
                read_index(&mut Cursor::new(bytes), limits, &CancellationToken::new())
            {
                assert!(index.entries.len() <= 1);
            }
        }
    }
}
