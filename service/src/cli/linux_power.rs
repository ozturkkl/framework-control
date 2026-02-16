use crate::types::{PowerCapabilities, PowerProfile, PowerState};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Linux-native power management using kernel interfaces
#[derive(Clone)]
pub struct LinuxPower {
    rapl: Option<RaplBackend>,
    amd_pstate: Option<AmdPStateBackend>,
    cpufreq: Option<CpufreqBackend>,
}



impl LinuxPower {
    pub async fn new() -> Result<Self, String> {
        info!("Detecting Linux power management capabilities...");

        let rapl = RaplBackend::detect().await;
        let amd_pstate = AmdPStateBackend::detect().await;
        let cpufreq = CpufreqBackend::detect().await;

        // Log what we found
        if let Some(ref r) = rapl {
            info!("RAPL detected: TDP control available");
            if !r.check_writable().await {
                r.log_permission_diagnostics().await;
            }
        }
        if amd_pstate.is_some() {
            info!("AMD P-State detected: EPP control available");
        }
        if cpufreq.is_some() {
            info!("cpufreq detected: governor and frequency control available");
        }

        if rapl.is_none() && amd_pstate.is_none() && cpufreq.is_none() {
            warn!("No power management interfaces found");
        }

        Ok(Self {
            rapl,
            amd_pstate,
            cpufreq,
        })
    }

    pub fn method_name(&self) -> String {
        let mut methods = Vec::new();
        if self.rapl.is_some() {
            methods.push("rapl");
        }
        if self.amd_pstate.is_some() {
            methods.push("amd-pstate");
        }
        if self.cpufreq.is_some() {
            methods.push("cpufreq");
        }

        if methods.is_empty() {
            "none".to_string()
        } else {
            methods.join("+")
        }
    }

    pub async fn get_capabilities(&self) -> PowerCapabilities {
        // Aggregate all available capabilities from all backends
        let mut caps = PowerCapabilities::default();

        // RAPL capabilities
        if let Some(rapl) = &self.rapl {
            caps.supports_tdp = true;
            let (tdp_min, tdp_max) = rapl.get_tdp_range().unwrap_or((15, 120));
            caps.tdp_min_watts = Some(tdp_min);
            caps.tdp_max_watts = Some(tdp_max);
        }

        // AMD P-State capabilities (read dynamically as they change with governor)
        if let Some(amd_pstate) = &self.amd_pstate {
            caps.supports_epp = true;
            caps.available_epp_preferences = amd_pstate.get_available_preferences().await;
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

        // Read RAPL state
        if let Some(rapl) = &self.rapl {
            state.tdp_limit_watts = rapl.get_tdp_limit().await.ok();
        }

        // Read AMD P-State state
        if let Some(amd_pstate) = &self.amd_pstate {
            state.epp_preference = amd_pstate.get_current_epp().await.ok();
        }

        // Read cpufreq state
        if let Some(cpufreq) = &self.cpufreq {
            state.governor = cpufreq.get_current_governor().await.ok();
            state.frequency_mhz = cpufreq.get_current_frequency().await.ok();

            // Read current frequency limits
            if let Ok((min, max)) = cpufreq.get_current_frequency_limits().await {
                state.min_freq_mhz = Some(min);
                state.max_freq_mhz = Some(max);
            }
        }

        Ok(state)
    }

    pub async fn apply_profile(&self, profile: &PowerProfile) -> Result<(), String> {
        // Apply RAPL settings
        if let Some(rapl) = &self.rapl {
            if let Some(tdp) = &profile.tdp_watts {
                if tdp.enabled && tdp.value > 0 {
                    info!("RAPL: Setting TDP to {}W", tdp.value);
                    rapl.set_tdp_watts(tdp.value).await?;
                }
            }
        }

        // Apply cpufreq governor FIRST (before EPP, as performance governor locks EPP)
        if let Some(cpufreq) = &self.cpufreq {
            if let Some(gov) = &profile.governor {
                if gov.enabled && !gov.value.is_empty() {
                    info!("cpufreq: Setting governor to '{}'", gov.value);
                    cpufreq.set_governor(&gov.value).await?;
                }
            }
        }

        // Apply AMD P-State settings (must come after governor change)
        if let Some(amd_pstate) = &self.amd_pstate {
            if let Some(epp) = &profile.epp_preference {
                if epp.enabled && !epp.value.is_empty() {
                    // Check if the requested EPP is available (performance governor locks EPP)
                    let available = amd_pstate.get_available_preferences().await;
                    let is_available = available
                        .as_ref()
                        .map(|prefs| prefs.iter().any(|p| p == &epp.value))
                        .unwrap_or(false);

                    if is_available {
                        info!("AMD P-State: Setting EPP to '{}'", epp.value);
                        amd_pstate.set_epp_preference(&epp.value).await?;
                    } else {
                        debug!("AMD P-State: Skipping EPP '{}' (not available with current governor)", epp.value);
                    }
                }
            }
        }

        // Apply cpufreq frequency limits
        if let Some(cpufreq) = &self.cpufreq {
            if let Some(min) = &profile.min_freq_mhz {
                if min.enabled && min.value > 0 {
                    if let Some(max) = &profile.max_freq_mhz {
                        if max.enabled && max.value > 0 {
                            info!("cpufreq: Setting frequency limits {}-{} MHz", min.value, max.value);
                            cpufreq.set_frequency_limits(min.value, max.value).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Legacy compatibility methods for drop-in replacement with RyzenAdj
    pub async fn set_tdp_watts(&self, watts: u32) -> Result<(), String> {
        if let Some(rapl) = &self.rapl {
            rapl.set_tdp_watts(watts).await
        } else {
            Err("TDP control not supported by active power method".to_string())
        }
    }

    pub async fn set_thermal_limit_c(&self, _celsius: u32) -> Result<(), String> {
        Err("Thermal limit control not yet implemented on Linux".to_string())
    }
}

// RAPL Backend (TDP control)
#[derive(Clone)]
struct RaplBackend {
    package_path: PathBuf,
}

impl RaplBackend {
    async fn detect() -> Option<Self> {
        let rapl_base = Path::new("/sys/class/powercap/intel-rapl");
        if !rapl_base.exists() {
            return None;
        }

        let mut entries = match fs::read_dir(rapl_base).await {
            Ok(e) => e,
            Err(_) => return None,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name_path = path.join("name");
            if let Ok(name) = fs::read_to_string(&name_path).await {
                if name.trim() == "package-0" {
                    // Check if constraint file exists (required for TDP control)
                    // On AMD systems, RAPL exists for monitoring but lacks power limit controls
                    let constraint_path = path.join("constraint_0_power_limit_uw");
                    if !constraint_path.exists() {
                        debug!(
                            "Found RAPL package-0 at {} but no constraint file (read-only, likely AMD)",
                            path.display()
                        );
                        return None;
                    }
                    debug!("Found RAPL package-0 at: {}", path.display());
                    return Some(Self { package_path: path });
                }
            }
        }

        None
    }

    fn get_tdp_range(&self) -> Option<(u32, u32)> {
        Some((15, 120))
    }

    async fn check_writable(&self) -> bool {
        let limit_path = self.package_path.join("constraint_0_power_limit_uw");

        // Try to read current value
        let current = match read_sysfs_u64(&limit_path).await {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Try to write the same value back (no-op write)
        write_sysfs_u64(&limit_path, current).await.is_ok()
    }

    async fn log_permission_diagnostics(&self) {
        use std::os::unix::fs::PermissionsExt;

        let limit_path = self.package_path.join("constraint_0_power_limit_uw");

        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        warn!("RAPL TDP Control Diagnostics");
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Check if running as root
        #[cfg(target_os = "linux")]
        {
            let euid = unsafe { libc::geteuid() };
            warn!("Running as UID: {} {}", euid, if euid == 0 { "(root)" } else { "(non-root - THIS IS THE PROBLEM!)" });
        }

        // Check file permissions
        if let Ok(metadata) = tokio::fs::metadata(&limit_path).await {
            let mode = metadata.permissions().mode();
            warn!("File: {}", limit_path.display());
            warn!("Permissions: {:o} (octal)", mode & 0o777);
            warn!("Writable: {} (this is what matters)", mode & 0o200 != 0);
        } else {
            warn!("Could not read file metadata for {}", limit_path.display());
        }

        // Check enabled file
        let enabled_path = self.package_path.join("enabled");
        if let Ok(content) = tokio::fs::read_to_string(&enabled_path).await {
            warn!("RAPL enabled: {}", content.trim());
        }

        // Try to detect kernel restriction
        let test_write = write_sysfs_u64(&limit_path, 15_000_000).await;
        match test_write {
            Ok(_) => {
                warn!("✓ Test write succeeded - RAPL is writable!");
            }
            Err(e) => {
                warn!("✗ Test write failed: {}", e);
                if e.contains("Permission denied") {
                    warn!("");
                    warn!("SOLUTION: RAPL writes are restricted by the kernel.");
                    warn!("");
                    warn!("To fix this, add kernel boot parameter:");
                    warn!("  intel_rapl.restrict_attr=N");
                    warn!("");
                    warn!("Instructions by distro:");
                    warn!("• NixOS: Add to boot.kernelParams in configuration.nix:");
                    warn!("    boot.kernelParams = [ \"intel_rapl.restrict_attr=N\" ];");
                    warn!("  Then: sudo nixos-rebuild switch");
                    warn!("");
                    warn!("• Ubuntu/Debian: Edit /etc/default/grub:");
                    warn!("    GRUB_CMDLINE_LINUX=\"intel_rapl.restrict_attr=N\"");
                    warn!("  Then: sudo update-grub && sudo reboot");
                    warn!("");
                    warn!("• Fedora/RHEL: Edit /etc/default/grub:");
                    warn!("    GRUB_CMDLINE_LINUX=\"intel_rapl.restrict_attr=N\"");
                    warn!("  Then: sudo grub2-mkconfig -o /boot/grub2/grub.cfg && sudo reboot");
                    warn!("");
                    warn!("• Arch: Edit /boot/loader/entries/*.conf or /etc/default/grub");
                    warn!("  Add intel_rapl.restrict_attr=N to kernel parameters");
                    warn!("");
                    warn!("After reboot, verify with:");
                    warn!("  cat /sys/module/intel_rapl_common/parameters/restrict_attr");
                    warn!("  (should show 'N')");
                }
            }
        }

        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    async fn set_tdp_watts(&self, watts: u32) -> Result<(), String> {
        let microwatts = (watts as u64) * 1_000_000;

        // Some systems require enabling the constraint first
        let enabled_path = self.package_path.join("enabled");
        if enabled_path.exists() {
            let _ = write_sysfs_u64(&enabled_path, 1).await;
        }

        let limit_path = self.package_path.join("constraint_0_power_limit_uw");

        write_sysfs_u64(&limit_path, microwatts)
            .await
            .map_err(|e| {
                format!(
                    "Failed to set TDP: {}. RAPL power limits are restricted by default on modern kernels. \
                    Fix: Add 'intel_rapl.restrict_attr=N' to kernel boot parameters (GRUB_CMDLINE_LINUX in /etc/default/grub), \
                    then run 'sudo update-grub' and reboot. Or run service as root with CAP_SYS_ADMIN capability.",
                    e
                )
            })?;

        debug!("Set TDP to {}W via RAPL", watts);
        Ok(())
    }

    async fn get_tdp_limit(&self) -> Result<u32, String> {
        let limit_path = self.package_path.join("constraint_0_power_limit_uw");
        let microwatts = read_sysfs_u64(&limit_path).await?;
        Ok((microwatts / 1_000_000) as u32)
    }

    // Note: RAPL energy_uj is cumulative energy counter, not instantaneous power
    // To calculate watts, we'd need to sample this over time intervals
    // Keeping this for potential future use
    #[allow(dead_code)]
    async fn get_energy_uj(&self) -> Result<u64, String> {
        let energy_path = self.package_path.join("energy_uj");
        read_sysfs_u64(&energy_path).await
    }
}

// AMD P-State Backend (EPP control)
#[derive(Clone)]
struct AmdPStateBackend {
    cpu_paths: Vec<PathBuf>,
}

impl AmdPStateBackend {
    /// Get available EPP preferences (read dynamically as they change with governor)
    async fn get_available_preferences(&self) -> Option<Vec<String>> {
        if let Some(first_cpu) = self.cpu_paths.first() {
            let available_path = first_cpu.join("cpufreq/energy_performance_available_preferences");
            if available_path.exists() {
                if let Ok(content) = fs::read_to_string(&available_path).await {
                    return Some(content.split_whitespace().map(String::from).collect());
                }
            }
        }
        // Fallback to common options if file doesn't exist
        Some(vec![
            "default".to_string(),
            "performance".to_string(),
            "balance_performance".to_string(),
            "balance_power".to_string(),
            "power".to_string(),
        ])
    }
    async fn detect() -> Option<Self> {
        let cpu0_path = Path::new("/sys/devices/system/cpu/cpu0/cpufreq");
        if !cpu0_path.exists() {
            return None;
        }

        let epp_path = cpu0_path.join("energy_performance_preference");
        if !epp_path.exists() {
            return None;
        }

        let cpu_base = Path::new("/sys/devices/system/cpu");
        let mut cpu_paths = Vec::new();

        if let Ok(mut entries) = fs::read_dir(cpu_base).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()))
                    .unwrap_or(false)
                {
                    let epp = path.join("cpufreq/energy_performance_preference");
                    if epp.exists() {
                        cpu_paths.push(path);
                    }
                }
            }
        }

        if cpu_paths.is_empty() {
            return None;
        }

        // Sort CPU paths by number (cpu0, cpu1, cpu2, ..., cpu10, cpu11, ...)
        cpu_paths.sort_by_key(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("cpu"))
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(999999)
        });

        debug!("Found AMD P-State with {} CPUs", cpu_paths.len());

        Some(Self { cpu_paths })
    }

    async fn set_epp_preference(&self, preference: &str) -> Result<(), String> {
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
        if let Some(first_cpu) = self.cpu_paths.first() {
            let epp_path = first_cpu.join("cpufreq/energy_performance_preference");
            read_sysfs_string(&epp_path).await
        } else {
            Err("No CPU paths available".to_string())
        }
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

        let available_path = cpu0_path.join("scaling_available_governors");
        let available_governors = if available_path.exists() {
            fs::read_to_string(&available_path)
                .await
                .ok()
                .map(|s| s.split_whitespace().map(String::from).collect::<Vec<String>>())
        } else {
            None
        };

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

        let cpu_base = Path::new("/sys/devices/system/cpu");
        let mut cpu_paths = Vec::new();

        if let Ok(mut entries) = fs::read_dir(cpu_base).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()))
                    .unwrap_or(false)
                {
                    let gov = path.join("cpufreq/scaling_governor");
                    if gov.exists() {
                        cpu_paths.push(path);
                    }
                }
            }
        }

        if cpu_paths.is_empty() {
            return None;
        }

        // Sort CPU paths by number (cpu0, cpu1, cpu2, ..., cpu10, cpu11, ...)
        cpu_paths.sort_by_key(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("cpu"))
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(999999)
        });

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
        if let Some(first_cpu) = self.cpu_paths.first() {
            let available_path = first_cpu.join("cpufreq/scaling_available_governors");
            if available_path.exists() {
                if let Ok(content) = fs::read_to_string(&available_path).await {
                    return Some(content.split_whitespace().map(String::from).collect());
                }
            }
        }
        None
    }

    async fn set_governor(&self, governor: &str) -> Result<(), String> {
        for cpu_path in &self.cpu_paths {
            let gov_path = cpu_path.join("cpufreq/scaling_governor");
            write_sysfs_string(&gov_path, governor).await?;
        }
        debug!("Set governor to: {}", governor);
        Ok(())
    }

    async fn get_current_governor(&self) -> Result<String, String> {
        if let Some(first_cpu) = self.cpu_paths.first() {
            let gov_path = first_cpu.join("cpufreq/scaling_governor");
            read_sysfs_string(&gov_path).await
        } else {
            Err("No CPU paths available".to_string())
        }
    }

    async fn set_frequency_limits(&self, min_mhz: u32, max_mhz: u32) -> Result<(), String> {
        let min_khz = min_mhz * 1000;
        let max_khz = max_mhz * 1000;

        for cpu_path in &self.cpu_paths {
            let min_path = cpu_path.join("cpufreq/scaling_min_freq");
            let max_path = cpu_path.join("cpufreq/scaling_max_freq");
            write_sysfs_u64(&min_path, min_khz as u64).await?;
            write_sysfs_u64(&max_path, max_khz as u64).await?;
        }
        debug!("Set frequency limits: {}-{} MHz", min_mhz, max_mhz);
        Ok(())
    }

    async fn get_current_frequency(&self) -> Result<u32, String> {
        if let Some(first_cpu) = self.cpu_paths.first() {
            let freq_path = first_cpu.join("cpufreq/scaling_cur_freq");
            let khz = read_sysfs_u64(&freq_path).await?;
            Ok((khz / 1000) as u32)
        } else {
            Err("No CPU paths available".to_string())
        }
    }

    async fn get_current_frequency_limits(&self) -> Result<(u32, u32), String> {
        // Read actual current frequencies from all CPUs and return min/max range
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
}

// Sysfs utility functions
async fn read_sysfs_u64(path: &Path) -> Result<u64, String> {
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    content
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

async fn read_sysfs_string(path: &Path) -> Result<String, String> {
    let content = fs::read_to_string(path)
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
