//! A process's GPU use, from the kernel's DRM client statistics
//! (`/proc/PID/fdinfo`, Documentation/gpu/drm-usage-stats.rst): engine time
//! and memory per client. amdgpu, i915/xe, nouveau and others report them;
//! NVIDIA's own driver doesn't (None then).

use std::collections::BTreeMap;

/// One DRM client (a process can hold several file descriptors to it).
#[derive(Debug, Default, PartialEq)]
pub(super) struct Client {
    pub id: String,
    /// The GPU's PCI address, for its whole-device busy percentage.
    pub pdev: Option<String>,
    /// Time spent on every engine (graphics, compute, video), in ns.
    pub engine_ns: u64,
    /// Memory region ("vram", "gtt", "system"...) -> bytes.
    pub memory: BTreeMap<String, u64>,
}

/// One fdinfo file's client; None for descriptors that aren't a GPU's.
pub(super) fn parse(text: &str) -> Option<Client> {
    let mut client = Client::default();
    let mut resident = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "drm-client-id" => client.id = value.to_string(),
            "drm-pdev" => client.pdev = Some(value.to_string()),
            key => {
                if let Some(engine) = key.strip_prefix("drm-engine-") {
                    // drm-engine-capacity-* counts engines, not time.
                    if !engine.starts_with("capacity") {
                        client.engine_ns += number(value).unwrap_or(0);
                    }
                } else if let Some(region) = key.strip_prefix("drm-memory-") {
                    client.memory.insert(region.to_string(), bytes(value)?);
                } else if let Some(region) = key.strip_prefix("drm-resident-") {
                    resident.insert(region.to_string(), bytes(value)?);
                }
            }
        }
    }
    if client.id.is_empty() {
        return None;
    }
    // Newer drivers report resident memory per region instead.
    if client.memory.is_empty() {
        client.memory = resident;
    }
    client.memory.retain(|_, b| *b > 0);
    Some(client)
}

/// The GPU clients `pid` holds, each once.
pub(super) fn clients(pid: u32) -> Vec<Client> {
    let mut found: Vec<Client> = Vec::new();
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fdinfo")) else {
        return found;
    };
    for entry in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        if let Some(client) = parse(&text)
            && !found.iter().any(|c| c.id == client.id)
        {
            found.push(client);
        }
    }
    found
}

/// The whole device's busy percentage (amdgpu), all programs together.
pub(super) fn device_busy(pdev: &str) -> Option<u32> {
    std::fs::read_to_string(format!("/sys/bus/pci/devices/{pdev}/gpu_busy_percent"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn number(value: &str) -> Option<u64> {
    value.split_whitespace().next()?.parse().ok()
}

fn bytes(value: &str) -> Option<u64> {
    let n = number(value)?;
    Some(match value.split_whitespace().nth(1) {
        Some("KiB") => n * 1024,
        Some("MiB") => n * 1024 * 1024,
        Some("GiB") => n * 1024 * 1024 * 1024,
        _ => n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amdgpu_engines_and_memory() {
        let text = "pos:\t0\nflags:\t02100002\ndrm-driver:\tamdgpu\ndrm-client-id:\t638\n\
                    drm-pdev:\t0000:2d:00.0\ndrm-memory-vram:\t46456 KiB\ndrm-memory-gtt: \t12436 KiB\n\
                    drm-memory-cpu: \t0 KiB\ndrm-engine-gfx:\t398060002 ns\ndrm-engine-dec:\t2 ns\n";
        let client = parse(text).unwrap();
        assert_eq!(client.id, "638");
        assert_eq!(client.pdev.as_deref(), Some("0000:2d:00.0"));
        assert_eq!(client.engine_ns, 398_060_004);
        assert_eq!(
            client.memory,
            BTreeMap::from([("gtt".into(), 12436 * 1024), ("vram".into(), 46456 * 1024)])
        );
    }

    #[test]
    fn resident_regions_and_engine_capacity() {
        // i915/xe style: capacity counts engines; memory as total/resident.
        let text = "drm-driver:\ti915\ndrm-client-id:\t7\ndrm-engine-render:\t1000 ns\n\
                    drm-engine-capacity-video:\t2\ndrm-total-system0:\t8 MiB\ndrm-resident-system0:\t4 MiB\n";
        let client = parse(text).unwrap();
        assert_eq!(client.engine_ns, 1000);
        assert_eq!(client.memory, BTreeMap::from([("system0".into(), 4 * 1024 * 1024)]));
    }

    #[test]
    fn other_descriptors_are_not_clients() {
        assert_eq!(parse("pos:\t0\nflags:\t02\nmnt_id:\t15\n"), None);
    }
}
