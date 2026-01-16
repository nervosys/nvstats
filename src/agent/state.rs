//! System state extraction for AI agent context
//!
//! This module extracts relevant system state from the hardware monitor
//! to provide context for agent responses.

use crate::error::{SimonError, Result};
use crate::gpu::GpuInfo;
use crate::SiliconMonitor;
use serde::{Deserialize, Serialize};

use super::Query;

/// Condensed system state for agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// GPU information (only for queried GPUs)
    pub gpus: Vec<GpuState>,

    /// Timestamp of state capture
    pub timestamp: u64,
}

/// Condensed GPU state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuState {
    /// GPU index
    pub index: usize,

    /// GPU name
    pub name: String,

    /// GPU vendor
    pub vendor: String,

    /// Graphics utilization (0-100%)
    pub utilization: u32,

    /// Memory used (MB)
    pub memory_used_mb: u64,

    /// Memory total (MB)
    pub memory_total_mb: u64,

    /// GPU temperature (Celsius)
    pub temperature_c: u32,

    /// Power usage (Watts)
    pub power_w: f32,

    /// Power limit (Watts)
    pub power_limit_w: Option<f32>,

    /// GPU clock (MHz)
    pub clock_mhz: Option<u32>,

    /// Memory clock (MHz)
    pub memory_clock_mhz: Option<u32>,

    /// Fan speed (%)
    pub fan_speed_percent: Option<u32>,

    /// Number of processes using this GPU
    pub process_count: usize,
}

impl SystemState {
    /// Extract system state from monitor based on query
    pub fn from_monitor(monitor: &SiliconMonitor, query: &Query) -> Result<Self> {
        let gpu_infos = monitor
            .snapshot_gpus()
            .map_err(|e| SimonError::Other(format!("Failed to get GPU state: {}", e)))?;

        // Determine which GPUs to include
        let gpu_states: Vec<GpuState> = if query.all_gpus || query.gpu_indices.is_empty() {
            // Include all GPUs
            gpu_infos
                .into_iter()
                .enumerate()
                .map(|(idx, info)| Self::gpu_to_state(idx, info))
                .collect()
        } else {
            // Include only specified GPUs
            query
                .gpu_indices
                .iter()
                .filter_map(|&idx| {
                    gpu_infos
                        .get(idx)
                        .map(|info| Self::gpu_to_state(idx, info.clone()))
                })
                .collect()
        };

        Ok(Self {
            gpus: gpu_states,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Convert GpuInfo to condensed GpuState
    fn gpu_to_state(index: usize, info: GpuInfo) -> GpuState {
        GpuState {
            index,
            name: info.static_info.name,
            vendor: format!("{:?}", info.static_info.vendor),
            utilization: info.dynamic_info.utilization as u32,
            memory_used_mb: info.dynamic_info.memory.used / 1024 / 1024,
            memory_total_mb: info.dynamic_info.memory.total / 1024 / 1024,
            temperature_c: info.dynamic_info.thermal.temperature.unwrap_or(0) as u32,
            power_w: info.dynamic_info.power.draw.unwrap_or(0) as f32 / 1000.0,
            power_limit_w: info.dynamic_info.power.limit.map(|l| l as f32 / 1000.0),
            clock_mhz: info.dynamic_info.clocks.graphics,
            memory_clock_mhz: info.dynamic_info.clocks.memory,
            fan_speed_percent: info.dynamic_info.thermal.fan_speed.map(|f| f as u32),
            process_count: info.dynamic_info.processes.len(),
        }
    }

    /// Format state as natural language text for agent context
    pub fn to_context_string(&self) -> String {
        let mut context = String::new();
        context.push_str("Current System State:\n");

        for gpu in &self.gpus {
            context.push_str(&format!(
                "\nGPU {}: {} ({})\n",
                gpu.index, gpu.name, gpu.vendor
            ));
            context.push_str(&format!("  Utilization: {}%\n", gpu.utilization));
            context.push_str(&format!(
                "  Memory: {} / {} MB ({:.1}%)\n",
                gpu.memory_used_mb,
                gpu.memory_total_mb,
                (gpu.memory_used_mb as f32 / gpu.memory_total_mb as f32) * 100.0
            ));
            context.push_str(&format!("  Temperature: {}°C\n", gpu.temperature_c));
            context.push_str(&format!("  Power: {:.1}W", gpu.power_w));
            if let Some(limit) = gpu.power_limit_w {
                context.push_str(&format!(" / {:.1}W", limit));
            }
            context.push('\n');

            if let Some(clock) = gpu.clock_mhz {
                context.push_str(&format!("  GPU Clock: {} MHz\n", clock));
            }
            if let Some(mem_clock) = gpu.memory_clock_mhz {
                context.push_str(&format!("  Memory Clock: {} MHz\n", mem_clock));
            }
            if gpu.process_count > 0 {
                context.push_str(&format!("  Active Processes: {}\n", gpu.process_count));
            }
        }

        context
    }

    /// Get GPU state by index
    pub fn get_gpu(&self, index: usize) -> Option<&GpuState> {
        self.gpus.iter().find(|g| g.index == index)
    }

    /// Get all GPU states
    pub fn all_gpus(&self) -> &[GpuState] {
        &self.gpus
    }

    /// Calculate total power consumption
    pub fn total_power_w(&self) -> f32 {
        self.gpus.iter().map(|g| g.power_w).sum()
    }

    /// Get average GPU utilization
    pub fn avg_utilization(&self) -> f32 {
        if self.gpus.is_empty() {
            return 0.0;
        }
        let sum: u32 = self.gpus.iter().map(|g| g.utilization).sum();
        sum as f32 / self.gpus.len() as f32
    }

    /// Get average GPU temperature
    pub fn avg_temperature(&self) -> f32 {
        if self.gpus.is_empty() {
            return 0.0;
        }
        let sum: u32 = self.gpus.iter().map(|g| g.temperature_c).sum();
        sum as f32 / self.gpus.len() as f32
    }

    /// Get hottest GPU
    pub fn hottest_gpu(&self) -> Option<&GpuState> {
        self.gpus.iter().max_by_key(|g| g.temperature_c)
    }

    /// Get most utilized GPU
    pub fn most_utilized_gpu(&self) -> Option<&GpuState> {
        self.gpus.iter().max_by_key(|g| g.utilization)
    }
}

impl GpuState {
    /// Get memory usage percentage
    pub fn memory_usage_percent(&self) -> f32 {
        if self.memory_total_mb == 0 {
            return 0.0;
        }
        (self.memory_used_mb as f32 / self.memory_total_mb as f32) * 100.0
    }

    /// Get power usage percentage (if limit available)
    pub fn power_usage_percent(&self) -> Option<f32> {
        self.power_limit_w
            .map(|limit| (self.power_w / limit) * 100.0)
    }

    /// Check if GPU is thermally throttling (above 80°C)
    pub fn is_hot(&self) -> bool {
        self.temperature_c >= 80
    }

    /// Check if GPU is critically hot (above 90°C)
    pub fn is_critical(&self) -> bool {
        self.temperature_c >= 90
    }

    /// Check if GPU is heavily utilized (above 80%)
    pub fn is_busy(&self) -> bool {
        self.utilization >= 80
    }

    /// Check if GPU is idle (below 10%)
    pub fn is_idle(&self) -> bool {
        self.utilization < 10
    }

    /// Get health status summary
    pub fn health_status(&self) -> &str {
        if self.is_critical() {
            "CRITICAL: Temperature too high"
        } else if self.is_hot() {
            "WARNING: Temperature elevated"
        } else if self.memory_usage_percent() > 95.0 {
            "WARNING: Memory nearly full"
        } else if self.is_busy() {
            "BUSY: High utilization"
        } else if self.is_idle() {
            "IDLE: Low utilization"
        } else {
            "HEALTHY: Normal operation"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_state_calculations() {
        let gpu = GpuState {
            index: 0,
            name: "Test GPU".to_string(),
            vendor: "NVIDIA".to_string(),
            utilization: 75,
            memory_used_mb: 8000,
            memory_total_mb: 16000,
            temperature_c: 65,
            power_w: 150.0,
            power_limit_w: Some(200.0),
            clock_mhz: Some(1500),
            memory_clock_mhz: Some(6000),
            fan_speed_percent: Some(60),
            process_count: 3,
        };

        assert_eq!(gpu.memory_usage_percent(), 50.0);
        assert_eq!(gpu.power_usage_percent(), Some(75.0));
        assert!(!gpu.is_hot());
        assert!(!gpu.is_idle());
        assert!(!gpu.is_busy());
        assert_eq!(gpu.health_status(), "HEALTHY: Normal operation");
    }

    #[test]
    fn test_system_state_aggregations() {
        let state = SystemState {
            gpus: vec![
                GpuState {
                    index: 0,
                    name: "GPU 0".to_string(),
                    vendor: "NVIDIA".to_string(),
                    utilization: 50,
                    memory_used_mb: 4000,
                    memory_total_mb: 8000,
                    temperature_c: 60,
                    power_w: 100.0,
                    power_limit_w: Some(150.0),
                    clock_mhz: None,
                    memory_clock_mhz: None,
                    fan_speed_percent: None,
                    process_count: 2,
                },
                GpuState {
                    index: 1,
                    name: "GPU 1".to_string(),
                    vendor: "AMD".to_string(),
                    utilization: 80,
                    memory_used_mb: 6000,
                    memory_total_mb: 8000,
                    temperature_c: 75,
                    power_w: 120.0,
                    power_limit_w: Some(180.0),
                    clock_mhz: None,
                    memory_clock_mhz: None,
                    fan_speed_percent: None,
                    process_count: 1,
                },
            ],
            timestamp: 0,
        };

        assert_eq!(state.total_power_w(), 220.0);
        assert_eq!(state.avg_utilization(), 65.0);
        assert_eq!(state.avg_temperature(), 67.5);
        assert_eq!(state.hottest_gpu().unwrap().index, 1);
        assert_eq!(state.most_utilized_gpu().unwrap().index, 1);
    }
}
