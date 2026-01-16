//! CLI tool for Silicon Monitor (simon)

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "simon")]
#[command(about = "Silicon Monitor: Comprehensive hardware monitoring for CPUs, GPUs, NPUs, memory, I/O, and network silicon", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Update interval in seconds
    #[arg(short, long, default_value = "1.0", global = true)]
    interval: f64,

    /// Output format (json or text)
    #[arg(short, long, default_value = "text", global = true)]
    format: String,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Launch Terminal User Interface (TUI) - interactive dashboard
    Tui,
    /// Launch Graphical User Interface (GUI) - desktop application
    #[cfg(feature = "gui")]
    Gui,
    /// Show board information
    Board,
    /// Monitor GPU statistics
    Gpu,
    /// Monitor CPU statistics
    Cpu,
    /// Monitor memory statistics
    Memory,
    /// Monitor power statistics
    Power,
    /// Monitor temperature statistics
    Temperature,
    /// Monitor processes
    Processes,
    /// Monitor engines
    Engines,
    /// Show all statistics (default)
    All,
    /// Interactive real-time monitoring mode
    Monitor,
    /// Ask AI agent about system state
    Ai {
        /// Question to ask the AI agent (if not provided, enters interactive mode)
        query: Option<String>,
    },

    // Jetson utilities
    /// Jetson Clocks - Maximize performance
    JetsonClocks {
        #[command(subcommand)]
        action: JetsonClocksAction,
    },
    /// NVPModel - Power mode management
    Nvpmodel {
        #[command(subcommand)]
        action: NvpmodelAction,
    },
    /// Swap management
    Swap {
        #[command(subcommand)]
        action: SwapAction,
    },
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum JetsonClocksAction {
    /// Enable jetson_clocks (maximize performance)
    Enable,
    /// Disable jetson_clocks (restore settings)
    Disable,
    /// Show jetson_clocks status
    Status,
    /// Store current configuration
    Store,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum NvpmodelAction {
    /// Show current power mode
    Show,
    /// List all available power modes
    List,
    /// Set power mode by ID
    Set {
        /// Mode ID
        mode_id: u32,
        /// Force mode change
        #[arg(short, long)]
        force: bool,
    },
    /// Set power mode by name
    SetName {
        /// Mode name
        name: String,
        /// Force mode change
        #[arg(short, long)]
        force: bool,
    },
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum SwapAction {
    /// Show current swap status
    Status,
    /// Create a new swap file
    Create {
        /// Swap file path
        #[arg(short, long, default_value = "/swapfile")]
        path: PathBuf,
        /// Size in GB
        #[arg(short, long, default_value = "8")]
        size: u32,
        /// Enable on boot
        #[arg(short, long)]
        auto: bool,
    },
    /// Enable swap file
    Enable {
        /// Swap file path
        path: PathBuf,
    },
    /// Disable swap file
    Disable {
        /// Swap file path
        path: PathBuf,
    },
    /// Remove swap file
    Remove {
        /// Swap file path
        path: PathBuf,
    },
}

#[cfg(feature = "cli")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use simon::Simon;

    let cli = Cli::parse();

    env_logger::init();

    match &cli.command {
        // TUI command - Terminal User Interface
        Some(Commands::Tui) => {
            simon::tui::run()?;
        }

        // GUI command - Graphical User Interface
        #[cfg(feature = "gui")]
        Some(Commands::Gui) => {
            simon::gui::run().map_err(|e| format!("GUI error: {}", e))?;
        }

        // Monitoring commands
        Some(Commands::Board) => {
            let stats = Simon::with_interval(cli.interval)?;
            let board = stats.board_info();
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(board)?);
            } else {
                print_board_info(board);
            }
        }
        Some(Commands::Gpu) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.gpus)?);
            } else {
                print_gpu_info(&snapshot.gpus);
            }
        }
        Some(Commands::Cpu) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.cpu)?);
            } else {
                print_cpu_info(&snapshot.cpu);
            }
        }
        Some(Commands::Memory) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.memory)?);
            } else {
                print_memory_info(&snapshot.memory);
            }
        }
        Some(Commands::Power) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.power)?);
            } else {
                print_power_info(&snapshot.power);
            }
        }
        Some(Commands::Temperature) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.temperature)?);
            } else {
                print_temperature_info(&snapshot.temperature);
            }
        }
        Some(Commands::Processes) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.processes)?);
            } else {
                print_process_info(&snapshot.processes);
            }
        }
        Some(Commands::Engines) => {
            let mut stats = Simon::with_interval(cli.interval)?;
            let snapshot = stats.snapshot()?;
            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&snapshot.engines)?);
            } else {
                print_engine_info(&snapshot.engines);
            }
        }

        // AI Agent command
        Some(Commands::Ai { query }) => {
            handle_ai_query(query.as_deref())?;
        }

        // Jetson Clocks commands
        Some(Commands::JetsonClocks { action }) => {
            handle_jetson_clocks(action)?;
        }

        // NVPModel commands
        Some(Commands::Nvpmodel { action }) => {
            handle_nvpmodel(action)?;
        }

        // Swap commands
        Some(Commands::Swap { action }) => {
            handle_swap(action)?;
        }

        // Interactive monitoring mode
        Some(Commands::Monitor) => {
            let stats = Simon::with_interval(cli.interval)?;
            run_interactive_mode(stats)?;
        }

        _ => {
            // Launch TUI by default
            simon::tui::run()?;
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn print_board_info(board: &simon::core::platform_info::BoardInfo) {
    println!("=== Board Information ===");
    println!("Model: {}", board.hardware.model);
    println!(
        "System: {} {}",
        board.platform.system, board.platform.machine
    );
    if let Some(dist) = &board.platform.distribution {
        println!("Distribution: {}", dist);
    }
    println!("Kernel: {}", board.platform.release);

    if let Some(l4t) = &board.hardware.l4t {
        println!("L4T: {}", l4t);
    }
    if let Some(cuda) = &board.libraries.cuda {
        println!("CUDA: {}", cuda);
    }
}

#[cfg(feature = "cli")]
fn print_gpu_info(gpus: &std::collections::HashMap<String, simon::core::gpu::GpuInfo>) {
    println!("=== GPU Information ===");
    for (name, gpu) in gpus {
        println!("\n{} ({:?}):", name, gpu.gpu_type);
        println!("  Load: {:.1}%", gpu.status.load);
        println!(
            "  Frequency: {} MHz (min: {}, max: {})",
            gpu.frequency.current, gpu.frequency.min, gpu.frequency.max
        );
        println!("  Governor: {}", gpu.frequency.governor);

        if let Some(temp) = gpu.status.temperature {
            println!("  Temperature: {:.1}°C", temp);
        }
        if let Some(power) = gpu.status.power_draw {
            println!("  Power: {:.1}W", power);
        }
    }
}

#[cfg(feature = "cli")]
fn print_cpu_info(cpu: &simon::core::cpu::CpuStats) {
    println!("=== CPU Information ===");
    println!(
        "Total Cores: {} (Online: {})",
        cpu.core_count(),
        cpu.online_count()
    );
    println!(
        "Average Usage: {:.1}% (user: {:.1}%, system: {:.1}%, idle: {:.1}%)",
        100.0 - cpu.total.idle,
        cpu.total.user,
        cpu.total.system,
        cpu.total.idle
    );

    println!("\nPer-Core:");
    for core in &cpu.cores {
        if core.online {
            let usage = 100.0 - core.idle.unwrap_or(0.0);
            print!("  CPU{}: {:.1}%", core.id, usage);
            if let Some(freq) = &core.frequency {
                print!(" @ {} MHz", freq.current);
            }
            println!();
        } else {
            println!("  CPU{}: OFFLINE", core.id);
        }
    }
}

#[cfg(feature = "cli")]
fn print_memory_info(memory: &simon::core::memory::MemoryStats) {
    println!("=== Memory Information ===");
    println!(
        "RAM: {:.2} GB / {:.2} GB ({:.1}%)",
        memory.ram.used as f64 / 1024.0 / 1024.0,
        memory.ram.total as f64 / 1024.0 / 1024.0,
        memory.ram_usage_percent()
    );

    if memory.swap.total > 0 {
        println!(
            "SWAP: {:.2} GB / {:.2} GB ({:.1}%)",
            memory.swap.used as f64 / 1024.0 / 1024.0,
            memory.swap.total as f64 / 1024.0 / 1024.0,
            memory.swap_usage_percent()
        );
    }
}

#[cfg(feature = "cli")]
fn print_power_info(power: &simon::core::power::PowerStats) {
    println!("=== Power Information ===");
    println!("Total Power: {:.2}W", power.total_watts());

    println!("\nPower Rails:");
    for (name, rail) in &power.rails {
        if rail.online {
            println!(
                "  {}: {:.2}W ({:.1}V, {:.1}mA)",
                name,
                rail.power as f64 / 1000.0,
                rail.voltage as f64 / 1000.0,
                rail.current as f64
            );
        }
    }
}

#[cfg(feature = "cli")]
fn print_temperature_info(temp: &simon::core::temperature::TemperatureStats) {
    println!("=== Temperature Information ===");

    for (name, sensor) in &temp.sensors {
        if sensor.online {
            print!("  {}: {:.1}°C", name, sensor.temp);
            if let Some(max) = sensor.max {
                print!(" (max: {:.1}°C)", max);
            }
            println!();
        }
    }
}

#[cfg(feature = "cli")]
fn run_interactive_mode(mut stats: simon::Simon) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        event::{self, Event, KeyCode},
        terminal::{disable_raw_mode, enable_raw_mode},
    };
    use std::time::Duration;

    println!("Interactive monitoring mode - Press 'q' to quit");
    println!("Updating every {:.1}s\n", stats.interval().as_secs_f64());

    enable_raw_mode()?;

    loop {
        // Check for quit key
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        // Get snapshot
        let snapshot = stats.snapshot()?;

        // Print summary
        println!("=== Simon - NVIDIA GPU Monitoring ===");
        println!("Uptime: {:?}\n", snapshot.uptime);

        // GPU
        print_gpu_info(&snapshot.gpus);
        println!();

        // CPU
        print_cpu_info(&snapshot.cpu);
        println!();

        // Memory
        print_memory_info(&snapshot.memory);
        println!();

        // Temperature
        if let Some(max_temp) = snapshot.temperature.max_temp() {
            println!("Max Temperature: {:.1}°C", max_temp);
        }

        // Power
        println!("Total Power: {:.2}W", snapshot.power.total_watts());

        println!("\nPress 'q' to quit");

        std::thread::sleep(stats.interval());
    }

    disable_raw_mode()?;
    Ok(())
}

#[cfg(feature = "cli")]
fn handle_jetson_clocks(action: &JetsonClocksAction) -> Result<(), Box<dyn std::error::Error>> {
    use simon::utils::clocks;

    if !clocks::is_available() {
        eprintln!("jetson_clocks is not available on this system");
        std::process::exit(1);
    }

    match action {
        JetsonClocksAction::Enable => {
            println!("Enabling jetson_clocks (maximizing performance)...");
            clocks::enable()?;
            println!("jetson_clocks enabled successfully");
        }
        JetsonClocksAction::Disable => {
            println!("Disabling jetson_clocks (restoring settings)...");
            clocks::disable()?;
            println!("jetson_clocks disabled successfully");
        }
        JetsonClocksAction::Status => {
            let status = clocks::show()?;
            println!("=== Jetson Clocks Status ===");
            println!("Active: {}", if status.active { "YES" } else { "NO" });
            println!("\nConfigured Engines:");
            for engine in &status.engines {
                println!("  - {}", engine);
            }
        }
        JetsonClocksAction::Store => {
            println!("Storing current configuration...");
            clocks::store()?;
            println!("Configuration stored successfully");
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn handle_nvpmodel(action: &NvpmodelAction) -> Result<(), Box<dyn std::error::Error>> {
    use simon::utils::power_mode;

    if !power_mode::is_available() {
        eprintln!("nvpmodel is not available on this system");
        std::process::exit(1);
    }

    match action {
        NvpmodelAction::Show => {
            let mode = power_mode::query()?;
            println!("=== Current Power Mode ===");
            println!("ID: {}", mode.id);
            println!("Name: {}", mode.name);
        }
        NvpmodelAction::List => {
            let status = power_mode::list_modes()?;
            println!("=== Available Power Modes ===");
            println!("\nCurrent Mode:");
            println!(
                "  ID: {} - {} {}",
                status.current.id,
                status.current.name,
                if status.current.is_default {
                    "(default)"
                } else {
                    ""
                }
            );

            println!("\nAll Modes:");
            for mode in &status.modes {
                println!(
                    "  ID: {} - {} {}",
                    mode.id,
                    mode.name,
                    if mode.is_default { "(default)" } else { "" }
                );
            }

            println!("\nDefault Mode:");
            println!("  ID: {} - {}", status.default.id, status.default.name);
        }
        NvpmodelAction::Set { mode_id, force } => {
            println!("Setting power mode to ID {}...", mode_id);
            power_mode::set_mode(*mode_id, *force)?;
            println!("Power mode set successfully");

            // Show new mode
            let mode = power_mode::query()?;
            println!("New mode: {} ({})", mode.name, mode.id);
        }
        NvpmodelAction::SetName { name, force } => {
            println!("Setting power mode to '{}'...", name);
            power_mode::set_mode_by_name(name, *force)?;
            println!("Power mode set successfully");

            // Show new mode
            let mode = power_mode::query()?;
            println!("New mode: {} ({})", mode.name, mode.id);
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn handle_swap(action: &SwapAction) -> Result<(), Box<dyn std::error::Error>> {
    use simon::utils::swap;

    match action {
        SwapAction::Status => {
            let swaps = swap::status()?;

            if swaps.is_empty() {
                println!("No active swap");
            } else {
                println!("=== Active Swap ===");
                println!(
                    "{:<30} {:<10} {:<10} {:<10} {:<10}",
                    "NAME", "TYPE", "SIZE", "USED", "PRIO"
                );
                println!("{}", "-".repeat(80));

                for swap_info in swaps {
                    println!(
                        "{:<30} {:<10} {:<10} {:<10} {:<10}",
                        swap_info.path,
                        swap_info.swap_type,
                        format_size(swap_info.size_kb),
                        format_size(swap_info.used_kb),
                        swap_info.priority,
                    );
                }
            }
        }
        SwapAction::Create { path, size, auto } => {
            println!("This operation requires sudo privileges");
            swap::create(path, *size, *auto)?;
        }
        SwapAction::Enable { path } => {
            println!("Enabling swap: {}", path.display());
            swap::enable(path)?;
            println!("Swap enabled successfully");
        }
        SwapAction::Disable { path } => {
            println!("Disabling swap: {}", path.display());
            swap::disable(path)?;
            println!("Swap disabled successfully");
        }
        SwapAction::Remove { path } => {
            println!("Removing swap file: {}", path.display());
            println!("This operation requires sudo privileges");
            swap::remove(path)?;
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn format_size(kb: u64) -> String {
    if kb < 1024 {
        format!("{}K", kb)
    } else if kb < 1024 * 1024 {
        format!("{:.1}M", kb as f64 / 1024.0)
    } else {
        format!("{:.1}G", kb as f64 / 1024.0 / 1024.0)
    }
}

#[cfg(feature = "cli")]
fn print_process_info(processes: &simon::core::process::ProcessStats) {
    println!("=== Process Information ===");
    println!("Total Processes: {}", processes.process_count());
    println!(
        "Total GPU Memory: {:.2} MB\n",
        processes.total_gpu_memory_kb as f64 / 1024.0
    );

    if processes.process_count() > 0 {
        println!(
            "{:<8} {:<12} {:<8} {:<8} {:<8} {:<10} {:<10} {:<20}",
            "PID", "USER", "GPU", "TYPE", "STATE", "CPU%", "GPU MEM", "NAME"
        );
        println!("{}", "-".repeat(100));

        for proc in processes.sorted_by_gpu_memory().iter().take(10) {
            println!(
                "{:<8} {:<12} {:<8} {:<8} {:<8} {:<10.1} {:<10} {:<20}",
                proc.pid,
                &proc.user,
                &proc.gpu,
                &proc.process_type,
                proc.state,
                proc.cpu_percent,
                format_size(proc.gpu_memory_kb),
                &proc.name,
            );
        }
    }
}

#[cfg(feature = "cli")]
fn print_engine_info(engines: &simon::core::engine::EngineStats) {
    println!("=== Engine Information ===");
    println!("Engine Groups: {}", engines.group_count());
    println!("Total Engines: {}\n", engines.engine_count());

    if engines.group_count() > 0 {
        for (group_name, group_engines) in &engines.groups {
            println!("{}:", group_name);

            for (engine_name, info) in group_engines {
                let status = if info.online { "ONLINE " } else { "OFFLINE" };
                let freq_info = match (info.min, info.max) {
                    (Some(min), Some(max)) => {
                        format!("{} MHz (range: {}-{} MHz)", info.current, min, max)
                    }
                    _ => format!("{} MHz", info.current),
                };

                println!("  {:<15} {:<8} {}", engine_name, status, freq_info);
            }
            println!();
        }
    } else {
        println!("No engines available");
    }
}

#[cfg(feature = "cli")]
fn handle_ai_query(query: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use simon::agent::{Agent, AgentConfig};
    use simon::SiliconMonitor;
    use std::io::{self, Write};

    // Create monitor for system state
    let monitor = SiliconMonitor::new()?;

    // Auto-detect and configure best available backend
    let config = match AgentConfig::auto_detect() {
        Ok(cfg) => {
            // Successfully auto-detected a backend
            if let Some(ref backend) = cfg.backend {
                eprintln!("[*] Using backend: {}", backend.backend_type.display_name());
            }
            cfg
        }
        Err(e) => {
            // No backends available - return error instead of falling back
            eprintln!("[!] No AI backends available: {}", e);
            eprintln!("[!] To use AI features, install Ollama (https://ollama.com) or set an API key (OPENAI_API_KEY, GITHUB_TOKEN, etc.)");
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No AI backend configured",
            )));
        }
    }
    .with_caching(true)
    .with_cache_size(50)
    .with_timeout(Duration::from_secs(30)); // Longer timeout for remote backends

    let mut agent = Agent::new(config)?;

    if let Some(question) = query {
        // Single query mode
        println!("[AI Monitor]");
        println!("Question: {}\n", question);

        let response = agent.ask(question, &monitor)?;
        println!("{}", response.response);

        if response.from_cache {
            println!("\n[CACHE] (from cache, <1ms)");
        } else {
            println!("\n[TIME] ({}ms)", response.inference_time_ms);
        }
    } else {
        // Interactive mode
        println!("[AI Monitor - Interactive Mode]");
        println!("Ask questions about your system state. Type 'quit' or 'exit' to leave.\n");

        loop {
            print!("You: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                println!("Goodbye!");
                break;
            }

            match agent.ask(input, &monitor) {
                Ok(response) => {
                    println!("\n[Agent]: {}\n", response.response);
                    if response.from_cache {
                        println!("[CACHE] (from cache, <1ms)\n");
                    } else {
                        println!("[TIME] ({}ms)\n", response.inference_time_ms);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}\n", e);
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI features not enabled. Please compile with --features cli");
    std::process::exit(1);
}
