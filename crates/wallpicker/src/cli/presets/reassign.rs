use super::{fail, print_json, saved};
use wallpicker_core::{presets::MonitorMapping, store::Store};

pub(super) fn run(store: &Store, args: &[&str], json: bool) -> i32 {
    let Some(id) = args.first().filter(|id| !id.starts_with("--")) else {
        return fail(&"usage: wallpicker presets reassign ID OLD=NEW ... [--preview] [--json]");
    };
    let mut mapping = MonitorMapping::new();
    let mut preview = false;
    for arg in &args[1..] {
        if *arg == "--preview" {
            preview = true;
            continue;
        }
        let Some((source, target)) = arg.split_once('=') else {
            return fail(&"monitor mappings must be OLD=NEW");
        };
        if mapping.insert(source.into(), target.into()).is_some() {
            return fail(&"each source monitor can be specified only once");
        }
    }
    if mapping.is_empty() {
        return fail(&"specify at least one OLD=NEW monitor mapping");
    }
    if preview {
        match store.preview_preset_reassignment(id, &mapping) {
            Ok(preset) if json => print_json(&preset),
            Ok(preset) => {
                for (source, target) in mapping {
                    println!("{source} → {target}");
                }
                if let Some(span) = preset.span {
                    println!("span: {}", span.monitors.join(" + "));
                }
                println!("preview only; preset unchanged");
                0
            }
            Err(error) => fail(&error.to_string()),
        }
    } else {
        saved(store.reassign_preset(id, &mapping, None), json)
    }
}
