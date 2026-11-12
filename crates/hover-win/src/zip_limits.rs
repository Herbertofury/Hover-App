//! ZIP directory allocation guard, based on PKWARE APPNOTE 6.3.10.
//! Checks budgets before zip builds its central-directory index.
use hover_core::CancellationToken;
use std::io::{Read, Seek, SeekFrom};

pub fn preflight(reader: &mut (impl Read + Seek), token: &CancellationToken) -> Result<(), String> {
    if token.is_cancelled() {
        return Err("Preview was cancelled.".into());
    }
    let len = reader.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
    let tail_len = len.min(65_557) as usize;
    let tail_start = len - tail_len as u64;
    reader
        .seek(SeekFrom::Start(tail_start))
        .map_err(|e| e.to_string())?;
    let mut tail = vec![0; tail_len];
    reader.read_exact(&mut tail).map_err(|e| e.to_string())?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            tail[i..].starts_with(b"PK\x05\x06")
                && i + 22 + u16_at(&tail, i + 20) as usize == tail.len()
        })
        .ok_or("ZIP end record is missing or truncated.")?;
    let eocd_offset = tail_start + end as u64;
    if u16_at(&tail, end + 4) != 0 || u16_at(&tail, end + 6) != 0 {
        return Err("Multi-disk ZIP previews are not supported.".into());
    }
    let mut entries = u16_at(&tail, end + 10) as u64;
    let mut directory_bytes = u32_at(&tail, end + 12) as u64;
    let mut directory_offset = u32_at(&tail, end + 16) as u64;
    let mut directory_end = eocd_offset;
    if entries == 0xffff || directory_bytes == 0xffff_ffff || directory_offset == 0xffff_ffff {
        let locator_offset = eocd_offset
            .checked_sub(20)
            .ok_or("Truncated ZIP64 locator.")?;
        let mut locator = [0; 20];
        reader
            .seek(SeekFrom::Start(locator_offset))
            .map_err(|e| e.to_string())?;
        reader.read_exact(&mut locator).map_err(|e| e.to_string())?;
        if &locator[..4] != b"PK\x06\x07" || u32_at(&locator, 4) != 0 || u32_at(&locator, 16) != 1 {
            return Err("Invalid or multi-disk ZIP64 locator.".into());
        }
        let record_offset = u64_at(&locator, 8);
        let mut record = [0; 56];
        if record_offset
            .checked_add(56)
            .is_none_or(|end| end > locator_offset)
        {
            return Err("ZIP64 end record is outside the archive.".into());
        }
        reader
            .seek(SeekFrom::Start(record_offset))
            .map_err(|e| e.to_string())?;
        reader.read_exact(&mut record).map_err(|e| e.to_string())?;
        if &record[..4] != b"PK\x06\x06"
            || u64_at(&record, 4) < 44
            || record_offset
                .checked_add(12)
                .and_then(|v| v.checked_add(u64_at(&record, 4)))
                != Some(locator_offset)
            || u32_at(&record, 16) != 0
            || u32_at(&record, 20) != 0
            || u64_at(&record, 24) != u64_at(&record, 32)
        {
            return Err("Invalid or multi-disk ZIP64 directory.".into());
        }
        entries = u64_at(&record, 32);
        directory_bytes = u64_at(&record, 40);
        directory_offset = u64_at(&record, 48);
        directory_end = record_offset;
    } else if u16_at(&tail, end + 8) as u64 != entries {
        return Err("Inconsistent ZIP entry counts.".into());
    }
    if entries > 20_000 || directory_bytes > 8 * 1024 * 1024 {
        return Err("ZIP directory exceeds the automatic preview budget (20,000 entries / 8 MiB). The archive was not extracted.".into());
    }
    if directory_offset.checked_add(directory_bytes) != Some(directory_end)
        || entries > directory_bytes / 46
    {
        return Err("ZIP directory offsets/counts are inconsistent; prefixed archives require a separate reader.".into());
    }
    if token.is_cancelled() {
        return Err("Preview was cancelled.".into());
    }
    reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    Ok(())
}

fn u16_at(bytes: &[u8], i: usize) -> u16 {
    u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    #[test]
    fn permits_real_zip_listing_and_rejects_corrupt_or_hostile_directory() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("hello.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"Hover fixture").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let token = CancellationToken::new();
        preflight(&mut Cursor::new(&bytes), &token).unwrap();
        let mut bad = bytes.clone();
        let end = bad.len() - 22;
        bad[end + 12..end + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(preflight(&mut Cursor::new(bad), &token).is_err());
        for len in 0..22 {
            assert!(preflight(&mut Cursor::new(&bytes[..len]), &token).is_err());
        }
        token.cancel();
        assert!(preflight(&mut Cursor::new(bytes), &token).is_err());
    }
}
