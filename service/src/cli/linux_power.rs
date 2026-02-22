use crate::types::{PowerCapabilities, PowerState};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Linux-native power management using kernel interfaces
#[derive(Clone)]
pub struct LinuxPower {
    amd_pstate: Option<AmdPStateBackend>,
    cpufreq: Option<CpufreqBackend>,
}

impl LinuxPower {
    pub async fn new() -> Result<Self, String> {
        info!("Detecting Linux power management capabilities...");

        let amd_pstate = AmdPStateBackend::detect().await;
        let cpufreq = CpufreqBackend::detect().await;

        // Log what we found
        if amd_pstate.is_some() {
            info!("AMD P-State detected: EPP control available");
        }
        if cpufreq.is_some() {
            info!("cpufreq detected: governor and frequency control available");
        }

        if amd_pstate.is_none() && cpufreq.is_none() {
            warn!("No power management interfaces found");
        }

        Ok(Self { amd_pstate, cpufreq })
    }

    pub async fn get_capabilities(&self) -> PowerCapabilities {
        // Aggregate all available capabilities from all backends
        let mut caps = PowerCapabilities::default();

        // AMD P-State capabilities (read dynamically as they change with governor)
        if let Some(amd_pstate) = &self.amd_pstate {
            caps.available_epp_preferences = amd_pstate.get_available_preferences().await;
            caps.supports_epp = caps.available_epp_preferences.is_some();
        }

        // cpufreq capabilities (read dynamically as they may change)
        if let Some(cpufreq) = &self.cpufreq {
            caps.supports_governor = true;
            caps.supports_frequency_limits = true;
            caps.available_governors = cpufreq.get_available_governors().await;
            if let Some((freq_min, freq_max)) = cpufreq.frequency_range {
                caps.frequency_min_mhz = if freq_min > 0 { Some(freq_min) } else { None };
                caps.frequency_max_mhz = if freq_max > 0 { Some(freq_max) } else { None };
            }
        }

        caps
    }

    pub async fn get_state(&self) -> Result<PowerState, String> {
        let mut state = PowerState::default();

        // Read AMD P-State state
        if let Some(amd_pstate) = &self.amd_pstate {
            state.epp_preference = amd_pstate.get_current_epp().await.ok();
        }

        // Read cpufreq state
        if let Some(cpufreq) = &self.cpufreq {
            state.governor = cpufreq.get_current_governor().await.ok();

            // Read live frequency range across cores
            if let Ok((min, max)) = cpufreq.get_live_frequency_range().await {
                state.min_freq_mhz = Some(min);
                state.max_freq_mhz = Some(max);
            }
        }

        Ok(state)
    }

    pub async fn get_configured_frequency_limits(&self) -> Result<(u32, u32), String> {
        if let Some(cpufreq) = &self.cpufreq {
            cpufreq.get_current_frequency_limits().await
        } else {
            Err("cpufreq backend not available".to_string())
        }
    }

    pub async fn set_governor(&self, governor: &str) -> Result<(), String> {
        let cpufreq = self.cpufreq.as_ref().ok_or("cpufreq backend not available")?;
        cpufreq.set_governor(governor).await
    }

    pub async fn set_epp_preference(&self, preference: &str) -> Result<(), String> {
        let amd_pstate = self.amd_pstate.as_ref().ok_or("AMD P-State backend not available")?;
        amd_pstate.set_epp_preference(preference).await
    }

    /// Applies frequency limits, clamping against hardware range.
    /// Pass None for either value to reset it to the hardware default.
    pub async fn set_freq_limits(&self, min_mhz: Option<u32>, max_mhz: Option<u32>) -> Result<(), String> {
        let cpufreq = self.cpufreq.as_ref().ok_or("cpufreq backend not available")?;
        let (hw_min, hw_max) = cpufreq.frequency_range.unwrap_or((0, u32::MAX));
        let max = max_mhz.unwrap_or(hw_max);
        let min = min_mhz.unwrap_or(hw_min).min(max);
        cpufreq.set_frequency_limits(min, max).await
    }
}

// AMD P-State Backend (EPP control)
#[derive(Clone)]
struct AmdPStateBackend {
    cpu_paths: Vec<PathBuf>,
}

impl AmdPStateBackend {
    async fn detect() -> Option<Self> {
        let cpu0_path = Path::new("/sys/devices/system/cpu/cpu0/cpufreq");
        if !cpu0_path.exists() {
            return None;
        }

        let epp_path = cpu0_path.join("energy_performance_preference");
        if !epp_path.exists() {
            return None;
        }

        let mut cpu_paths =
            collect_cpu_paths_with_file("cpufreq/energy_performance_preference").await;

        if cpu_paths.is_empty() {
            return None;
        }

        sort_cpu_paths(&mut cpu_paths);

        debug!("Found AMD P-State with {} CPUs", cpu_paths.len());

        Some(Self { cpu_paths })
    }

    async fn get_available_preferences(&self) -> Option<Vec<String>> {
        let first_cpu = self.cpu_paths.first()?;
        let available_path = first_cpu.join("cpufreq/energy_performance_available_preferences");
        read_sysfs_list(&available_path).await.ok()
    }

    async fn set_epp_preference(&self, preference: &str) -> Result<(), String> {
        if let Some(available) = self.get_available_preferences().await {
            if !available.iter().any(|p| p == preference) {
                warn!(
                    "Requested EPP preference '{}' not in available list {:?}; skipping",
                    preference, available
                );
                return Ok(());
            }
        }

        for (idx, cpu_path) in self.cpu_paths.iter().enumerate() {
            let epp_path = cpu_path.join("cpufreq/energy_performance_preference");
            write_sysfs_string(&epp_path, preference).await.map_err(|e| {
                format!("Failed to set EPP on CPU{}: {}", idx, e)
            })?;
        }
        Ok(())
    }

    async fn get_current_epp(&self) -> Result<String, String> {
        // Always read from cpu0 (first in sorted list)
        let first_cpu = self
            .cpu_paths
            .first()
            .ok_or_else(|| "No CPU paths available".to_string())?;
        let epp_path = first_cpu.join("cpufreq/energy_performance_preference");
        read_sysfs_string(&epp_path).await
    }
}

// Cpufreq Backend (Governor + frequency limits)
#[derive(Clone)]
struct CpufreqBackend {
    cpu_paths: Vec<PathBuf>,
    frequency_range: Option<(u32, u32)>,
}

impl CpufreqBackend {
    async fn detect() -> Option<Self> {
        let cpu0_path = Path::new("/sys/devices/system/cpu/cpu0/cpufreq");
        if !cpu0_path.exists() {
            return None;
        }

        let governor_path = cpu0_path.join("scaling_governor");
        if !governor_path.exists() {
            return None;
        }

        let min_freq_path = cpu0_path.join("cpuinfo_min_freq");
        let max_freq_path = cpu0_path.join("cpuinfo_max_freq");
        let frequency_range = if min_freq_path.exists() && max_freq_path.exists() {
            let min = read_sysfs_u64(&min_freq_path).await.ok().map(|v| v as u32);
            let max = read_sysfs_u64(&max_freq_path).await.ok().map(|v| v as u32);
            match (min, max) {
                (Some(min_khz), Some(max_khz)) => Some((min_khz / 1000, max_khz / 1000)),
                _ => None,
            }
        } else {
            None
        };

        let mut cpu_paths = collect_cpu_paths_with_file("cpufreq/scaling_governor").await;

        if cpu_paths.is_empty() {
            return None;
        }

        sort_cpu_paths(&mut cpu_paths);

        debug!(
            "Found cpufreq with {} CPUs, freq range: {:?} MHz",
            cpu_paths.len(),
            frequency_range
        );

        Some(Self {
            cpu_paths,
            frequency_range,
        })
    }

    /// Get available governors (read dynamically as they may change)
    async fn get_available_governors(&self) -> Option<Vec<String>> {
        let first_cpu = self.cpu_paths.first()?;
        let available_path = first_cpu.join("cpufreq/scaling_available_governors");
        read_sysfs_list(&available_path).await.ok()
    }

    async fn set_governor(&self, governor: &str) -> Result<(), String> {
        if let Some(available) = self.get_available_governors().await {
            if !available.iter().any(|g| g == governor) {
                warn!(
                    "Requested governor '{}' not in available list {:?}; skipping",
                    governor, available
                );
                return Ok(());
            }
        }

        for cpu_path in &self.cpu_paths {
            let gov_path = cpu_path.join("cpufreq/scaling_governor");
            write_sysfs_string(&gov_path, governor).await?;
        }
        debug!("Set governor to: {}", governor);
        Ok(())
    }

    async fn get_current_governor(&self) -> Result<String, String> {
        let first_cpu = self
            .cpu_paths
            .first()
            .ok_or_else(|| "No CPU paths available".to_string())?;
        let gov_path = first_cpu.join("cpufreq/scaling_governor");
        read_sysfs_string(&gov_path).await
    }

    async fn set_frequency_limits(&self, min_mhz: u32, max_mhz: u32) -> Result<(), String> {
        let min_khz = min_mhz * 1000;
        let max_khz = max_mhz * 1000;

        for cpu_path in &self.cpu_paths {
            let min_path = cpu_path.join("cpufreq/scaling_min_freq");
            let max_path = cpu_path.join("cpufreq/scaling_max_freq");
            // Always write max first so the kernel never sees min > max
            write_sysfs_u64(&max_path, max_khz as u64).await?;
            write_sysfs_u64(&min_path, min_khz as u64).await?;
        }
        debug!("Set frequency limits: {}-{} MHz", min_mhz, max_mhz);
        Ok(())
    }


    async fn get_live_frequency_range(&self) -> Result<(u32, u32), String> {
        let mut min_freq = u32::MAX;
        let mut max_freq = u32::MIN;

        for cpu_path in &self.cpu_paths {
            let cur_path = cpu_path.join("cpufreq/scaling_cur_freq");

            if let Ok(cur_khz) = read_sysfs_u64(&cur_path).await {
                let cur_mhz = (cur_khz / 1000) as u32;
                min_freq = min_freq.min(cur_mhz);
                max_freq = max_freq.max(cur_mhz);
            }
        }

        if min_freq != u32::MAX && max_freq != u32::MIN {
            Ok((min_freq, max_freq))
        } else {
            Err("Could not read current frequency from any CPU".to_string())
        }
    }

    async fn get_current_frequency_limits(&self) -> Result<(u32, u32), String> {
        let first_cpu = self
            .cpu_paths
            .first()
            .ok_or_else(|| "No CPU paths available".to_string())?;
        let min_path = first_cpu.join("cpufreq/scaling_min_freq");
        let max_path = first_cpu.join("cpufreq/scaling_max_freq");

        let min_khz = read_sysfs_u64(&min_path).await?;
        let max_khz = read_sysfs_u64(&max_path).await?;

        Ok(((min_khz / 1000) as u32, (max_khz / 1000) as u32))
    }
}

fn is_cpu_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

fn cpu_index(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let suffix = name.strip_prefix("cpu")?;
    suffix.parse::<u32>().ok()
}

fn sort_cpu_paths(cpu_paths: &mut Vec<PathBuf>) {
    cpu_paths.sort_by_key(|p| cpu_index(p).unwrap_or(u32::MAX));
}

async fn collect_cpu_paths_with_file(relative_path: &str) -> Vec<PathBuf> {
    let cpu_base = Path::new("/sys/devices/system/cpu");
    let mut cpu_paths = Vec::new();

    if let Ok(mut entries) = fs::read_dir(cpu_base).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if is_cpu_dir(&path) {
                let file_path = path.join(relative_path);
                if file_path.exists() {
                    cpu_paths.push(path);
                }
            }
        }
    }

    cpu_paths
}

// Sysfs utility functions
async fn read_sysfs_list(path: &Path) -> Result<Vec<String>, String> {
    let content: String = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    Ok(content.split_whitespace().map(String::from).collect())
}

async fn read_sysfs_u64(path: &Path) -> Result<u64, String> {
    let content: String = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    content
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

async fn read_sysfs_string(path: &Path) -> Result<String, String> {
    let content: String = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    Ok(content.trim().to_string())
}

async fn write_sysfs_u64(path: &Path, value: u64) -> Result<(), String> {
    fs::write(path, value.to_string())
        .await
        .map_err(|e| format!("Failed to write to {}: {}", path.display(), e))
}

async fn write_sysfs_string(path: &Path, value: &str) -> Result<(), String> {
    fs::write(path, value)
        .await
        .map_err(|e| format!("Failed to write to {}: {}", path.display(), e))
}
