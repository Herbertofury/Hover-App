//! Prints the reproducible synthetic two-frame test fixture as hexadecimal.
use std::borrow::Cow;

fn main() {
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 2, 1, &[255, 0, 0, 0, 255, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Infinite).unwrap();
        for (pixels, delay) in [(vec![0, 1], 12), (vec![1, 0], 6)] {
            let frame = gif::Frame {
                width: 2,
                height: 1,
                delay,
                dispose: gif::DisposalMethod::Keep,
                buffer: Cow::Owned(pixels),
                ..Default::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    println!(
        "{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
}
