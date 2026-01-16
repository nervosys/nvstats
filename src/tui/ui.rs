//! UI rendering functions
//!
//! Glances-inspired single-screen layout with color-coded thresholds:
//! - Header: Title and system info with quicklook summary
//! - Hardware section: CPU, GPU, RAM, Disk, Network with trend indicators
//! - Process section: Sortable process list with color coding
//! - Footer: Help and controls
//!
//! Color thresholds (Glances-style):
//! - OK (Green): 0-50%
//! - CAREFUL (Cyan): 50-70%
//! - WARNING (Yellow): 70-90%
//! - CRITICAL (Red): 90-100%

#[allow(unused_imports)]
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Row, Sparkline, Table, Tabs},
    Frame,
};

use super::app::App;

// ═══════════════════════════════════════════════════════════════════════════════
// GLANCES-STYLE COLOR SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════

/// Glances-style threshold colors
mod glances_colors {
    use ratatui::style::Color;

    /// OK status - safe level (0-50%)
    pub const OK: Color = Color::Green;
    /// CAREFUL status - watch level (50-70%)
    pub const CAREFUL: Color = Color::Cyan;
    /// WARNING status - attention needed (70-90%)
    pub const WARNING: Color = Color::Yellow;
    /// CRITICAL status - urgent (90-100%)
    pub const CRITICAL: Color = Color::Red;
    /// Title/header color
    pub const TITLE: Color = Color::Cyan;
    /// Separator/border color
    pub const SEPARATOR: Color = Color::DarkGray;
    /// Inactive/disabled color
    pub const INACTIVE: Color = Color::DarkGray;
}

/// Get color based on percentage threshold (Glances-style)
/// - 0-50%: Green (OK)
/// - 50-70%: Cyan (CAREFUL)
/// - 70-90%: Yellow (WARNING)
/// - 90-100%: Red (CRITICAL)
fn threshold_color(percent: f32) -> Color {
    match percent {
        p if p >= 90.0 => glances_colors::CRITICAL,
        p if p >= 70.0 => glances_colors::WARNING,
        p if p >= 50.0 => glances_colors::CAREFUL,
        _ => glances_colors::OK,
    }
}

/// Get trend indicator arrow based on value change
/// Returns (arrow, color) tuple
fn trend_indicator(current: f32, previous: f32) -> (&'static str, Color) {
    let delta = current - previous;
    if delta.abs() < 0.5 {
        ("→", Color::DarkGray)
    } else if delta > 0.0 {
        ("↑", Color::Red)
    } else {
        ("↓", Color::Green)
    }
}

/// Format bytes to human-readable with auto unit (Glances-style)
fn auto_unit(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    match bytes {
        b if b >= TB => format!("{:.1}T", b as f64 / TB as f64),
        b if b >= GB => format!("{:.1}G", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1}M", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1}K", b as f64 / KB as f64),
        _ => format!("{}B", bytes),
    }
}

/// Main drawing function - nvtop-style single screen layout with bar gauges
/// Order: CPU(s), Accelerators (GPU/NPU/FPGA/etc.), RAM, Disk(s), Network
pub fn draw(f: &mut Frame, app: &App) {
    // Calculate dynamic constraints based on hardware and available space
    let cpu_section_height: u16 = 3; // 1 CPU bar
    let accelerator_section_height: u16 = if app.accelerators.is_empty() {
        0
    } else {
        (app.accelerators.len() * 3) as u16 // 3 lines per accelerator (compact bar style)
    };
    let ram_section_height: u16 = 3; // 1 RAM bar
    let disk_section_height: u16 = 3; // 1 Disk bar (aggregated)
    let network_section_height: u16 = 3; // 1 Network bar

    let hardware_height = cpu_section_height
        + accelerator_section_height
        + ram_section_height
        + disk_section_height
        + network_section_height;

    // Calculate remaining space for process list
    let total_height = f.area().height;
    let used_height = 3 + hardware_height + 3; // header + hardware + footer
    let process_height = total_height.saturating_sub(used_height).max(5);

    // Build constraints dynamically
    let mut constraints = vec![Constraint::Length(3)]; // Header
    constraints.push(Constraint::Length(cpu_section_height)); // CPU

    if accelerator_section_height > 0 {
        constraints.push(Constraint::Length(accelerator_section_height)); // Accelerators
    }

    constraints.push(Constraint::Length(ram_section_height)); // RAM
    constraints.push(Constraint::Length(disk_section_height)); // Disk
    constraints.push(Constraint::Length(network_section_height)); // Network
    constraints.push(Constraint::Length(process_height)); // Process list (dynamic)
    constraints.push(Constraint::Length(3)); // Footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut chunk_idx = 0;
    draw_nvtop_header(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    // Draw in order: CPU, Accelerators, RAM, Disk, Network
    draw_cpu_bar(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    if accelerator_section_height > 0 {
        draw_accelerators(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    draw_memory_bar(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    draw_disk_bar(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    draw_network_bar(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    draw_nvtop_processes(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    draw_nvtop_footer(f, app, chunks[chunk_idx]);
}

/// Draw header with Glances-style quicklook summary
fn draw_nvtop_header(f: &mut Frame, app: &App, area: Rect) {
    let uptime_secs = app.system_info.uptime.as_secs();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let minutes = (uptime_secs % 3600) / 60;

    // Format uptime like Glances
    let uptime_str = if days > 0 {
        format!("{}d {:02}:{:02}", days, hours, minutes)
    } else {
        format!("{:02}:{:02}", hours, minutes)
    };

    // CPU with threshold color
    let cpu_color = threshold_color(app.cpu_info.utilization);
    let cpu_span = Span::styled(
        format!("{:.0}%", app.cpu_info.utilization),
        Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
    );

    // Memory with threshold color
    let mem_percent = (app.memory_info.used as f64 / app.memory_info.total as f64) * 100.0;
    let mem_color = threshold_color(mem_percent as f32);
    let mem_span = Span::styled(
        format!("{:.0}%", mem_percent),
        Style::default().fg(mem_color).add_modifier(Modifier::BOLD),
    );

    // Swap with threshold color
    let swap_percent = if app.memory_info.swap_total > 0 {
        (app.memory_info.swap_used as f64 / app.memory_info.swap_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_color = threshold_color(swap_percent as f32);
    let swap_span = Span::styled(
        format!("{:.0}%", swap_percent),
        Style::default().fg(swap_color).add_modifier(Modifier::BOLD),
    );

    let header_text = vec![
        Span::styled(
            "Silicon Monitor",
            Style::default()
                .fg(glances_colors::TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(glances_colors::SEPARATOR)),
        Span::raw(format!(
            "{}@{}",
            app.system_info.hostname, app.system_info.os
        )),
        Span::styled(" │ ", Style::default().fg(glances_colors::SEPARATOR)),
        Span::styled("⏱", Style::default().fg(Color::White)),
        Span::raw(format!(" {} ", uptime_str)),
        Span::styled(" │ ", Style::default().fg(glances_colors::SEPARATOR)),
        // Quicklook style: CPU MEM SWAP
        Span::styled("CPU:", Style::default().fg(Color::White)),
        cpu_span,
        Span::raw(" "),
        Span::styled("MEM:", Style::default().fg(Color::White)),
        mem_span,
        Span::raw(" "),
        Span::styled("SWAP:", Style::default().fg(Color::White)),
        swap_span,
        Span::styled(" │ ", Style::default().fg(glances_colors::SEPARATOR)),
        Span::styled("ACCEL:", Style::default().fg(Color::White)),
        Span::styled(
            format!("{}", app.accelerators.len()),
            Style::default().fg(glances_colors::TITLE),
        ),
    ];

    let header = Paragraph::new(Line::from(header_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    f.render_widget(header, area);
}

/// Draw all accelerators (GPUs, NPUs, FPGAs, etc.) with detailed metrics
fn draw_accelerators(f: &mut Frame, app: &App, area: Rect) {
    if app.accelerators.is_empty() {
        let no_accel = Paragraph::new("No accelerators detected")
            .block(Block::default().borders(Borders::ALL).title("Accelerators"))
            .alignment(Alignment::Center);
        f.render_widget(no_accel, area);
        return;
    }

    // Split area for each accelerator
    let accel_count = app.accelerators.len();
    let constraints: Vec<Constraint> = std::iter::repeat(Constraint::Ratio(1, accel_count as u32))
        .take(accel_count)
        .collect();

    let accel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (idx, accel) in app.accelerators.iter().enumerate() {
        draw_single_accelerator(f, accel, idx, accel_chunks[idx]);
    }
}

/// Draw a single accelerator with all its metrics (Glances-style compact format)
fn draw_single_accelerator(
    f: &mut Frame,
    accel: &super::app::AcceleratorInfo,
    idx: usize,
    area: Rect,
) {
    let type_str = format!("{}", accel.accel_type);
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        format!("{} {} │ {} ({})", type_str, idx, accel.name, accel.vendor),
        Style::default()
            .fg(glances_colors::TITLE)
            .add_modifier(Modifier::BOLD),
    ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Memory percentage for threshold color
    let mem_percent = if accel.memory_total > 0 {
        (accel.memory_used as f64 / accel.memory_total as f64) * 100.0
    } else {
        0.0
    };

    // Compact: All key metrics with Glances-style formatting
    let accel_util_label = format!(
        "{}: {:.0}% @ {} MHz │ MEM: {}/{} ({:.0}%) @ {} MHz │ {:.0}°C │ {:.0}/{:.0}W",
        type_str,
        accel.utilization,
        accel.clock_core.unwrap_or(0),
        auto_unit(accel.memory_used),
        auto_unit(accel.memory_total),
        mem_percent,
        accel.clock_memory.unwrap_or(0),
        accel.temperature.unwrap_or(0.0),
        accel.power.unwrap_or(0.0),
        accel.power_limit.unwrap_or(0.0)
    );

    let accel_color = threshold_color(accel.utilization);

    let accel_gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(accel_color)
                .add_modifier(Modifier::BOLD),
        )
        .percent(accel.utilization as u16)
        .label(accel_util_label);
    f.render_widget(accel_gauge, inner);
}

/// Draw all GPU bars with detailed metrics (nvtop style) - DEPRECATED, use draw_accelerators
#[allow(dead_code)]
fn draw_nvtop_gpus(f: &mut Frame, app: &App, area: Rect) {
    if app.gpu_info.is_empty() {
        let no_gpu = Paragraph::new("No GPUs detected")
            .block(Block::default().borders(Borders::ALL).title("GPUs"))
            .alignment(Alignment::Center);
        f.render_widget(no_gpu, area);
        return;
    }

    // Split area for each GPU
    let gpu_count = app.gpu_info.len();
    let constraints: Vec<Constraint> = std::iter::repeat(Constraint::Ratio(1, gpu_count as u32))
        .take(gpu_count)
        .collect();

    let gpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (idx, gpu) in app.gpu_info.iter().enumerate() {
        draw_single_gpu(f, gpu, idx, gpu_chunks[idx]);
    }
}

/// Draw a single GPU with all its metrics (Glances-style compact format) - DEPRECATED, use draw_single_accelerator
#[allow(dead_code)]
fn draw_single_gpu(f: &mut Frame, gpu: &super::app::GpuInfo, idx: usize, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        format!("GPU {} │ {} ({})", idx, gpu.name, gpu.vendor),
        Style::default()
            .fg(glances_colors::TITLE)
            .add_modifier(Modifier::BOLD),
    ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Memory percentage for threshold color
    let mem_percent = if gpu.memory_total > 0 {
        (gpu.memory_used as f64 / gpu.memory_total as f64) * 100.0
    } else {
        0.0
    };

    // Compact: All key metrics with Glances-style formatting
    let gpu_util_label = format!(
        "GPU: {:.0}% @ {} MHz │ MEM: {}/{} ({:.0}%) @ {} MHz │ {:.0}°C │ {:.0}/{:.0}W",
        gpu.utilization,
        gpu.clock_graphics.unwrap_or(0),
        auto_unit(gpu.memory_used),
        auto_unit(gpu.memory_total),
        mem_percent,
        gpu.clock_memory.unwrap_or(0),
        gpu.temperature.unwrap_or(0.0),
        gpu.power.unwrap_or(0.0),
        gpu.power_limit.unwrap_or(0.0)
    );

    let gpu_color = threshold_color(gpu.utilization);

    let gpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(gpu_color).add_modifier(Modifier::BOLD))
        .percent(gpu.utilization as u16)
        .label(gpu_util_label);
    f.render_widget(gpu_gauge, inner);
}

/// Draw system monitoring graphs (DEPRECATED - bars now drawn individually in order)
#[allow(dead_code)]
fn draw_system_graphs(f: &mut Frame, app: &App, area: Rect) {
    let graph_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // CPU bar
            Constraint::Length(3), // RAM bar
            Constraint::Length(3), // Disk bar
            Constraint::Length(3), // Network bar
        ])
        .split(area);

    draw_cpu_bar(f, app, graph_chunks[0]);
    draw_memory_bar(f, app, graph_chunks[1]);
    draw_disk_bar(f, app, graph_chunks[2]);
    draw_network_bar(f, app, graph_chunks[3]);
}

/// Draw CPU utilization bar gauge with Glances-style formatting
fn draw_cpu_bar(f: &mut Frame, app: &App, area: Rect) {
    // Get previous CPU value for trend indicator
    let prev_cpu = app
        .cpu_history
        .iter()
        .rev()
        .nth(1)
        .map(|&v| v as f32)
        .unwrap_or(app.cpu_info.utilization);
    let (trend_arrow, _trend_color) = trend_indicator(app.cpu_info.utilization, prev_cpu);

    let cpu_label = format!(
        "CPU {} {:.0}% │ {} cores @ {} MHz │ {:.0}°C",
        trend_arrow,
        app.cpu_info.utilization,
        app.cpu_info.cores,
        app.cpu_info.frequency.unwrap_or(0),
        app.cpu_info.temperature.unwrap_or(0.0)
    );

    let cpu_color = threshold_color(app.cpu_info.utilization);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                "CPU",
                Style::default()
                    .fg(glances_colors::TITLE)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .gauge_style(Style::default().fg(cpu_color).add_modifier(Modifier::BOLD))
        .percent(app.cpu_info.utilization as u16)
        .label(cpu_label);

    f.render_widget(cpu_gauge, area);
}

/// Draw memory utilization bar gauge with Glances-style formatting
fn draw_memory_bar(f: &mut Frame, app: &App, area: Rect) {
    let mem_percent = ((app.memory_info.used as f64 / app.memory_info.total as f64) * 100.0) as u16;

    // Get previous memory value for trend indicator
    let prev_mem = app
        .memory_history
        .iter()
        .rev()
        .nth(1)
        .map(|&v| v as f32)
        .unwrap_or(mem_percent as f32);
    let (trend_arrow, _) = trend_indicator(mem_percent as f32, prev_mem);

    let mem_label = format!(
        "MEM {} {:.0}% │ {}/{} │ SWAP: {}",
        trend_arrow,
        mem_percent,
        auto_unit(app.memory_info.used),
        auto_unit(app.memory_info.total),
        auto_unit(app.memory_info.swap_used)
    );

    let mem_color = threshold_color(mem_percent as f32);

    let mem_gauge = Gauge::default()
        .block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                "Memory",
                Style::default()
                    .fg(glances_colors::TITLE)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .gauge_style(Style::default().fg(mem_color).add_modifier(Modifier::BOLD))
        .percent(mem_percent)
        .label(mem_label);

    f.render_widget(mem_gauge, area);
}

/// Draw disk usage bar gauge with Glances-style auto units
fn draw_disk_bar(f: &mut Frame, app: &App, area: Rect) {
    let total_space: u64 = app.disk_info.iter().map(|d| d.total).sum();
    let used_space: u64 = app.disk_info.iter().map(|d| d.used).sum();
    let disk_percent = if total_space > 0 {
        ((used_space as f64 / total_space as f64) * 100.0) as u16
    } else {
        0
    };

    // Build disk list string with Glances-style formatting
    let disk_list: Vec<String> = app
        .disk_info
        .iter()
        .take(3)
        .map(|d| {
            let percent = if d.total > 0 {
                (d.used as f64 / d.total as f64) * 100.0
            } else {
                0.0
            };
            format!("{}:{:.0}%", d.name, percent)
        })
        .collect();

    let disk_label = if !disk_list.is_empty() {
        format!(
            "DISK {:.0}% │ {}/{} │ {}",
            disk_percent,
            auto_unit(used_space),
            auto_unit(total_space),
            disk_list.join(" ")
        )
    } else {
        format!(
            "DISK {:.0}% │ {}/{} │ No disks",
            disk_percent,
            auto_unit(used_space),
            auto_unit(total_space)
        )
    };

    let disk_color = threshold_color(disk_percent as f32);

    let disk_gauge = Gauge::default()
        .block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                "Disk",
                Style::default()
                    .fg(glances_colors::TITLE)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .gauge_style(Style::default().fg(disk_color).add_modifier(Modifier::BOLD))
        .percent(disk_percent)
        .label(disk_label);

    f.render_widget(disk_gauge, area);
}

/// Draw network bar gauge with Glances-style formatting
fn draw_network_bar(f: &mut Frame, _app: &App, area: Rect) {
    // For Windows, show basic network info with Glances styling
    #[cfg(windows)]
    {
        let net_label = "NET │ Rx: -- │ Tx: -- │ Windows interface";
        let net_gauge = Gauge::default()
            .block(
                Block::default().borders(Borders::ALL).title(Span::styled(
                    "Network",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD),
                )),
            )
            .gauge_style(
                Style::default()
                    .fg(glances_colors::OK)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(50) // Placeholder
            .label(net_label);

        f.render_widget(net_gauge, area);
    }

    #[cfg(not(windows))]
    {
        let net_label = "NET │ Rx: -- │ Tx: -- │ Platform-specific";
        let net_gauge = Gauge::default()
            .block(
                Block::default().borders(Borders::ALL).title(Span::styled(
                    "Network",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD),
                )),
            )
            .gauge_style(
                Style::default()
                    .fg(glances_colors::INACTIVE)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(0)
            .label(net_label);

        f.render_widget(net_gauge, area);
    }
}

/// Draw CPU utilization graph with sparkline (DEPRECATED - use draw_cpu_bar)
#[allow(dead_code)]
fn draw_cpu_graph(f: &mut Frame, app: &App, area: Rect) {
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Info
            Constraint::Min(0),     // Graph
        ])
        .split(area);

    // CPU info
    let cpu_text = vec![
        Line::from(format!("CPU: {:.0}%", app.cpu_info.utilization)),
        Line::from(format!("{} cores", app.cpu_info.cores,)),
        Line::from(format!("@ {} MHz", app.cpu_info.frequency.unwrap_or(0))),
    ];
    let cpu_info = Paragraph::new(cpu_text)
        .block(Block::default().borders(Borders::ALL).title("CPU"))
        .style(Style::default().fg(Color::White));
    f.render_widget(cpu_info, inner_chunks[0]);

    // CPU history sparkline
    let cpu_data: Vec<u64> = app.cpu_history.iter().copied().collect();
    if !cpu_data.is_empty() {
        let sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
            .data(&cpu_data)
            .style(Style::default().fg(usage_color(app.cpu_info.utilization)));
        f.render_widget(sparkline, inner_chunks[1]);
    }
}

/// Draw memory utilization graph with sparkline (DEPRECATED - use draw_memory_bar)
#[allow(dead_code)]
fn draw_memory_graph(f: &mut Frame, app: &App, area: Rect) {
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Info
            Constraint::Min(0),     // Graph
        ])
        .split(area);

    // Memory info
    let mem_used_gb = app.memory_info.used as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_total_gb = app.memory_info.total as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_percent = (mem_used_gb / mem_total_gb) * 100.0;

    let mem_text = vec![
        Line::from(format!("RAM: {:.0}%", mem_percent)),
        Line::from(format!("{:.1} GB", mem_used_gb)),
        Line::from(format!("/ {:.1} GB", mem_total_gb)),
    ];
    let mem_info = Paragraph::new(mem_text)
        .block(Block::default().borders(Borders::ALL).title("Memory"))
        .style(Style::default().fg(Color::White));
    f.render_widget(mem_info, inner_chunks[0]);

    // Memory history sparkline
    let mem_data: Vec<u64> = app.memory_history.iter().copied().collect();
    if !mem_data.is_empty() {
        let sparkline = Sparkline::default()
            .block(Block::default().borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
            .data(&mem_data)
            .style(Style::default().fg(usage_color(mem_percent as f32)));
        f.render_widget(sparkline, inner_chunks[1]);
    }
}

/// Draw disk I/O information (DEPRECATED - use draw_disk_bar)
#[allow(dead_code)]
fn draw_disk_graph(f: &mut Frame, app: &App, area: Rect) {
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Info
            Constraint::Min(0),     // Stats
        ])
        .split(area);

    // Disk summary
    let _total_disks = app.disk_info.len();
    let total_space: u64 = app.disk_info.iter().map(|d| d.total).sum();
    let used_space: u64 = app.disk_info.iter().map(|d| d.used).sum();
    let total_gb = total_space as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_gb = used_space as f64 / (1024.0 * 1024.0 * 1024.0);
    let disk_percent = if total_space > 0 {
        (used_gb / total_gb) * 100.0
    } else {
        0.0
    };

    let disk_text = vec![
        Line::from(format!("Disk: {:.0}%", disk_percent)),
        Line::from(format!("{:.1} GB", used_gb)),
        Line::from(format!("/ {:.1} GB", total_gb)),
    ];
    let disk_info = Paragraph::new(disk_text)
        .block(Block::default().borders(Borders::ALL).title("Disk"))
        .style(Style::default().fg(Color::White));
    f.render_widget(disk_info, inner_chunks[0]);

    // Disk list spanning full width
    if !app.disk_info.is_empty() {
        let disk_items: Vec<Span> = app
            .disk_info
            .iter()
            .map(|disk| {
                let used = disk.used as f64 / (1024.0 * 1024.0 * 1024.0);
                let total = disk.total as f64 / (1024.0 * 1024.0 * 1024.0);
                let percent = if total > 0.0 {
                    (used / total) * 100.0
                } else {
                    0.0
                };
                Span::styled(
                    format!(" {}: {:.0}% ", disk.name, percent),
                    Style::default().fg(usage_color(percent as f32)),
                )
            })
            .collect();

        let disk_list = Paragraph::new(Line::from(disk_items))
            .block(Block::default().borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
            .style(Style::default().fg(Color::White));
        f.render_widget(disk_list, inner_chunks[1]);
    } else {
        let no_disk = Paragraph::new("No disks detected")
            .block(Block::default().borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(no_disk, inner_chunks[1]);
    }
}

/// Draw network information (DEPRECATED - use draw_network_bar)
#[allow(dead_code)]
fn draw_network_graph(f: &mut Frame, _app: &App, area: Rect) {
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Info
            Constraint::Min(0),     // Stats
        ])
        .split(area);

    // Network summary (placeholder for now)
    let net_text = vec![
        Line::from("Network: N/A"),
        Line::from(""),
        Line::from("(Win: Not impl)"),
    ];
    let net_info = Paragraph::new(net_text)
        .block(Block::default().borders(Borders::ALL).title("Network"))
        .style(Style::default().fg(Color::White));
    f.render_widget(net_info, inner_chunks[0]);

    // Network placeholder
    let net_placeholder = Paragraph::new("Network monitoring requires Linux/macOS")
        .block(Block::default().borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(net_placeholder, inner_chunks[1]);
}

/// Draw GPU processes table (nvtop style)
fn draw_nvtop_processes(f: &mut Frame, app: &App, area: Rect) {
    let mode_name = app.process_mode_name();
    let processes = app.get_filtered_processes();

    // Determine columns based on mode - Glances-style headers
    let (header, rows) = match app.process_display_mode {
        super::app::ProcessDisplayMode::All | super::app::ProcessDisplayMode::Cpu => {
            let header = Row::new(vec![
                Span::styled(
                    "PID",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "USER",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "COMMAND",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "CPU%",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "MEM",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ])
            .bottom_margin(1);

            let rows: Vec<Row> = processes
                .iter()
                .take(20) // Limit to 20 visible processes
                .map(|p| {
                    // Use Glances threshold colors
                    let cpu_color = threshold_color(p.cpu_percent);

                    Row::new(vec![
                        Span::styled(format!("{:>7}", p.pid), Style::default().fg(Color::White)),
                        Span::styled(
                            format!(
                                "{:<10}",
                                p.user
                                    .as_deref()
                                    .unwrap_or("?")
                                    .chars()
                                    .take(10)
                                    .collect::<String>()
                            ),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(p.name.clone(), Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{:>5.1}%", p.cpu_percent),
                            Style::default().fg(cpu_color),
                        ),
                        Span::styled(auto_unit(p.memory_bytes), Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            (header, rows)
        }
        super::app::ProcessDisplayMode::Gpu(gpu_idx) => {
            let header = Row::new(vec![
                Span::styled(
                    "PID",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "USER",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "COMMAND",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "GPU MEM",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "GPU%",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "TYPE",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ])
            .bottom_margin(1);

            let rows: Vec<Row> = processes
                .iter()
                .take(20)
                .map(|p| {
                    let gpu_mem = p
                        .gpu_memory_per_device
                        .get(&gpu_idx)
                        .map(|&m| auto_unit(m))
                        .unwrap_or_else(|| "0B".to_string());

                    let gpu_usage = p
                        .gpu_usage_percent
                        .map(|u| format!("{:>5.1}%", u))
                        .unwrap_or_else(|| "  N/A".to_string());

                    // Use Glances threshold colors for GPU usage
                    let gpu_color = threshold_color(p.gpu_usage_percent.unwrap_or(0.0));

                    let proc_type = format!("{:?}", p.gpu_process_type);

                    Row::new(vec![
                        Span::styled(format!("{:>7}", p.pid), Style::default().fg(Color::White)),
                        Span::styled(
                            format!(
                                "{:<10}",
                                p.user
                                    .as_deref()
                                    .unwrap_or("?")
                                    .chars()
                                    .take(10)
                                    .collect::<String>()
                            ),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(p.name.clone(), Style::default().fg(Color::White)),
                        Span::styled(format!("{:>7}", gpu_mem), Style::default().fg(gpu_color)),
                        Span::styled(gpu_usage, Style::default().fg(gpu_color)),
                        Span::styled(proc_type, Style::default().fg(glances_colors::INACTIVE)),
                    ])
                })
                .collect();

            (header, rows)
        }
        super::app::ProcessDisplayMode::Npu(_) => {
            let header = Row::new(vec![Span::styled(
                "No NPU processes available",
                Style::default().fg(glances_colors::INACTIVE),
            )]);
            (header, vec![])
        }
        super::app::ProcessDisplayMode::Accelerator(accel_idx) => {
            let header = Row::new(vec![
                Span::styled(
                    "PID",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "USER",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "COMMAND",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "ACCEL MEM",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "ACCEL%",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    "TYPE",
                    Style::default()
                        .fg(glances_colors::TITLE)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ])
            .bottom_margin(1);

            let rows: Vec<Row> = processes
                .iter()
                .take(20)
                .map(|p| {
                    let accel_mem = p
                        .gpu_memory_per_device
                        .get(&accel_idx)
                        .map(|&m| auto_unit(m))
                        .unwrap_or_else(|| "0B".to_string());

                    let accel_usage = p
                        .gpu_usage_percent
                        .map(|u| format!("{:>5.1}%", u))
                        .unwrap_or_else(|| "  N/A".to_string());

                    let accel_color = threshold_color(p.gpu_usage_percent.unwrap_or(0.0));

                    let proc_type = format!("{:?}", p.gpu_process_type);

                    Row::new(vec![
                        Span::styled(format!("{:>7}", p.pid), Style::default().fg(Color::White)),
                        Span::styled(
                            format!(
                                "{:<10}",
                                p.user
                                    .as_deref()
                                    .unwrap_or("?")
                                    .chars()
                                    .take(10)
                                    .collect::<String>()
                            ),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(p.name.clone(), Style::default().fg(Color::White)),
                        Span::styled(
                            format!("{:>9}", accel_mem),
                            Style::default().fg(accel_color),
                        ),
                        Span::styled(accel_usage, Style::default().fg(accel_color)),
                        Span::styled(proc_type, Style::default().fg(glances_colors::INACTIVE)),
                    ])
                })
                .collect();

            (header, rows)
        }
    };

    // Define column widths
    let widths = match app.process_display_mode {
        super::app::ProcessDisplayMode::All | super::app::ProcessDisplayMode::Cpu => {
            vec![
                Constraint::Length(8),  // PID
                Constraint::Length(12), // User
                Constraint::Min(20),    // Name (flexible)
                Constraint::Length(8),  // CPU%
                Constraint::Length(12), // Memory
            ]
        }
        super::app::ProcessDisplayMode::Gpu(_) => {
            vec![
                Constraint::Length(8),  // PID
                Constraint::Length(12), // User
                Constraint::Min(15),    // Name (flexible)
                Constraint::Length(12), // GPU Mem
                Constraint::Length(8),  // GPU%
                Constraint::Length(10), // Type
            ]
        }
        super::app::ProcessDisplayMode::Npu(_) => {
            vec![Constraint::Percentage(100)]
        }
        super::app::ProcessDisplayMode::Accelerator(_) => {
            vec![
                Constraint::Length(8),  // PID
                Constraint::Length(12), // User
                Constraint::Min(15),    // Name (flexible)
                Constraint::Length(12), // Accel Mem
                Constraint::Length(8),  // Accel%
                Constraint::Length(10), // Type
            ]
        }
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Processes - {} ({} shown)",
            mode_name,
            processes.len().min(20)
        )))
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Draw footer with controls (Glances-style hotkey display)
fn draw_nvtop_footer(f: &mut Frame, _app: &App, area: Rect) {
    let help_text = vec![
        Span::styled(
            "q",
            Style::default()
                .fg(glances_colors::TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(glances_colors::TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Process  "),
        Span::styled(
            "r",
            Style::default()
                .fg(glances_colors::TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Reset  "),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(glances_colors::TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Scroll  "),
        Span::styled("│", Style::default().fg(glances_colors::SEPARATOR)),
        Span::raw(" "),
        Span::styled("OK", Style::default().fg(glances_colors::OK)),
        Span::raw(":0-50% "),
        Span::styled("CAREFUL", Style::default().fg(glances_colors::CAREFUL)),
        Span::raw(":50-70% "),
        Span::styled("WARNING", Style::default().fg(glances_colors::WARNING)),
        Span::raw(":70-90% "),
        Span::styled("CRITICAL", Style::default().fg(glances_colors::CRITICAL)),
        Span::raw(":90%+"),
    ];

    let help = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    f.render_widget(help, area);
}

// Keep old functions for potential reuse, but mark as unused for now
#[allow(dead_code)]
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app.tabs.iter().map(|t| Line::from(*t)).collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Silicon Monitor"),
        )
        .select(app.selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Cyan)
                .fg(Color::Black),
        );

    f.render_widget(tabs, area);
}

#[allow(dead_code)]
fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    match app.selected_tab {
        0 => draw_overview(f, app, area),
        1 => draw_cpu(f, app, area),
        2 => draw_gpu(f, app, area),
        3 => draw_memory(f, app, area),
        4 => draw_system(f, app, area),
        5 => draw_agent(f, app, area),
        _ => {}
    }
}

#[allow(dead_code)]
fn draw_overview(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // CPU Overview
    let cpu_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("CPU - {}", app.cpu_info.name));

    let cpu_gauge = Gauge::default()
        .block(cpu_block)
        .gauge_style(
            Style::default()
                .fg(cpu_color(app.cpu_info.utilization))
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .percent(app.cpu_info.utilization as u16)
        .label(format!(
            "{:.1}% | {} cores | {:.0}°C",
            app.cpu_info.utilization,
            app.cpu_info.cores,
            app.cpu_info.temperature.unwrap_or(0.0)
        ));

    f.render_widget(cpu_gauge, chunks[0]);

    // Memory Overview
    let mem_percent = ((app.memory_info.used as f64 / app.memory_info.total as f64) * 100.0) as u16;
    let mem_block = Block::default().borders(Borders::ALL).title("Memory");

    let mem_gauge = Gauge::default()
        .block(mem_block)
        .gauge_style(
            Style::default()
                .fg(usage_color(mem_percent as f32))
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .percent(mem_percent)
        .label(format!(
            "{:.1} GB / {:.1} GB ({:.0}%)",
            app.memory_info.used as f64 / (1024.0 * 1024.0 * 1024.0),
            app.memory_info.total as f64 / (1024.0 * 1024.0 * 1024.0),
            mem_percent
        ));

    f.render_widget(mem_gauge, chunks[1]);

    // GPU Overview
    if !app.gpu_info.is_empty() {
        let gpu = &app.gpu_info[0];
        let gpu_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("GPU - {} ({})", gpu.name, gpu.vendor));

        let gpu_gauge = Gauge::default()
            .block(gpu_block)
            .gauge_style(
                Style::default()
                    .fg(usage_color(gpu.utilization))
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(gpu.utilization as u16)
            .label(format!(
                "{:.0}% | {:.0}°C | {:.0}W / {:.0}W",
                gpu.utilization,
                gpu.temperature.unwrap_or(0.0),
                gpu.power.unwrap_or(0.0),
                gpu.power_limit.unwrap_or(0.0)
            ));

        f.render_widget(gpu_gauge, chunks[2]);
    } else {
        let no_gpu = Paragraph::new("No GPUs detected")
            .block(Block::default().borders(Borders::ALL).title("GPU"))
            .alignment(Alignment::Center);
        f.render_widget(no_gpu, chunks[2]);
    }
}

#[allow(dead_code)]
fn draw_cpu(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // CPU Info
    let info_text = vec![
        Line::from(format!("Name: {}", app.cpu_info.name)),
        Line::from(format!(
            "Cores: {} ({} threads)",
            app.cpu_info.cores, app.cpu_info.threads
        )),
        Line::from(format!("Utilization: {:.1}%", app.cpu_info.utilization)),
        Line::from(format!(
            "Temperature: {:.1}°C",
            app.cpu_info.temperature.unwrap_or(0.0)
        )),
        Line::from(format!(
            "Frequency: {} MHz",
            app.cpu_info.frequency.unwrap_or(0)
        )),
    ];

    let info = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPU Information"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(info, chunks[0]);

    // CPU History Graph
    let sparkline_data: Vec<u64> = app.cpu_history.iter().copied().collect();
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPU History (60s)"),
        )
        .data(&sparkline_data)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(sparkline, chunks[1]);
}

#[allow(dead_code)]
fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    if app.gpu_info.is_empty() {
        let no_gpu = Paragraph::new("No GPUs detected")
            .block(Block::default().borders(Borders::ALL).title("GPU"))
            .alignment(Alignment::Center);
        f.render_widget(no_gpu, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let gpu = &app.gpu_info[0];

    // GPU Info
    let mem_percent = ((gpu.memory_used as f64 / gpu.memory_total as f64) * 100.0) as u16;
    let info_text = vec![
        Line::from(format!("Name: {}", gpu.name)),
        Line::from(format!("Vendor: {}", gpu.vendor)),
        Line::from(format!("Utilization: {:.0}%", gpu.utilization)),
        Line::from(format!(
            "Temperature: {:.0}°C",
            gpu.temperature.unwrap_or(0.0)
        )),
        Line::from(format!(
            "Power: {:.0}W / {:.0}W",
            gpu.power.unwrap_or(0.0),
            gpu.power_limit.unwrap_or(0.0)
        )),
        Line::from(format!(
            "Memory: {:.1} GB / {:.1} GB ({:.0}%)",
            gpu.memory_used as f64 / (1024.0 * 1024.0 * 1024.0),
            gpu.memory_total as f64 / (1024.0 * 1024.0 * 1024.0),
            mem_percent
        )),
        Line::from(format!(
            "Graphics Clock: {} MHz",
            gpu.clock_graphics
                .map(|c| c.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        )),
        Line::from(format!(
            "Memory Clock: {} MHz",
            gpu.clock_memory
                .map(|c| c.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        )),
    ];

    let info = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("GPU Information"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(info, chunks[0]);

    // GPU History
    if !app.gpu_histories.is_empty() {
        let sparkline_data: Vec<u64> = app.gpu_histories[0].iter().copied().collect();
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("GPU Utilization History (60s)"),
            )
            .data(&sparkline_data)
            .style(Style::default().fg(Color::Green));

        f.render_widget(sparkline, chunks[1]);
    }
}

#[allow(dead_code)]
fn draw_memory(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Memory Info
    let used_gb = app.memory_info.used as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = app.memory_info.total as f64 / (1024.0 * 1024.0 * 1024.0);
    let avail_gb = app.memory_info.available as f64 / (1024.0 * 1024.0 * 1024.0);
    let swap_used_gb = app.memory_info.swap_used as f64 / (1024.0 * 1024.0 * 1024.0);
    let swap_total_gb = app.memory_info.swap_total as f64 / (1024.0 * 1024.0 * 1024.0);

    let info_text = vec![
        Line::from(format!("Total: {:.2} GB", total_gb)),
        Line::from(format!("Used: {:.2} GB", used_gb)),
        Line::from(format!("Available: {:.2} GB", avail_gb)),
        Line::from(format!("Usage: {:.1}%", (used_gb / total_gb) * 100.0)),
        Line::from(""),
        Line::from(format!("Swap Total: {:.2} GB", swap_total_gb)),
        Line::from(format!("Swap Used: {:.2} GB", swap_used_gb)),
    ];

    let info = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Memory Information"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(info, chunks[0]);

    // Memory History
    let sparkline_data: Vec<u64> = app.memory_history.iter().copied().collect();
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Memory Usage History (60s)"),
        )
        .data(&sparkline_data)
        .style(Style::default().fg(Color::Magenta));

    f.render_widget(sparkline, chunks[1]);
}

#[allow(dead_code)]
fn draw_system(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // System Info
    let uptime_secs = app.system_info.uptime.as_secs();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let minutes = (uptime_secs % 3600) / 60;

    let mut info_lines = vec![
        Line::from(format!("Hostname: {}", app.system_info.hostname)),
        Line::from(format!("OS: {}", app.system_info.os)),
        Line::from(format!("Kernel: {}", app.system_info.kernel)),
        Line::from(format!("Uptime: {}d {}h {}m", days, hours, minutes)),
    ];

    if let Some(ref manufacturer) = app.system_info.manufacturer {
        info_lines.push(Line::from(format!("Manufacturer: {}", manufacturer)));
    }
    if let Some(ref model) = app.system_info.model {
        info_lines.push(Line::from(format!("Model: {}", model)));
    }

    let info = Paragraph::new(info_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("System Information"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(info, chunks[0]);

    // Disk Info
    let disk_items: Vec<ListItem> = app
        .disk_info
        .iter()
        .map(|disk| {
            let used_gb = disk.used as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = disk.total as f64 / (1024.0 * 1024.0 * 1024.0);
            let percent = (used_gb / total_gb) * 100.0;

            ListItem::new(format!(
                "{}: {:.1} GB / {:.1} GB ({:.0}%) - {}",
                disk.name, used_gb, total_gb, percent, disk.mount_point
            ))
        })
        .collect();

    let disks = List::new(disk_items)
        .block(Block::default().borders(Borders::ALL).title("Disks"))
        .style(Style::default().fg(Color::White));

    f.render_widget(disks, chunks[1]);
}

#[allow(dead_code)]
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    // Check if there's a status message to display
    if let Some(status_msg) = app.get_status_message() {
        let status = Paragraph::new(Line::from(vec![Span::styled(
            status_msg,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        f.render_widget(status, area);
    } else if app.agent_input_mode {
        // Show agent input mode
        let input_text = format!("> {}", app.agent_input);
        let input = Paragraph::new(Line::from(vec![
            Span::styled(
                "Agent Query: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&input_text),
            Span::styled(
                "█",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);
        f.render_widget(input, area);
    } else {
        let help_text = vec![
            Span::raw("Press "),
            Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to quit | "),
            Span::styled("</", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to switch tabs | "),
            Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to reset graphs | "),
            Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" for agent | "),
            Span::styled("F12", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to save config"),
        ];

        let help = Paragraph::new(Line::from(help_text))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(help, area);
    }
}

#[allow(dead_code)]
fn draw_agent(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Info/help
            Constraint::Min(0),    // Conversation history
        ])
        .split(area);

    // Agent info header
    let info_lines = if let Some(ref _agent) = app.agent {
        let cache_stats = app.agent_cache_stats().unwrap_or_default();
        vec![
            Line::from(vec![Span::styled(
                "[AI Agent Active]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(format!("Model: Medium (500M params) | {}", cache_stats)),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to ask a question | "),
                Span::styled("c", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to clear history"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![Span::styled(
                "❌ AI Agent Unavailable",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Agent failed to initialize. Check error logs."),
        ]
    };

    let info = Paragraph::new(info_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("AI Agent - Natural Language Queries"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(info, chunks[0]);

    // Conversation history
    if app.agent_history.is_empty() {
        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No queries yet. Try asking:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("  * What's my GPU temperature?"),
            Line::from("  * How much power am I using?"),
            Line::from("  * Show GPU utilization"),
            Line::from("  * Is my system healthy?"),
            Line::from("  * Compare GPU temperatures"),
            Line::from("  * What's my memory usage?"),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start asking questions"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Examples"))
            .alignment(Alignment::Left);

        f.render_widget(help, chunks[1]);
    } else {
        // Show conversation history (most recent first)
        let history_items: Vec<ListItem> = app
            .agent_history
            .iter()
            .rev() // Show newest first
            .enumerate()
            .flat_map(|(i, response)| {
                let time_str = format!(
                    "[{}ms{}]",
                    response.inference_time_ms,
                    if response.from_cache { ", cached" } else { "" }
                );

                vec![
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("Q{}: ", app.agent_history.len() - i),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&response.query),
                        Span::styled(
                            format!(" {}", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])),
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            "A:  ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&response.response, Style::default().fg(Color::White)),
                    ])),
                    ListItem::new(Line::from("")), // Spacer
                ]
            })
            .collect();

        let history = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Conversation History ({} queries)",
                app.agent_history.len()
            )))
            .style(Style::default().fg(Color::White));

        f.render_widget(history, chunks[1]);
    }
}

#[allow(dead_code)]
fn cpu_color(utilization: f32) -> Color {
    if utilization < 40.0 {
        Color::Green
    } else if utilization < 70.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Get color based on usage percentage (Glances-style thresholds)
fn usage_color(percent: f32) -> Color {
    threshold_color(percent)
}
