// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2024 NervoSys

//! Windows disk monitoring via WMI and Windows Storage APIs
//!
//! This module provides disk monitoring for Windows using:
//! - WMI Win32_DiskDrive for device information
//! - Performance Counters for I/O statistics
//! - DeviceIoControl for SMART data

use crate::disk::traits::*;
use std::collections::HashMap;
use std::path::PathBuf;
use wmi::{COMLibrary, WMIConnection};

/// Create WMI connection with robust COM initialization
/// Handles cases where COM is already initialized by the GUI framework
fn create_wmi_connection() -> Result<WMIConnection, Error> {
    // Strategy 1: Fresh COM initialization (works best in background threads)
    if let Ok(com) = COMLibrary::new() {
        if let Ok(conn) = WMIConnection::with_namespace_path("root\\CIMV2", com) {
            return Ok(conn);
        }
    }

    // Strategy 2: COM without security init
    if let Ok(com) = COMLibrary::without_security() {
        if let Ok(conn) = WMIConnection::with_namespace_path("root\\CIMV2", com) {
            return Ok(conn);
        }
    }

    // Strategy 3: Assume COM is already initialized by the runtime (e.g., GUI apps)
    let com = unsafe { COMLibrary::assume_initialized() };
    WMIConnection::with_namespace_path("root\\CIMV2", com)
        .map_err(|e| Error::InitializationFailed(e.to_string()))
}

/// Windows disk device implementation
pub struct WindowsDisk {
    name: String,
    device_path: PathBuf,
    disk_type: DiskType,
    model: Option<String>,
    serial: Option<String>,
    size: u64,
}

impl WindowsDisk {
    pub fn new(
        name: String,
        device_path: PathBuf,
        disk_type: DiskType,
        model: Option<String>,
        serial: Option<String>,
        size: u64,
    ) -> Self {
        Self {
            name,
            device_path,
            disk_type,
            model,
            serial,
            size,
        }
    }

    /// Create from device index (e.g., 0 for PhysicalDrive0)
    pub fn from_index(index: u32) -> Result<Self, Error> {
        let name = format!("PhysicalDrive{}", index);
        let device_path = PathBuf::from(format!("\\\\.\\PhysicalDrive{}", index));

        // Try to detect disk type
        let disk_type = Self::detect_disk_type(index);

        Ok(Self {
            name,
            device_path,
            disk_type,
            model: None,
            serial: None,
            size: 0,
        })
    }

    /// Detect disk type from Windows APIs
    fn detect_disk_type(_index: u32) -> DiskType {
        // Would use IOCTL_STORAGE_QUERY_PROPERTY with StorageDeviceProperty
        // For now, default to unknown
        DiskType::Unknown
    }

    /// Read I/O statistics from WMI Performance Counters
    fn read_io_counters(&self) -> Result<(u64, u64, u64, u64), Error> {
        use serde::Deserialize;

        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "PascalCase")]
        #[allow(dead_code)]
        struct DiskPerf {
            name: String,
            disk_read_bytes_per_sec: u64,
            disk_write_bytes_per_sec: u64,
            disk_reads_per_sec: u32,
            disk_writes_per_sec: u32,
        }

        // Use robust WMI connection
        let wmi_con = create_wmi_connection()?;

        // Query disk performance data
        let query = format!(
            "SELECT Name, DiskReadBytesPerSec, DiskWriteBytesPerSec, DiskReadsPerSec, DiskWritesPerSec FROM Win32_PerfFormattedData_PerfDisk_PhysicalDisk WHERE Name LIKE '%{}'",
            self.name.replace("PhysicalDrive", "")
        );

        let perfs: Vec<DiskPerf> = wmi_con.raw_query(&query).unwrap_or_default();

        if let Some(perf) = perfs.first() {
            Ok((
                perf.disk_read_bytes_per_sec,
                perf.disk_write_bytes_per_sec,
                perf.disk_reads_per_sec as u64,
                perf.disk_writes_per_sec as u64,
            ))
        } else {
            // Try querying the "_Total" instance
            let total_perfs: Vec<DiskPerf> = wmi_con
                .raw_query("SELECT Name, DiskReadBytesPerSec, DiskWriteBytesPerSec, DiskReadsPerSec, DiskWritesPerSec FROM Win32_PerfFormattedData_PerfDisk_PhysicalDisk WHERE Name = '_Total'")
                .unwrap_or_default();

            if let Some(perf) = total_perfs.first() {
                Ok((
                    perf.disk_read_bytes_per_sec,
                    perf.disk_write_bytes_per_sec,
                    perf.disk_reads_per_sec as u64,
                    perf.disk_writes_per_sec as u64,
                ))
            } else {
                Ok((0, 0, 0, 0))
            }
        }
    }
}

impl DiskDevice for WindowsDisk {
    fn name(&self) -> &str {
        &self.name
    }

    fn disk_type(&self) -> DiskType {
        self.disk_type
    }

    fn info(&self) -> Result<DiskInfo, Error> {
        Ok(DiskInfo {
            name: self.name.clone(),
            model: self.model.clone().unwrap_or_else(|| "Unknown".to_string()),
            serial: self.serial.clone(),
            firmware: None,
            capacity: self.size,
            block_size: 512, // Most common
            disk_type: self.disk_type,
            physical_sector_size: Some(512),
            logical_sector_size: Some(512),
            rotation_rate: if matches!(self.disk_type, DiskType::NvmeSsd | DiskType::SataSsd) {
                Some(0)
            } else {
                Some(7200) // Common HDD speed
            },
            vendor: None,
        })
    }

    fn io_stats(&self) -> Result<DiskIoStats, Error> {
        let (read_bytes, write_bytes, read_ops, write_ops) = self.read_io_counters()?;

        Ok(DiskIoStats {
            read_bytes,
            write_bytes,
            read_ops,
            write_ops,
            read_time_ms: Some(0),
            write_time_ms: Some(0),
            queue_depth: Some(0),
            avg_latency_us: None,
            read_throughput: None,
            write_throughput: None,
        })
    }

    fn health(&self) -> Result<DiskHealth, Error> {
        // Would use DeviceIoControl with SMART_RCV_DRIVE_DATA
        // or WMI MSStorageDriver_FailurePredictStatus
        // For basic implementation, return Unknown
        Ok(DiskHealth::Unknown)
    }

    fn device_path(&self) -> PathBuf {
        self.device_path.clone()
    }
}

/// Enumerate Windows disk devices using WMI
pub fn enumerate() -> Result<Vec<Box<dyn DiskDevice>>, Error> {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct Win32DiskDrive {
        device_i_d: String,
        model: Option<String>,
        serial_number: Option<String>,
        size: Option<u64>,
        media_type: Option<String>,
        index: u32,
    }

    let mut disks: Vec<Box<dyn DiskDevice>> = Vec::new();

    // Use robust WMI connection that handles GUI context
    if let Ok(wmi_con) = create_wmi_connection() {
        let wmi_disks: Vec<Win32DiskDrive> = wmi_con
            .raw_query(
                "SELECT DeviceID, Model, SerialNumber, Size, MediaType, Index FROM Win32_DiskDrive",
            )
            .unwrap_or_default();

        for wmi_disk in wmi_disks {
            // Determine disk type - first check model name for SSDs since MediaType is often wrong
            let disk_type = {
                // Check model name first (most reliable for SSDs)
                if let Some(ref model) = wmi_disk.model {
                    let model_upper = model.to_uppercase();
                    if model_upper.contains("NVME")
                        || model_upper.contains("990 PRO")
                        || model_upper.contains("9100 PRO")
                    {
                        DiskType::NvmeSsd
                    } else if model_upper.contains("SSD")
                        || model_upper.contains("970 EVO")
                        || model_upper.contains("980 PRO")
                    {
                        DiskType::SataSsd
                    } else {
                        // Fall back to media type
                        match wmi_disk.media_type.as_deref() {
                            Some(media) if media.contains("SSD") || media.contains("Solid") => {
                                DiskType::SataSsd
                            }
                            Some(media) if media.contains("NVMe") => DiskType::NvmeSsd,
                            Some(media) if media.contains("Removable") => DiskType::Usb,
                            Some(media) if media.contains("Fixed") => DiskType::SataHdd,
                            _ => DiskType::Unknown,
                        }
                    }
                } else {
                    // No model - use media type
                    match wmi_disk.media_type.as_deref() {
                        Some(media) if media.contains("SSD") => DiskType::SataSsd,
                        Some(media) if media.contains("NVMe") => DiskType::NvmeSsd,
                        Some(media) if media.contains("Removable") => DiskType::Usb,
                        Some(media) if media.contains("Fixed") => DiskType::SataHdd,
                        _ => DiskType::Unknown,
                    }
                }
            };

            let disk = WindowsDisk::new(
                format!("PhysicalDrive{}", wmi_disk.index),
                PathBuf::from(format!("\\\\.\\PhysicalDrive{}", wmi_disk.index)),
                disk_type,
                wmi_disk.model,
                wmi_disk.serial_number.map(|s| s.trim().to_string()),
                wmi_disk.size.unwrap_or(0),
            );

            disks.push(Box::new(disk));
        }
    }

    // Fallback: try to enumerate physical drives directly
    if disks.is_empty() {
        for index in 0..8 {
            match WindowsDisk::from_index(index) {
                Ok(disk) => {
                    use std::fs::OpenOptions;
                    use std::os::windows::fs::OpenOptionsExt;

                    const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
                    const FILE_SHARE_READ: u32 = 0x00000001;
                    const FILE_SHARE_WRITE: u32 = 0x00000002;

                    let device_path = format!("\\\\.\\PhysicalDrive{}", index);

                    if let Ok(_file) = OpenOptions::new()
                        .read(true)
                        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                        .custom_flags(FILE_FLAG_NO_BUFFERING)
                        .open(&device_path)
                    {
                        disks.push(Box::new(disk));
                    }
                }
                Err(_) => continue,
            }
        }
    }

    // If no disks found, return error
    if disks.is_empty() {
        return Err(Error::NotFound);
    }

    Ok(disks)
}

/// Enhanced disk monitor with caching
pub struct WindowsDiskMonitor {
    disks: HashMap<String, WindowsDisk>,
}

impl WindowsDiskMonitor {
    pub fn new() -> Result<Self, Error> {
        let disks_vec = enumerate()?;
        let mut disks = HashMap::new();

        for disk in disks_vec {
            disks.insert(
                disk.name().to_string(),
                WindowsDisk {
                    name: disk.name().to_string(),
                    device_path: disk.device_path(),
                    disk_type: disk.disk_type(),
                    model: None,
                    serial: None,
                    size: 0,
                },
            );
        }

        Ok(Self { disks })
    }

    pub fn disks(&self) -> Vec<&WindowsDisk> {
        self.disks.values().collect()
    }

    pub fn disk_by_name(&self, name: &str) -> Option<&WindowsDisk> {
        self.disks.get(name)
    }
}

impl Default for WindowsDiskMonitor {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            disks: HashMap::new(),
        })
    }
}
