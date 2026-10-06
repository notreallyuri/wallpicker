//! `wallpicker backup`: export, import and the automatic snapshots.

use wallpicker_core::store::{Dirs, Store};

use super::{fail, print_json, usage_for};

const USAGE: &str = "usage: wallpicker backup export [FILE]      the library (favorites, tags, fits) to one file\n       \
     wallpicker backup import FILE [--replace]  merge a backup in (or make the library exactly it)\n       \
     wallpicker backup snapshots [--json]     the automatic daily copies, newest first\n       \
     wallpicker backup restore ID             put a snapshot back (what's there now is kept as one)";

pub fn backup(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    let json = args.iter().any(|a| a == "--json");
    match args.first().map(String::as_str) {
        Some("export") => match crate::backup::export(&store, args.get(1).map(String::as_str)) {
            Ok(e) => {
                println!("exported {} wallpapers to {}", e.items, e.path.display());
                0
            }
            Err(e) => fail(&e),
        },
        Some("import") => {
            let Some(file) = args.get(1).filter(|a| !a.starts_with("--")) else {
                return usage_for("backup", "import needs the file", USAGE);
            };
            let replace = args.iter().any(|a| a == "--replace");
            match crate::backup::import(&store, file, replace) {
                Ok(r) if json => print_json(&r),
                Ok(r) => {
                    println!("found {} where they were, {} moved", r.same_place, r.moved);
                    if !r.missing.is_empty() {
                        println!("not in the library now ({}): {}", r.missing.len(), r.missing.join(", "));
                    }
                    0
                }
                Err(e) => fail(&e),
            }
        }
        Some("snapshots") => match store.snapshots() {
            Ok(list) if json => print_json(&list),
            Ok(list) => {
                if list.is_empty() {
                    println!("no snapshots yet (one is made before the first change each day)");
                }
                for s in list {
                    println!("{}  {}  ({} wallpaper{})", s.id, s.label, s.items, if s.items == 1 { "" } else { "s" });
                }
                0
            }
            Err(e) => fail(&e),
        },
        Some("restore") => {
            let Some(id) = args.get(1) else {
                return usage_for("backup", "restore needs a snapshot id (see `backup snapshots`)", USAGE);
            };
            match store.restore_snapshot(id) {
                Ok(c) => {
                    println!("restored {id} ({} wallpapers)", c.items.len());
                    0
                }
                Err(e) => fail(&e),
            }
        }
        _ => usage_for("backup", "use export, import, snapshots or restore", USAGE),
    }
}
