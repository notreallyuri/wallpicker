//! Typed batch handling; preflight is independent of renderer startup for headless tests.
use super::Drawing;
use crate::preparation::{PreparedAdjustment, PreparedBatch};
use serde_json::Value;
use std::time::{Duration, Instant};
use wallpicker_core::{
    media::Limits,
    render::{Batch, Content, Intent, Report},
};

type PendingShow = (String, std::sync::mpsc::Receiver<Result<(), String>>);

enum Prepared {
    Apply(PreparedBatch),
    Adjust(PreparedAdjustment),
}

fn parse(request: &Value) -> Result<Batch, String> {
    serde_json::from_value(request.get("batch").cloned().ok_or("missing render batch")?).map_err(|e| e.to_string())
}

/// Preflight without decoder workers: static pictures, animations shown as
/// their first frame, and adjustments (which reuse a playing session).
fn prepare(batch: Batch) -> Result<Prepared, String> {
    if batch.intent == Intent::Adjust {
        PreparedAdjustment::new(batch, Limits::default()).map(Prepared::Adjust)
    } else {
        PreparedBatch::without_playback(batch).map(Prepared::Apply)
    }
}

pub(super) fn render(request: &Value, drawing: &Drawing) -> Result<Value, String> {
    let report = render_batch(parse(request)?, drawing)?;
    serde_json::to_value(report).map_err(|e| e.to_string())
}

/// Show `batch` and wait for every output's answer.
pub(super) fn render_batch(batch: Batch, drawing: &Drawing) -> Result<Report, String> {
    if batch.outputs.is_empty() {
        batch.validate().map_err(|e| e.to_string())?;
        return Ok(Report::default());
    }
    let animated = batch
        .outputs
        .iter()
        .any(|o| matches!(o.picture.content, Content::Animated { .. }));
    let answers = if animated && batch.intent != Intent::Adjust && super::playback::enabled(drawing) {
        play(batch, drawing)?
    } else {
        let prepared = prepare(batch)?;
        drawing.with(|renderer| match prepared {
            Prepared::Apply(prepared) => renderer.show_batch(prepared),
            Prepared::Adjust(prepared) => renderer.adjust_batch(prepared),
        })?
    };
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut report = Report::default();
    for (monitor, answer) in answers {
        match answer.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(Ok(())) => report.applied.push(monitor),
            Ok(Err(error)) => report.failed.push((monitor, error)),
            Err(error) => report
                .failed
                .push((monitor, format!("renderer did not confirm application: {error}"))),
        }
    }
    Ok(report)
}

/// Decoder workers start under the renderer lock, so their notifications go to
/// the renderer that takes them. If playback can't take them (admission, GPU
/// texture budget), the batch is shown as stills instead and the reason said.
fn play(batch: Batch, drawing: &Drawing) -> Result<Vec<PendingShow>, String> {
    drawing.with(|renderer| {
        let played = drawing
            .preparer
            .prepare(batch.clone(), renderer.decoder_wake())
            .and_then(|prepared| renderer.show_batch(prepared));
        match played {
            Ok(answers) => Ok(answers),
            Err(e) => {
                eprintln!("wallpicker: animated wallpaper shown as a still: {e}");
                renderer.show_batch(PreparedBatch::without_playback(batch)?)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallpicker_core::render::{Intent, Output, Picture};

    #[test]
    fn malformed_and_corrupt_batches_fail_before_renderer_startup() {
        let tmp = tempfile::tempdir().unwrap();
        let good = tmp.path().join("good.png");
        let bad = tmp.path().join("bad.png");
        wallpicker_core::compose::RgbImage::new(4, 4).save(&good).unwrap();
        std::fs::write(&bad, "broken").unwrap();
        let mut batch = Batch::new(
            Intent::Apply,
            Default::default(),
            vec![
                Output {
                    monitor: "DP-1".into(),
                    picture: Picture::legacy(good, None),
                },
                Output {
                    monitor: "DP-2".into(),
                    picture: Picture::legacy(bad, None),
                },
            ],
        );
        assert!(prepare(batch.clone()).is_err());
        batch.outputs.pop();
        assert!(prepare(batch.clone()).is_ok());
        batch.intent = Intent::Adjust;
        assert!(matches!(prepare(batch.clone()), Ok(Prepared::Adjust(_))));
        batch.version += 1;
        assert!(prepare(batch.clone()).err().unwrap().contains("restart"));
    }

    #[test]
    fn animated_adjustments_are_prepared_without_workers() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("source.gif");
        let mut bytes = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut bytes, 1, 1, &[255, 0, 0, 0, 255, 0]).unwrap();
            for color in [0, 1] {
                encoder
                    .write_frame(&gif::Frame {
                        width: 1,
                        height: 1,
                        delay: 10,
                        buffer: std::borrow::Cow::Owned(vec![color]),
                        ..Default::default()
                    })
                    .unwrap();
            }
        }
        std::fs::write(&file, bytes).unwrap();
        let source = wallpicker_core::media::Source::open(&file, Limits::default()).unwrap();
        let batch = Batch::new(
            Intent::Adjust,
            Default::default(),
            vec![Output {
                monitor: "A".into(),
                picture: Picture {
                    source: file,
                    geometry: Default::default(),
                    content: wallpicker_core::render::Content::Animated {
                        identity: source.identity().clone(),
                    },
                },
            }],
        );
        assert!(matches!(prepare(batch.clone()), Ok(Prepared::Adjust(_))));
    }
}
