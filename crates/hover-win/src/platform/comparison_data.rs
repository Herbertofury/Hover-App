//! Read-only comparison jobs. One replacement worker decodes panes in order,
//! publishes useful panes progressively, then computes bounded differences.
use super::*;
use crossbeam_channel::Sender;
use hover_core::comparison::{LinePair, PixelDifference, compare_lines, compare_pixels};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub(super) const MAX_PANES: usize = 4;
pub(super) const MAX_PANE_BYTES: usize = 64 * 1024 * 1024;
pub(super) const MAX_RESIDENT_BYTES: usize = 128 * 1024 * 1024;
pub(super) const MAX_BYTE_SCAN: u64 = 128 * 1024 * 1024;

pub(super) struct Job {
    pub generation: u64,
    pub paths: Vec<PathBuf>,
    pub settings: PreviewTypeSettings,
}
pub(super) enum Event {
    Pane {
        generation: u64,
        index: usize,
        result: Result<Arc<PaneData>, String>,
    },
    Pair {
        generation: u64,
        index: usize,
        result: Arc<PairData>,
    },
}
pub(super) struct PaneData {
    pub content: Content,
    pub bytes: u64,
    pub modified: Option<u128>,
    pub is_dir: bool,
    pub note: String,
    pub resident_bytes: usize,
    pub has_alpha: Vec<bool>,
    pub stage: &'static str,
}
pub(super) enum Content {
    Images {
        frames: Vec<OverlayFrame>,
    },
    Rows {
        subtitle: String,
        lines: Vec<String>,
        truncated: bool,
    },
}
pub(super) struct PairData {
    pub byte_scan: ByteScan,
    pub lines: Option<Vec<LinePair>>,
    pub pixel_stats: Vec<PixelDifference>,
    pub masks: Vec<OverlayFrame>,
    pub note: String,
    pub timing_equal: Option<bool>,
}
#[derive(Clone)]
pub(super) struct ByteScan {
    pub state: &'static str,
    pub examined: u64,
    pub changed: Option<u64>,
    pub note: String,
}
fn unavailable(note: impl Into<String>) -> ByteScan {
    ByteScan {
        state: "not_compared",
        examined: 0,
        changed: None,
        note: note.into(),
    }
}

pub(super) fn run(job: Job, token: &CancellationToken, events: &Sender<Event>, notify: &impl Fn()) {
    let mut data = Vec::with_capacity(job.paths.len());
    let mut resident = 0usize;
    for (index, path) in job.paths.iter().enumerate() {
        if token.is_cancelled() {
            return;
        }
        let result=load(path,token,&job.settings).and_then(|pane|{
            if pane.resident_bytes>MAX_PANE_BYTES || resident.saturating_add(pane.resident_bytes)>MAX_RESIDENT_BYTES{return Err("Comparison memory budget reached. Open this file alone for the full single-file preview.".into());}
            resident+=pane.resident_bytes;Ok(Arc::new(pane))
        });
        data.push(result.as_ref().ok().cloned());
        if token.is_cancelled() {
            return;
        }
        if events
            .try_send(Event::Pane {
                generation: job.generation,
                index,
                result,
            })
            .is_err()
        {
            return;
        }
        notify();
    }
    let mut pixel_budget = hover_core::comparison::MAX_DIFF_PIXELS;
    for index in 1..data.len() {
        if token.is_cancelled() {
            return;
        }
        let mut pair = PairData {
            byte_scan: unavailable("A pane is unavailable"),
            lines: None,
            pixel_stats: Vec::new(),
            masks: Vec::new(),
            note: String::new(),
            timing_equal: None,
        };
        if let (Some(left), Some(right)) = (&data[0], &data[index]) {
            pair.byte_scan = scan_bytes(&job.paths[0], &job.paths[index], token);
            match (&left.content, &right.content) {
                (Content::Rows { lines: a, .. }, Content::Rows { lines: b, .. }) => {
                    match compare_lines(a, b, token) {
                        Ok(lines) => {
                            pair.lines = Some(lines);
                            pair.note="Aligned displayed lines; omitted source content is outside this text comparison".into();
                        }
                        Err(error) => pair.note = error.into(),
                    }
                }
                (Content::Images { frames: a }, Content::Images { frames: b }) => {
                    pair.timing_equal = Some(
                        a.len() == b.len()
                            && a.iter().zip(b).all(|(a, b)| a.delay_ms == b.delay_ms),
                    );
                    let pixels = a.iter().try_fold(0usize, |sum, frame| {
                        sum.checked_add(frame.width as usize * frame.height as usize)
                    });
                    if a.len() != b.len() {
                        pair.note =
                            "Different decoded frame counts; pixel masks are unavailable".into();
                    } else if a
                        .iter()
                        .zip(b)
                        .any(|(a, b)| a.width != b.width || a.height != b.height)
                    {
                        pair.note="Different decoded dimensions; pixels are not resampled to imply alignment".into();
                    } else if pixels.is_none_or(|pixels| pixels > pixel_budget) {
                        pair.note =
                            "Decoded-pixel differences exceed the shared 8 Mi-pixel budget".into();
                    } else {
                        for (a, b) in a.iter().zip(b) {
                            match compare_pixels(
                                a.width,
                                a.height,
                                &a.pixels_bgra,
                                &b.pixels_bgra,
                                token,
                            ) {
                                Ok((stats, pixels)) => {
                                    pair.pixel_stats.push(stats);
                                    pair.masks.push(OverlayFrame {
                                        width: a.width,
                                        height: a.height,
                                        stride: a.stride,
                                        pixels_bgra: pixels,
                                        delay_ms: a.delay_ms,
                                    });
                                }
                                Err(error) => {
                                    pair.note = error.into();
                                    pair.masks.clear();
                                    pair.pixel_stats.clear();
                                    break;
                                }
                            }
                        }
                        if !pair.masks.is_empty() {
                            pixel_budget -= pixels.unwrap();
                            pair.note="Decoded preview BGRA pixels at equal coordinates; decoder scaling limits still apply".into();
                        }
                    }
                }
                _ => {
                    pair.note =
                        "Different preview kinds; use the metadata and independent panes".into()
                }
            }
        }
        if token.is_cancelled() {
            return;
        }
        if events
            .try_send(Event::Pair {
                generation: job.generation,
                index,
                result: Arc::new(pair),
            })
            .is_err()
        {
            return;
        }
        notify();
    }
}

fn observation(path: &Path) -> Result<(u64, Option<u128>, bool), String> {
    let m = local_metadata(path).map_err(|e| e.to_string())?;
    Ok((
        m.len(),
        m.modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos()),
        m.is_dir(),
    ))
}
fn load(
    path: &Path,
    token: &CancellationToken,
    settings: &PreviewTypeSettings,
) -> Result<PaneData, String> {
    let before = observation(path)?;
    let payload = build_preview_payload(path, token, settings)?;
    if token.is_cancelled() {
        return Err("Comparison cancelled".into());
    }
    let after = observation(path)?;
    if before != after {
        return Err("File changed while loading. Reload to inspect the new contents.".into());
    }
    let mut note = "Read independently; reload after external edits".to_string();
    let mut has_alpha = Vec::new();
    let mut stage = "bounded_text";
    let (content, resident_bytes) = match payload {
        PreviewPayload::Frames { frames } => {
            stage = "decoded_preview_frames";
            has_alpha = frames
                .iter()
                .map(|frame| {
                    frame
                        .pixels_bgra
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| pixel[3] != 255)
                })
                .collect();
            let bytes = frames
                .iter()
                .try_fold(0usize, |sum, frame| {
                    sum.checked_add(frame.pixels_bgra.len())
                })
                .ok_or("Decoded size overflow")?;
            (Content::Images { frames }, bytes)
        }
        PreviewPayload::Text {
            subtitle,
            lines,
            truncated,
            ..
        } => {
            if is_sims_package_path(path) {
                stage = "bounded_dbpf_index_metadata";
            }
            let bytes = lines.iter().map(String::len).sum();
            (
                Content::Rows {
                    subtitle,
                    lines,
                    truncated,
                },
                bytes,
            )
        }
        PreviewPayload::List {
            subtitle,
            entries,
            omitted,
            ..
        } => {
            stage = if before.2 {
                "bounded_directory_metadata"
            } else {
                "bounded_archive_index_metadata"
            };
            let potentially_omitted =
                omitted > 0 || (before.2 && entries.len() >= MAX_PREVIEW_ENTRIES);
            let lines: Vec<String> = entries
                .into_iter()
                .map(|entry| format!("{}   {}", entry.name, entry.detail))
                .collect();
            let bytes = lines.iter().map(String::len).sum();
            note="Displayed folder/archive metadata only; entries and dependencies are not recursively decoded".into();
            (
                Content::Rows {
                    subtitle,
                    lines,
                    truncated: potentially_omitted,
                },
                bytes,
            )
        }
        PreviewPayload::Video { .. } | PreviewPayload::Audio { .. } => {
            stage = "recognized_only";
            note="Media playback is available in the single-file preview; comparison never starts an external player".into();
            (
                Content::Rows {
                    subtitle: "Media recognized".into(),
                    lines: vec![note.clone()],
                    truncated: false,
                },
                note.len(),
            )
        }
    };
    Ok(PaneData {
        content,
        bytes: before.0,
        modified: before.1,
        is_dir: before.2,
        note,
        resident_bytes,
        has_alpha,
        stage,
    })
}

fn scan_bytes(left: &Path, right: &Path, token: &CancellationToken) -> ByteScan {
    let result = (|| -> Result<ByteScan, String> {
        let before_a = observation(left)?;
        let before_b = observation(right)?;
        if before_a.2 || before_b.2 {
            return Ok(unavailable("Directories are not recursively scanned"));
        }
        if before_a.0.max(before_b.0) > MAX_BYTE_SCAN {
            return Ok(unavailable(
                "Full-file byte scan exceeds the 128 MiB-per-file budget",
            ));
        }
        let mut a = open_preview_file(left).map_err(|e| e.to_string())?;
        let mut b = open_preview_file(right).map_err(|e| e.to_string())?;
        let mut x = [0u8; 64 * 1024];
        let mut y = [0u8; 64 * 1024];
        let mut examined = 0u64;
        let mut changed = 0u64;
        let length = before_a.0.max(before_b.0);
        while examined < length {
            if token.is_cancelled() {
                return Err("Comparison cancelled".into());
            }
            let an = (before_a.0.saturating_sub(examined)).min(x.len() as u64) as usize;
            let bn = (before_b.0.saturating_sub(examined)).min(y.len() as u64) as usize;
            a.read_exact(&mut x[..an]).map_err(|e| e.to_string())?;
            b.read_exact(&mut y[..bn]).map_err(|e| e.to_string())?;
            changed += x[..an].iter().zip(&y[..bn]).filter(|(a, b)| a != b).count() as u64
                + an.abs_diff(bn) as u64;
            examined += an.max(bn) as u64;
        }
        drop(a);
        drop(b);
        if before_a != observation(left)? || before_b != observation(right)? {
            return Err("File changed during the byte scan; reload to compare again".into());
        }
        Ok(ByteScan{state:"completed",examined,changed:Some(changed),note:"Byte positions read independently with shared write/delete access; not an atomic two-file snapshot".into()})
    })();
    result.unwrap_or_else(unavailable)
}
