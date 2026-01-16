//! Unified Process Monitoring with GPU Attribution
//!
//! This module provides cross-platform process monitoring with GPU usage attribution.
//! It combines system-wide process enumeration with GPU-specific process tracking from
//! NVIDIA NVML, AMD sysfs, and Intel GPU drivers.
//!
//! The [`ProcessMonitor`] correlates system processes with GPU usage by matching process IDs
//! (PIDs) from GPU driver data with information from `/proc` (Linux), task manager (Windows),
//! or similar platform-specific sources.
//!
//! # Examples
//!
//! ## Basic Process Monitoring
//!
//! ```no_run
//! use simon::{ProcessMonitor, GpuCollection};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create monitor with GPU attribution
//! let gpus = GpuCollection::auto_detect()?;
//! let mut monitor = ProcessMonitor::with_gpus(gpus)?;
//!
//! // Get all processes
//! let processes = monitor.processes()?;
//! println!("Total processes: {}", processes.len());
//!
//! // Get GPU processes only
//! let gpu_processes = monitor.gpu_processes()?;
//! println!("GPU processes: {}", gpu_processes.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Top GPU Consumers
//!
//! ```no_run
//! use simon::{ProcessMonitor, GpuCollection};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let gpus = GpuCollection::auto_detect()?;
//! let mut monitor = ProcessMonitor::with_gpus(gpus)?;
//!
//! // Get top 10 processes by GPU memory usage
//! let top_gpu = monitor.processes_by_gpu_memory()?;
//! println!("Top GPU consumers:");
//! for proc in top_gpu.iter().take(10) {
//!     println!("  {} (PID {}): {} MB on {} GPUs",
//!         proc.name,
//!         proc.pid,
//!         proc.total_gpu_memory_bytes / 1024 / 1024,
//!         proc.gpu_indices.len()
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Top CPU Consumers
//!
//! ```no_run
//! use simon::{ProcessMonitor, GpuCollection};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let gpus = GpuCollection::auto_detect()?;
//! let mut monitor = ProcessMonitor::with_gpus(gpus)?;
//!
//! // Get top 10 processes by CPU usage
//! let top_cpu = monitor.processes_by_cpu()?;
//! println!("Top CPU consumers:");
//! for proc in top_cpu.iter().take(10) {
//!     println!("  {} (PID {}): {:.1}%",
//!         proc.name,
//!         proc.pid,
//!         proc.cpu_percent
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Monitor Specific Process
//!
//! ```no_run
//! use simon::{ProcessMonitor, GpuCollection};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let gpus = GpuCollection::auto_detect()?;
//! let mut monitor = ProcessMonitor::with_gpus(gpus)?;
//!
//! // Get specific process by PID
//! if let Some(proc) = monitor.process_by_pid(1234)? {
//!     println!("Process: {}", proc.name);
//!     println!("CPU: {:.1}%", proc.cpu_percent);
//!     println!("Memory: {} MB", proc.memory_bytes / 1024 / 1024);
//!     println!("GPU Memory: {} MB", proc.total_gpu_memory_bytes / 1024 / 1024);
//!     println!("Using GPUs: {:?}", proc.gpu_indices);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Platform Support
//!
//! | Platform | Process Enum | GPU Attribution | CPU % | Memory | User |
//! |----------|--------------|-----------------|-------|--------|------|
//! | Linux    | ✅ /proc      | ✅ All vendors  | ✅    | ✅     | ✅   |
//! | Windows  | 🚧 Stubs     | 🚧              | 🚧    | 🚧     | 🚧   |
//! | macOS    | 🚧 Stubs     | 🚧              | 🚧    | 🚧     | 🚧   |

use crate::error::{SimonError, Result};
use crate::gpu::GpuCollection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GPU process type classification for process monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessGpuType {
    /// Process uses graphics rendering (3D, OpenGL, Vulkan, DirectX)
    Graphical,
    /// Process uses compute workloads (CUDA, OpenCL, SYCL)
    Compute,
    /// Process uses both graphics and compute
    GraphicalCompute,
    /// Unknown or unable to determine
    Unknown,
}

impl ProcessGpuType {
    /// Create from engine usage pattern
    pub fn from_engine_usage(gfx: u64, compute: u64) -> Self {
        match (gfx > 0, compute > 0) {
            (true, true) => Self::GraphicalCompute,
            (true, false) => Self::Graphical,
            (false, true) => Self::Compute,
            (false, false) => Self::Unknown,
        }
    }
}

impl std::fmt::Display for ProcessGpuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Graphical => write!(f, "Graphics"),
            Self::Compute => write!(f, "Compute"),
            Self::GraphicalCompute => write!(f, "Gfx+Compute"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Unified process information with GPU attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMonitorInfo {
    /// Process ID
    pub pid: u32,
    /// Process name/command
    pub name: String,
    /// User running the process
    pub user: Option<String>,
    /// CPU usage percentage (0-100 per core, can exceed 100 on multi-core)
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// GPU indices this process is using
    pub gpu_indices: Vec<usize>,
    /// GPU memory usage per GPU (GPU index -> memory in bytes)
    pub gpu_memory_per_device: HashMap<usize, u64>,
    /// Total GPU memory used across all GPUs
    pub total_gpu_memory_bytes: u64,
    /// Process state (R=Running, S=Sleeping, D=Disk sleep, Z=Zombie, T=Stopped)
    pub state: char,
    /// Process priority/nice value
    pub priority: Option<i32>,

    // nvtop feature parity: Per-process engine utilization
    /// GPU graphics engine time used (nanoseconds)
    pub gfx_engine_used: Option<u64>,
    /// GPU compute engine time used (nanoseconds)
    pub compute_engine_used: Option<u64>,
    /// GPU encoder time used (nanoseconds)
    pub enc_engine_used: Option<u64>,
    /// GPU decoder time used (nanoseconds)
    pub dec_engine_used: Option<u64>,
    /// GPU usage percentage (0-100)
    pub gpu_usage_percent: Option<f32>,
    /// Encoder usage percentage (0-100)
    pub encoder_usage_percent: Option<f32>,
    /// Decoder usage percentage (0-100)
    pub decoder_usage_percent: Option<f32>,
    /// GPU process type (Graphics, Compute, Mixed)
    pub gpu_process_type: ProcessGpuType,
    /// GPU memory percentage of total device memory
    pub gpu_memory_percentage: Option<f32>,
}

impl ProcessMonitorInfo {
    /// Get total CPU usage percentage
    pub fn cpu_usage(&self) -> f32 {
        self.cpu_percent
    }

    /// Get memory usage in megabytes
    pub fn memory_mb(&self) -> f64 {
        self.memory_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get total GPU memory usage in megabytes
    pub fn gpu_memory_mb(&self) -> f64 {
        self.total_gpu_memory_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Check if process is using any GPU
    pub fn is_gpu_process(&self) -> bool {
        !self.gpu_indices.is_empty()
    }

    /// Get number of GPUs used by this process
    pub fn gpu_count(&self) -> usize {
        self.gpu_indices.len()
    }
}

/// Process monitor that combines system and GPU process information
pub struct ProcessMonitor {
    /// GPU collection for GPU process tracking
    gpu_collection: Option<GpuCollection>,
    /// Cache of last update time (for CPU percentage calculation)
    last_update: std::time::Instant,
}

impl ProcessMonitor {
    /// Create a new process monitor
    ///
    /// Automatically detects available GPUs for GPU process attribution.
    pub fn new() -> Result<Self> {
        let gpu_collection = GpuCollection::auto_detect().ok();

        Ok(Self {
            gpu_collection,
            last_update: std::time::Instant::now(),
        })
    }

    /// Create a process monitor with a pre-initialized GPU collection
    ///
    /// This is useful when you already have a [`GpuCollection`] instance
    /// and want to reuse it for process monitoring.
    pub fn with_gpus(gpu_collection: GpuCollection) -> Result<Self> {
        Ok(Self {
            gpu_collection: Some(gpu_collection),
            last_update: std::time::Instant::now(),
        })
    }

    /// Create a process monitor without GPU tracking
    pub fn without_gpu() -> Result<Self> {
        Ok(Self {
            gpu_collection: None,
            last_update: std::time::Instant::now(),
        })
    }

    /// Get all running processes with GPU attribution
    pub fn processes(&mut self) -> Result<Vec<ProcessMonitorInfo>> {
        // Get system processes
        let mut system_processes = self.get_system_processes()?;

        // Add GPU information if available
        if let Some(ref gpu_collection) = self.gpu_collection {
            self.add_gpu_attribution(&mut system_processes, gpu_collection)?;
        }

        self.last_update = std::time::Instant::now();

        Ok(system_processes)
    }

    /// Get processes sorted by CPU usage (descending)
    pub fn processes_by_cpu(&mut self) -> Result<Vec<ProcessMonitorInfo>> {
        let mut procs = self.processes()?;
        procs.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(procs)
    }

    /// Get processes sorted by memory usage (descending)
    pub fn processes_by_memory(&mut self) -> Result<Vec<ProcessMonitorInfo>> {
        let mut procs = self.processes()?;
        procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        Ok(procs)
    }

    /// Get processes sorted by GPU memory usage (descending)
    pub fn processes_by_gpu_memory(&mut self) -> Result<Vec<ProcessMonitorInfo>> {
        let mut procs = self.processes()?;
        procs.sort_by(|a, b| b.total_gpu_memory_bytes.cmp(&a.total_gpu_memory_bytes));
        Ok(procs)
    }

    /// Get only GPU-using processes
    pub fn gpu_processes(&mut self) -> Result<Vec<ProcessMonitorInfo>> {
        let procs = self.processes()?;
        Ok(procs.into_iter().filter(|p| p.is_gpu_process()).collect())
    }

    /// Get process by PID
    pub fn process_by_pid(&mut self, pid: u32) -> Result<Option<ProcessMonitorInfo>> {
        let procs = self.processes()?;
        Ok(procs.into_iter().find(|p| p.pid == pid))
    }

    /// Get total number of running processes
    pub fn process_count(&mut self) -> Result<usize> {
        Ok(self.processes()?.len())
    }

    /// Get number of GPU-using processes
    pub fn gpu_process_count(&mut self) -> Result<usize> {
        Ok(self.gpu_processes()?.len())
    }

    /// Get number of detected GPUs
    pub fn gpu_count(&self) -> usize {
        self.gpu_collection.as_ref().map(|gc| gc.len()).unwrap_or(0)
    }

    /// Update process list (refresh data)
    ///
    /// This method triggers a refresh of the internal process cache.
    /// The actual update happens automatically in `processes()`, so this
    /// is primarily for explicit refresh semantics in user code.
    pub fn update(&mut self) -> Result<()> {
        // Refresh by calling processes() and discarding result
        let _ = self.processes()?;
        Ok(())
    }

    // Platform-specific system process enumeration
    #[cfg(target_os = "linux")]
    fn get_system_processes(&self) -> Result<Vec<ProcessMonitorInfo>> {
        linux::enumerate_processes()
    }

    #[cfg(target_os = "windows")]
    fn get_system_processes(&self) -> Result<Vec<ProcessMonitorInfo>> {
        windows_impl::enumerate_processes()
    }

    #[cfg(target_os = "macos")]
    fn get_system_processes(&self) -> Result<Vec<ProcessMonitorInfo>> {
        macos::enumerate_processes()
    }

    /// Add GPU attribution to processes
    fn add_gpu_attribution(
        &self,
        processes: &mut [ProcessMonitorInfo],
        gpu_collection: &GpuCollection,
    ) -> Result<()> {
        // Create a map of PID -> Process for quick lookup
        let mut process_map: HashMap<u32, &mut ProcessMonitorInfo> =
            processes.iter_mut().map(|p| (p.pid, p)).collect();

        // Iterate through each GPU and its processes
        for (gpu_idx, gpu) in gpu_collection.gpus().iter().enumerate() {
            if let Ok(gpu_processes) = gpu.processes() {
                for gpu_proc in gpu_processes {
                    let pid = gpu_proc.pid; // Field, not method
                    let gpu_mem = gpu_proc.memory_usage.unwrap_or(0);

                    if let Some(proc_info) = process_map.get_mut(&pid) {
                        // Add this GPU to the process's GPU list
                        if !proc_info.gpu_indices.contains(&gpu_idx) {
                            proc_info.gpu_indices.push(gpu_idx);
                        }

                        // Add GPU memory for this device
                        proc_info.gpu_memory_per_device.insert(gpu_idx, gpu_mem);
                        proc_info.total_gpu_memory_bytes += gpu_mem;
                    }
                }
            }
        }

        Ok(())
    }

    /// Kill a process by PID
    ///
    /// This method attempts to terminate a process. On Unix systems, it sends SIGTERM
    /// by default, which allows the process to clean up. On Windows, it terminates
    /// the process forcefully.
    ///
    /// # Safety
    ///
    /// Killing processes requires appropriate permissions:
    /// - On Linux/Unix: Must have permission to send signals to the target process
    /// - On Windows: Must have PROCESS_TERMINATE access rights
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID to terminate
    /// * `force` - If true, use SIGKILL (Unix) or forceful termination (Windows)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use simon::{ProcessMonitor, GpuCollection};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let gpus = GpuCollection::auto_detect()?;
    /// let mut monitor = ProcessMonitor::with_gpus(gpus)?;
    ///
    /// // Gracefully terminate process 1234
    /// monitor.kill_process(1234, false)?;
    ///
    /// // Force kill process 5678
    /// monitor.kill_process(5678, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn kill_process(&self, pid: u32, force: bool) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            let signal = if force { "KILL" } else { "TERM" };
            let status = Command::new("kill")
                .arg(format!("-{}", signal))
                .arg(pid.to_string())
                .status()
                .map_err(|e| SimonError::Io(e))?;

            if !status.success() {
                return Err(SimonError::Other(format!(
                    "Failed to kill process {}: {}",
                    pid,
                    status.code().unwrap_or(-1)
                )));
            }

            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::Threading::{
                OpenProcess, TerminateProcess, PROCESS_TERMINATE,
            };

            let _ = force; // Windows always uses forceful termination

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
                    SimonError::Other(format!("Failed to open process {}: {}", pid, e))
                })?;

                if handle.is_invalid() {
                    return Err(SimonError::Other(format!(
                        "Invalid handle for process {}",
                        pid
                    )));
                }

                let result = TerminateProcess(handle, 1);
                let _ = CloseHandle(handle);

                if result.is_err() {
                    return Err(SimonError::Other(format!(
                        "Failed to terminate process {}",
                        pid
                    )));
                }

                Ok(())
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            let signal = if force { "KILL" } else { "TERM" };
            let status = Command::new("kill")
                .arg(format!("-{}", signal))
                .arg(pid.to_string())
                .status()
                .map_err(|e| SimonError::Io(e))?;

            if !status.success() {
                return Err(SimonError::Other(format!(
                    "Failed to kill process {}: {}",
                    pid,
                    status.code().unwrap_or(-1)
                )));
            }

            Ok(())
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            let _ = (pid, force);
            Err(SimonError::UnsupportedPlatform(
                "Process termination not supported on this platform".to_string(),
            ))
        }
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            gpu_collection: None,
            last_update: std::time::Instant::now(),
        })
    }
}

// Linux-specific process enumeration
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs;
    use std::path::Path;

    pub fn enumerate_processes() -> Result<Vec<ProcessMonitorInfo>> {
        let mut processes = Vec::new();

        let proc_dir = Path::new("/proc");
        if !proc_dir.exists() {
            return Err(SimonError::UnsupportedPlatform(
                "/proc filesystem not available".to_string(),
            ));
        }

        // Read uptime for CPU calculation
        let uptime = read_uptime()?;

        // Iterate through /proc entries
        for entry in fs::read_dir(proc_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            // Check if directory name is a number (PID)
            if let Ok(pid) = filename_str.parse::<u32>() {
                if let Ok(proc_info) = read_process_info(pid, uptime) {
                    processes.push(proc_info);
                }
            }
        }

        Ok(processes)
    }

    fn read_uptime() -> Result<f64> {
        let uptime_str = fs::read_to_string("/proc/uptime")?;
        let uptime: f64 = uptime_str
            .split_whitespace()
            .next()
            .ok_or_else(|| SimonError::Parse("Invalid uptime format".to_string()))?
            .parse()
            .map_err(|e| SimonError::Parse(format!("Failed to parse uptime: {}", e)))?;
        Ok(uptime)
    }

    fn read_process_info(pid: u32, uptime: f64) -> Result<ProcessMonitorInfo> {
        let proc_path = format!("/proc/{}", pid);
        let proc_dir = Path::new(&proc_path);

        if !proc_dir.exists() {
            return Err(SimonError::DeviceNotFound(format!(
                "Process {} not found",
                pid
            )));
        }

        // Read /proc/[pid]/stat
        let stat_path = format!("{}/stat", proc_path);
        let stat_content = fs::read_to_string(&stat_path)?;

        // Parse stat file (fields documented in proc(5) man page)
        let (name, stat_fields) = parse_stat_line(&stat_content)?;

        if stat_fields.len() < 22 {
            return Err(SimonError::Parse(
                "Insufficient fields in stat file".to_string(),
            ));
        }

        // Extract fields (0-indexed after splitting on ')')
        let state = stat_fields[0].chars().next().unwrap_or('?');
        let utime: u64 = stat_fields[11].parse().unwrap_or(0);
        let stime: u64 = stat_fields[12].parse().unwrap_or(0);
        let priority: i32 = stat_fields[15].parse().unwrap_or(0);
        let starttime: u64 = stat_fields[19].parse().unwrap_or(0);

        // Calculate CPU percentage
        let clk_tck = 100.0; // SC_CLK_TCK, typically 100
        let total_time = (utime + stime) as f64 / clk_tck;
        let seconds_since_boot = uptime;
        let proc_uptime = (seconds_since_boot - (starttime as f64 / clk_tck)).max(1.0);
        let cpu_percent = ((total_time / proc_uptime) * 100.0) as f32;

        // Read /proc/[pid]/statm for memory
        let statm_path = format!("{}/statm", proc_path);
        let memory_bytes = if let Ok(statm_content) = fs::read_to_string(&statm_path) {
            let parts: Vec<&str> = statm_content.split_whitespace().collect();
            if parts.len() > 1 {
                // RSS (Resident Set Size) in pages, multiply by page size (typically 4KB)
                parts[1].parse::<u64>().unwrap_or(0) * 4096
            } else {
                0
            }
        } else {
            0
        };

        // Try to read user
        let user = read_process_user(pid);

        Ok(ProcessMonitorInfo {
            pid,
            name,
            user,
            cpu_percent,
            memory_bytes,
            gpu_indices: Vec::new(),
            gpu_memory_per_device: HashMap::new(),
            total_gpu_memory_bytes: 0,
            state,
            priority: Some(priority),
            gfx_engine_used: None,
            compute_engine_used: None,
            enc_engine_used: None,
            dec_engine_used: None,
            gpu_usage_percent: None,
            encoder_usage_percent: None,
            decoder_usage_percent: None,
            gpu_process_type: ProcessGpuType::Unknown,
            gpu_memory_percentage: None,
        })
    }

    fn parse_stat_line(stat: &str) -> Result<(String, Vec<String>)> {
        // Format: pid (name) state ...
        // Name can contain spaces and parentheses, so we need to find the last ')'
        let start = stat
            .find('(')
            .ok_or_else(|| SimonError::Parse("No opening parenthesis in stat".to_string()))?;
        let end = stat
            .rfind(')')
            .ok_or_else(|| SimonError::Parse("No closing parenthesis in stat".to_string()))?;

        let name = stat[start + 1..end].to_string();
        let rest = &stat[end + 2..]; // Skip ') '

        let fields: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();

        Ok((name, fields))
    }

    fn read_process_user(pid: u32) -> Option<String> {
        // Read UID from /proc/[pid]/status
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(content) = fs::read_to_string(&status_path) {
            for line in content.lines() {
                if line.starts_with("Uid:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        // Get real UID (first number after "Uid:")
                        if let Ok(uid) = parts[1].parse::<u32>() {
                            // Try to resolve UID to username
                            return get_username_from_uid(uid);
                        }
                    }
                }
            }
        }
        None
    }

    fn get_username_from_uid(uid: u32) -> Option<String> {
        // Simple approach: read /etc/passwd
        if let Ok(content) = fs::read_to_string("/etc/passwd") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 2 {
                    if let Ok(line_uid) = parts[2].parse::<u32>() {
                        if line_uid == uid {
                            return Some(parts[0].to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

// Windows-specific process enumeration
#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use ::windows::Win32::Foundation::CloseHandle;
    use ::windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use ::windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use ::windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_INFORMATION,
        PROCESS_VM_READ,
    };
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    pub fn enumerate_processes() -> Result<Vec<ProcessMonitorInfo>> {
        let mut processes = Vec::new();

        unsafe {
            // Take a snapshot of all processes
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| {
                SimonError::Other(format!("Failed to create process snapshot: {}", e))
            })?;

            if snapshot.is_invalid() {
                return Err(SimonError::Other("Invalid snapshot handle".to_string()));
            }

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            // Get first process
            if Process32FirstW(snapshot, &mut entry).is_err() {
                let _ = CloseHandle(snapshot);
                return Ok(processes);
            }

            // Iterate through all processes
            loop {
                let pid = entry.th32ProcessID;

                // Try to open process for querying
                let process_handle =
                    OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);

                if let Ok(handle) = process_handle {
                    if !handle.is_invalid() {
                        // Get process name
                        let mut name_buffer = [0u16; 260];
                        let mut name_len = name_buffer.len() as u32;

                        use ::windows::core::PWSTR;
                        let name = if QueryFullProcessImageNameW(
                            handle,
                            ::windows::Win32::System::Threading::PROCESS_NAME_WIN32,
                            PWSTR(name_buffer.as_mut_ptr()),
                            &mut name_len,
                        )
                        .is_ok()
                        {
                            let os_string = OsString::from_wide(&name_buffer[..name_len as usize]);
                            std::path::Path::new(&os_string)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string()
                        } else {
                            // Fallback to executable name from snapshot
                            let null_pos = entry
                                .szExeFile
                                .iter()
                                .position(|&c| c == 0)
                                .unwrap_or(entry.szExeFile.len());
                            String::from_utf16_lossy(&entry.szExeFile[..null_pos])
                        };

                        // Get memory info
                        let mut mem_counters = PROCESS_MEMORY_COUNTERS_EX::default();
                        let memory_bytes = if GetProcessMemoryInfo(
                            handle,
                            std::ptr::addr_of_mut!(mem_counters) as *mut _,
                            std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                        )
                        .is_ok()
                        {
                            mem_counters.WorkingSetSize as u64
                        } else {
                            0
                        };

                        // Get CPU times
                        let mut creation_time = Default::default();
                        let mut exit_time = Default::default();
                        let mut kernel_time = Default::default();
                        let mut user_time = Default::default();

                        let _cpu_time_ms = if GetProcessTimes(
                            handle,
                            &mut creation_time,
                            &mut exit_time,
                            &mut kernel_time,
                            &mut user_time,
                        )
                        .is_ok()
                        {
                            // Convert FILETIME (100ns units) to milliseconds
                            let kernel_100ns = (kernel_time.dwHighDateTime as u64) << 32
                                | (kernel_time.dwLowDateTime as u64);
                            let user_100ns = (user_time.dwHighDateTime as u64) << 32
                                | (user_time.dwLowDateTime as u64);
                            ((kernel_100ns + user_100ns) / 10_000) as u64
                        } else {
                            0
                        };

                        processes.push(ProcessMonitorInfo {
                            pid,
                            name,
                            user: None,       // Would need token information
                            cpu_percent: 0.0, // Would need multiple samples to calculate
                            memory_bytes,
                            gpu_indices: Vec::new(),
                            gpu_memory_per_device: HashMap::new(),
                            total_gpu_memory_bytes: 0,
                            state: 'R', // Windows doesn't expose state easily - assume running
                            priority: Some(entry.th32DefaultHeapID as i32), // Use heap ID as proxy for priority
                            gfx_engine_used: None,
                            compute_engine_used: None,
                            enc_engine_used: None,
                            dec_engine_used: None,
                            gpu_usage_percent: None,
                            encoder_usage_percent: None,
                            decoder_usage_percent: None,
                            gpu_process_type: ProcessGpuType::Unknown,
                            gpu_memory_percentage: None,
                        });

                        let _ = CloseHandle(handle);
                    }
                }

                // Move to next process
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }

            let _ = CloseHandle(snapshot);
        }

        Ok(processes)
    }
}

// macOS-specific process enumeration
#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::CStr;
    use std::mem;

    // macOS proc_info constants
    const PROC_PIDTASKALLINFO: i32 = 2;
    const PROC_PIDPATHINFO_MAXSIZE: usize = 4096;

    // FFI declarations for libproc
    #[link(name = "proc", kind = "dylib")]
    extern "C" {
        fn proc_listpids(proc_type: u32, type_info: u32, buffer: *mut u8, buffer_size: i32) -> i32;

        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut u8, buffer_size: i32) -> i32;

        fn proc_pidpath(pid: i32, buffer: *mut u8, buffer_size: u32) -> i32;
    }

    #[repr(C)]
    struct ProcTaskAllInfo {
        pbsd: ProcBsdInfo,
        ptinfo: ProcTaskInfo,
    }

    #[repr(C)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        _pad1: u32,
        pbi_comm: [u8; 16],
        pbi_name: [u8; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    #[repr(C)]
    struct ProcTaskInfo {
        pti_virtual_size: u64,
        pti_resident_size: u64,
        pti_total_user: u64,
        pti_total_system: u64,
        pti_threads_user: u64,
        pti_threads_system: u64,
        pti_policy: i32,
        pti_faults: i32,
        pti_pageins: i32,
        pti_cow_faults: i32,
        pti_messages_sent: i32,
        pti_messages_received: i32,
        pti_syscalls_mach: i32,
        pti_syscalls_unix: i32,
        pti_csw: i32,
        pti_threadnum: i32,
        pti_numrunning: i32,
        pti_priority: i32,
    }

    pub fn enumerate_processes() -> Result<Vec<ProcessMonitorInfo>> {
        let mut processes = Vec::new();

        unsafe {
            // First, get the number of processes
            let num_pids = proc_listpids(1, 0, std::ptr::null_mut(), 0); // PROC_ALL_PIDS = 1
            if num_pids <= 0 {
                return Err(SimonError::Other(
                    "Failed to get process count".to_string(),
                ));
            }

            // Allocate buffer for PIDs
            let buffer_size = num_pids * mem::size_of::<i32>() as i32;
            let mut pid_buffer = vec![0i32; (num_pids / mem::size_of::<i32>() as i32) as usize];

            // Get all PIDs
            let actual_size = proc_listpids(
                1, // PROC_ALL_PIDS
                0,
                pid_buffer.as_mut_ptr() as *mut u8,
                buffer_size,
            );

            if actual_size <= 0 {
                return Err(SimonError::Other("Failed to list processes".to_string()));
            }

            let num_processes = (actual_size / mem::size_of::<i32>() as i32) as usize;

            // Iterate through each PID
            for &pid in &pid_buffer[..num_processes] {
                if pid <= 0 {
                    continue;
                }

                // Get process info
                let mut task_info: ProcTaskAllInfo = mem::zeroed();
                let info_size = proc_pidinfo(
                    pid,
                    PROC_PIDTASKALLINFO,
                    0,
                    &mut task_info as *mut _ as *mut u8,
                    mem::size_of::<ProcTaskAllInfo>() as i32,
                );

                if info_size <= 0 {
                    continue; // Process may have exited
                }

                // Get process path
                let mut path_buffer = [0u8; PROC_PIDPATHINFO_MAXSIZE];
                let path_len = proc_pidpath(
                    pid,
                    path_buffer.as_mut_ptr(),
                    PROC_PIDPATHINFO_MAXSIZE as u32,
                );

                let name = if path_len > 0 {
                    // Extract filename from path
                    let path_slice = &path_buffer[..path_len as usize];
                    if let Ok(path_str) = CStr::from_bytes_until_nul(path_slice)
                        .map(|c| c.to_string_lossy().to_string())
                    {
                        std::path::Path::new(&path_str)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string()
                    } else {
                        // Fallback to comm name
                        let null_pos = task_info
                            .pbsd
                            .pbi_comm
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(task_info.pbsd.pbi_comm.len());
                        String::from_utf8_lossy(&task_info.pbsd.pbi_comm[..null_pos]).to_string()
                    }
                } else {
                    // Use process name from pbsd info
                    let null_pos = task_info
                        .pbsd
                        .pbi_comm
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(task_info.pbsd.pbi_comm.len());
                    String::from_utf8_lossy(&task_info.pbsd.pbi_comm[..null_pos]).to_string()
                };

                // Convert times from microseconds to milliseconds
                let cpu_time_ms =
                    (task_info.ptinfo.pti_total_user + task_info.ptinfo.pti_total_system) / 1000;

                // Status mapping
                let status = match task_info.pbsd.pbi_status {
                    1 => "Idle",
                    2 => "Running",
                    3 => "Sleeping",
                    4 => "Stopped",
                    5 => "Zombie",
                    _ => "Unknown",
                }
                .to_string();

                processes.push(ProcessMonitorInfo {
                    pid: pid as u32,
                    name,
                    user: Some(format!("{}", task_info.pbsd.pbi_uid)), // Convert UID to string
                    cpu_percent: 0.0, // Would need multiple samples to calculate
                    memory_bytes: task_info.ptinfo.pti_resident_size,
                    gpu_indices: Vec::new(),
                    gpu_memory_per_device: HashMap::new(),
                    total_gpu_memory_bytes: 0,
                    state: status.chars().next().unwrap_or('U'), // First char of status
                    priority: Some(task_info.ptinfo.pti_priority),
                    gfx_engine_used: None,
                    compute_engine_used: None,
                    enc_engine_used: None,
                    dec_engine_used: None,
                    gpu_usage_percent: None,
                    encoder_usage_percent: None,
                    decoder_usage_percent: None,
                    gpu_process_type: ProcessGpuType::Unknown,
                    gpu_memory_percentage: None,
                });
            }
        }

        Ok(processes)
    }
}
