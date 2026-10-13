use super::tests::Desktop;
use super::*;
use serde_json::json;
use std::cell::RefCell;

#[test]
fn old_daemon_is_rejected_without_a_legacy_show_or_state_change() {
    let desktop = Desktop::default();
    let batch = Batch::new(
        Intent::Apply,
        Default::default(),
        vec![Output {
            monitor: "DP-1".into(),
            picture: Picture::legacy("/wall.png".into(), None),
        }],
    );
    let calls = RefCell::new(Vec::new());
    let error = transport::send_with(
        &desktop,
        &batch,
        &|request| {
            calls.borrow_mut().push(request["cmd"].as_str().unwrap().to_owned());
            Err(Error::Invalid("wallpicker daemon: unknown request".into()))
        },
        &|_| {},
    )
    .unwrap_err();
    assert!(error.to_string().contains("restart"));
    assert_eq!(*calls.borrow(), ["capabilities"]);
    assert_eq!(*desktop.spawns.borrow(), 0);
}

#[test]
fn transport_starts_daemon_and_preserves_batch_including_partial_reply() {
    let desktop = Desktop::default();
    let batch = Batch::new(
        Intent::Restore,
        Default::default(),
        ["DP-1", "DP-2"]
            .into_iter()
            .map(|m| Output {
                monitor: m.into(),
                picture: Picture::legacy("/wall.png".into(), None),
            })
            .collect(),
    );
    let calls = RefCell::new(0);
    let report = transport::send_with(
        &desktop,
        &batch,
        &|request| {
            *calls.borrow_mut() += 1;
            match *calls.borrow() {
                1 => Err(Error::DaemonNotRunning),
                2 => Ok(json!({"render_version":VERSION})),
                _ => {
                    assert_eq!(request["cmd"], "render");
                    assert_eq!(request["batch"]["application"], batch.application);
                    assert_eq!(request["batch"]["outputs"].as_array().unwrap().len(), 2);
                    Ok(json!({"applied":["DP-1"],"failed":[["DP-2","disconnected"]]}))
                }
            }
        },
        &|_| {},
    )
    .unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    assert_eq!(report.failed[0].0, "DP-2");
    assert_eq!(*desktop.spawns.borrow(), 1);
}

#[test]
fn invalid_reports_and_duplicate_outputs_are_rejected() {
    let mut batch = Batch::new(
        Intent::Apply,
        Default::default(),
        vec![Output {
            monitor: "DP-1".into(),
            picture: Picture::legacy("/wall.png".into(), None),
        }],
    );
    assert!(Report::default().validate_for(&batch).is_err());
    assert!(
        Report {
            applied: vec!["DP-2".into()],
            failed: vec![]
        }
        .validate_for(&batch)
        .is_err()
    );
    assert!(
        Report {
            applied: vec!["DP-1".into()],
            failed: vec![("DP-1".into(), "failed".into())]
        }
        .validate_for(&batch)
        .is_err()
    );
    batch.outputs.push(batch.outputs[0].clone());
    assert!(batch.validate().is_err());
}
