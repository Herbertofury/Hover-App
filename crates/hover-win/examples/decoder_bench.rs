//! Component comparison on a caller-authorized fixture, not Explorer latency.
#[cfg(all(windows, not(test)))]
#[path = "../src/decoder_process.rs"]
mod decoder_process;

#[cfg(all(windows, not(test)))]
fn main() {
    use hover_core::CancellationToken;
    use hover_decode::{DecodeLimits, DecodedFrame};
    use serde_json::json;
    use std::time::Instant;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--decode-worker"] {
        std::process::exit(decoder_process::worker_entry());
    }
    if args.len() != 2 {
        eprintln!("Usage: decoder_bench <authorized local fixture> <1..64 repeats>");
        std::process::exit(2);
    }
    let path = std::path::Path::new(&args[0]);
    let repeats = args[1]
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=64).contains(n))
        .expect("Repeat count must be 1..64");
    let animated = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"));
    let (width, height) = if animated { (1600, 1200) } else { (8192, 8192) };
    let mut samples = Vec::new();
    let mut expected: Option<Vec<DecodedFrame>> = None;
    for mode in ["in_process_reference", "isolated_process"] {
        for iteration in 0..repeats {
            let token = CancellationToken::new();
            let start = Instant::now();
            let frames = if mode == "isolated_process" {
                decoder_process::decode_file(path, animated, width, height, &token).unwrap()
            } else if animated {
                hover_decode::decode_gif_animation(
                    path,
                    width,
                    height,
                    DecodeLimits::default(),
                    &token,
                )
                .unwrap()
                .frames
            } else {
                let image = hover_decode::wic::decode_local_file(
                    path,
                    width,
                    height,
                    DecodeLimits::default(),
                    &token,
                )
                .unwrap();
                vec![DecodedFrame {
                    width: image.width,
                    height: image.height,
                    stride: image.stride,
                    pixels_bgra: image.pixels_bgra,
                    delay_ms: 0,
                }]
            };
            let elapsed = start.elapsed();
            let mut digest = 0xcbf29ce484222325u64;
            for frame in &frames {
                for byte in frame
                    .width
                    .to_le_bytes()
                    .into_iter()
                    .chain(frame.height.to_le_bytes())
                    .chain(frame.delay_ms.to_le_bytes())
                    .chain(frame.pixels_bgra.iter().copied())
                {
                    digest = (digest ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                }
            }
            if let Some(expected) = &expected {
                assert_eq!(frames.len(), expected.len(), "Frame count differs");
                for (actual, reference) in frames.iter().zip(expected) {
                    assert_eq!(
                        (actual.width, actual.height, actual.stride, actual.delay_ms),
                        (
                            reference.width,
                            reference.height,
                            reference.stride,
                            reference.delay_ms
                        ),
                        "Decoded geometry/timing differs"
                    );
                    assert_eq!(
                        actual.pixels_bgra, reference.pixels_bgra,
                        "Decoded pixels differ"
                    );
                }
            } else {
                expected = Some(frames.clone());
            }
            samples.push(json!({"mode":mode,"iteration":iteration,"wall_ms":elapsed.as_secs_f64()*1000.0,"frames":frames.len(),"digest_fnv1a64":format!("{digest:016x}"),"worker":if mode=="isolated_process"{decoder_process::diagnostics()}else{serde_json::Value::Null}}));
        }
    }
    println!(
        "{}",
        json!({"schema":1,"scope":"source-read/decode/process exchange component; no UI or Explorer timing","cache":"OS cache uncontrolled; fixtures may already be cached","order":"reference samples then isolated samples; same process","digest_is_cryptographic":false,"comparison":"exact geometry, stride, delay and pixel byte comparison after stopping each timer","all_outputs_equal":true,"samples":samples})
    );
}

#[cfg(any(not(windows), test))]
fn main() {}
