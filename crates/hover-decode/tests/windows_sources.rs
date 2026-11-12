#![cfg(all(windows, feature = "wic"))]
use hover_core::CancellationToken;
use hover_decode::local_io::{open_preview_file, read_local_snapshot};
use hover_decode::{DecodeError, DecodeLimits, wic::decode_local_file};
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str, bytes: &[u8]) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("hover-test-{}-{nonce}-{name}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join(name);
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only the uniquely created test directory is removed.
        let directory = self.0.parent().unwrap();
        assert!(
            directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("hover-test-")
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}

fn red_green_bmp() -> Vec<u8> {
    let mut bytes = vec![0u8; 62];
    bytes[..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&62u32.to_le_bytes());
    bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&2u32.to_le_bytes());
    bytes[22..26].copy_from_slice(&1u32.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes[54..62].copy_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
    bytes
}

#[test]
fn real_wic_pixels_are_correct_source_unchanged_and_file_is_unlocked() {
    let bytes = red_green_bmp();
    let fixture = Fixture::new("雪 café.bmp", &bytes);
    let image = decode_local_file(
        &fixture.0,
        100,
        100,
        DecodeLimits::default(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!((image.width, image.height, image.stride), (2, 1, 8));
    assert_eq!(image.pixels_bgra, [0, 0, 255, 255, 0, 255, 0, 255]);
    assert_eq!(std::fs::read(&fixture.0).unwrap(), bytes);
    let renamed = fixture.0.with_extension("renamed");
    std::fs::rename(&fixture.0, &renamed).unwrap();
    std::fs::rename(renamed, &fixture.0).unwrap();
}

#[test]
fn source_sharing_allows_another_process_to_rename_replace_and_delete() {
    let bytes = red_green_bmp();
    let fixture = Fixture::new("shared.bmp", &bytes);
    let _held_read = open_preview_file(&fixture.0).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "concurrent_file_probe", "--ignored"])
        .env("HOVER_TEST_PROBE_PATH", &fixture.0)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "Subprocess fixture helper; invoked by source_sharing test"]
fn concurrent_file_probe() {
    let path = PathBuf::from(std::env::var_os("HOVER_TEST_PROBE_PATH").expect("fixture path"));
    assert!(
        path.parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("hover-test-")
    );
    let renamed = path.with_extension("moved");
    std::fs::rename(&path, &renamed).unwrap();
    std::fs::write(&path, b"replacement from second process").unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(renamed).unwrap();
}

#[test]
fn bounded_snapshot_and_cancel_and_corrupt_input_fail_precisely() {
    let fixture = Fixture::new("bounded.bmp", &red_green_bmp());
    assert!(matches!(
        read_local_snapshot(&fixture.0, 10, &CancellationToken::new()),
        Err(DecodeError::FileTooLarge { .. })
    ));
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        decode_local_file(&fixture.0, 10, 10, DecodeLimits::default(), &token),
        Err(DecodeError::Cancelled)
    ));
    std::fs::write(&fixture.0, b"BMtruncated").unwrap();
    assert!(
        decode_local_file(
            &fixture.0,
            10,
            10,
            DecodeLimits::default(),
            &CancellationToken::new()
        )
        .is_err()
    );
}
