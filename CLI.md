# nvstats CLI - Complete Command Reference

A comprehensive command-line tool for NVIDIA GPU monitoring and Jetson device management.

## Installation

```bash
cargo build --release --features full
sudo cp target/release/nvstats /usr/local/bin/
```

## Command Structure

```
nvstats [OPTIONS] [COMMAND]
```

## Global Options

- `-i, --interval <SECONDS>` - Update interval in seconds (default: 1.0)
- `-f, --format <FORMAT>` - Output format: `text` or `json` (default: text)
- `--version` - Show version information
- `--help` - Show help information

## Monitoring Commands

### Interactive Mode (Default)

```bash
nvstats
```

Launches an interactive terminal UI showing real-time stats. Press 'q' to quit.

### Board Information

```bash
nvstats board
nvstats board --format json
```

Shows hardware information:
- Model name
- Module ID
- JetPack version (Jetson)
- L4T version (Jetson)
- Hardware revision
- Serial number

### GPU Monitoring

```bash
nvstats gpu
nvstats gpu --format json
```

Displays GPU statistics:
- GPU type (Integrated/Discrete)
- Frequency (current/min/max)
- Utilization percentage
- Memory usage
- Temperature
- Power consumption

### CPU Monitoring

```bash
nvstats cpu
nvstats cpu --format json
```

Shows CPU information:
- Total CPU usage
- Per-core usage
- Current frequencies
- CPU governor
- Online/offline cores

### Memory Monitoring

```bash
nvstats memory
nvstats memory --format json
```

Memory statistics:
- RAM (total/used/free/cached)
- SWAP (total/used/cached)
- EMC frequency (Jetson)
- IRAM (Jetson)

### Power Monitoring

```bash
nvstats power
nvstats power --format json
```

Power consumption:
- Total power (watts)
- Per-rail power (INA3221 sensors on Jetson)
- Voltage and current per rail
- Average power

### Temperature Monitoring

```bash
nvstats temperature
nvstats temperature --format json
```

Temperature readings:
- All thermal zones
- Maximum temperature
- Per-zone temperatures

### Process Monitoring

```bash
nvstats processes
nvstats processes --format json
```

GPU process information:
- PID
- User
- GPU assignment
- Process type
- CPU usage
- Memory usage
- GPU memory usage
- Process name

### Engine Monitoring

```bash
nvstats engines
nvstats engines --format json
```

Hardware accelerator status:
- APE (Audio Processing Engine)
- DLA (Deep Learning Accelerator)
- PVA (Programmable Vision Accelerator)
- VIC (Video Image Compositor)
- NVJPG (JPEG Encoder/Decoder)
- NVENC (Video Encoder)
- NVDEC (Video Decoder)
- SE (Security Engine)
- CVNAS
- MSENC
- OFA

### All Statistics

```bash
nvstats all
nvstats all --format json
```

## Advanced Utilities

### Jetson Clocks

Performance maximization tool.

#### Enable (Maximize Performance)

```bash
sudo nvstats jetson-clocks enable
```

Sets all frequencies to maximum:
- CPU: max frequency, all cores online
- GPU: max frequency
- EMC: max frequency
- All engines: max frequency

#### Disable (Restore Settings)

```bash
sudo nvstats jetson-clocks disable
```

Restores saved configuration or default settings.

#### Status

```bash
nvstats jetson-clocks status
```

Shows:
- Whether jetson_clocks is active
- Configured engines
- Current frequency settings

#### Store Configuration

```bash
sudo nvstats jetson-clocks store
```

Saves current configuration for later restoration.

### NVPModel

Power mode management.

#### Show Current Mode

```bash
nvstats nvpmodel show
```

Displays:
- Current power mode ID
- Current power mode name

#### List All Modes

```bash
nvstats nvpmodel list
```

Shows:
- All available power modes
- Mode IDs and names
- Default mode
- Current mode

#### Set Mode by ID

```bash
sudo nvstats nvpmodel set <MODE_ID>
sudo nvstats nvpmodel set <MODE_ID> --force
```

Changes power mode by ID (0, 1, 2, etc.).

Options:
- `--force, -f` - Skip confirmation prompt

#### Set Mode by Name

```bash
sudo nvstats nvpmodel set-name <MODE_NAME>
sudo nvstats nvpmodel set-name <MODE_NAME> --force
```

Changes power mode by name (MAXN, MODE_15W, MODE_10W, etc.).

Options:
- `--force, -f` - Skip confirmation prompt

### Swap Management

Swap file creation and management.

#### Status

```bash
nvstats swap status
```

Shows active swap files:
- Path
- Type (file/partition)
- Size
- Used space
- Priority

#### Create Swap

```bash
sudo nvstats swap create
sudo nvstats swap create --path <PATH> --size <GB> --auto
```

Creates a new swap file.

Options:
- `--path, -p <PATH>` - Swap file path (default: /swapfile)
- `--size, -s <GB>` - Size in GB (default: 8)
- `--auto, -a` - Enable on boot (add to /etc/fstab)

Examples:
```bash
# Create 8GB swap at /swapfile
sudo nvstats swap create

# Create 16GB swap with custom path
sudo nvstats swap create --path /mnt/swap16g --size 16

# Create and enable on boot
sudo nvstats swap create --size 12 --auto
```

#### Enable Swap

```bash
sudo nvstats swap enable <PATH>
```

Activates an existing swap file.

#### Disable Swap

```bash
sudo nvstats swap disable <PATH>
```

Temporarily deactivates swap file.

#### Remove Swap

```bash
sudo nvstats swap remove <PATH>
```

Disables and deletes swap file.

## Usage Examples

### Basic Monitoring

```bash
# Interactive monitoring
nvstats

# One-time snapshot
nvstats all

# JSON output for integration
nvstats all --format json | jq '.gpus'
```

### Performance Profiling

```bash
# Check current status
nvstats gpu
nvstats cpu
nvstats memory

# Enable maximum performance
sudo nvstats nvpmodel set-name MAXN --force
sudo nvstats jetson-clocks enable

# Verify
nvstats jetson-clocks status
nvstats nvpmodel show
```

### Power Management

```bash
# List available modes
nvstats nvpmodel list

# Switch to 15W mode
sudo nvstats nvpmodel set 1

# Disable jetson_clocks
sudo nvstats jetson-clocks disable
```

### Memory Management

```bash
# Check swap status
nvstats swap status

# Create swap if needed
sudo nvstats swap create --size 8 --auto

# Check memory after
nvstats memory
```

### Process Tracking

```bash
# Monitor GPU processes
nvstats processes

# Watch process changes
watch -n 1 'nvstats processes'
```

### System Setup

```bash
# First-time setup
sudo nvstats swap create --size 8 --auto
sudo nvstats nvpmodel set-name MAXN
nvstats board

# Start monitoring
nvstats
```

## Output Formats

### Text Format (Default)

Human-readable output with labels and formatting.

```bash
nvstats gpu
```

```
=== GPU Information ===
GPU 0 (Integrated):
  Frequency: 1300 MHz (204-1300 MHz)
  Load: 45%
  Memory: 1234 MB / 4096 MB
```

### JSON Format

Machine-readable JSON for scripting and integration.

```bash
nvstats gpu --format json
```

```json
{
  "gpu0": {
    "type": "Integrated",
    "freq": {
      "current": 1300,
      "min": 204,
      "max": 1300
    },
    "load": 45.0,
    "memory": {
      "used": 1234,
      "total": 4096
    }
  }
}
```

## Permissions

- **Read Operations**: No special permissions required
  - `nvstats board`, `nvstats gpu`, `nvstats cpu`, etc.
  
- **Write Operations**: Require `sudo`
  - `sudo nvstats jetson-clocks enable`
  - `sudo nvstats nvpmodel set <ID>`
  - `sudo nvstats swap create`

## Platform Availability

| Command       | Jetson | Linux Desktop | Windows |
| ------------- | ------ | ------------- | ------- |
| board         | ✅      | ✅             | 🚧       |
| gpu           | ✅      | ✅             | 🚧       |
| cpu           | ✅      | ✅             | 🚧       |
| memory        | ✅      | ✅             | 🚧       |
| power         | ✅      | ✅             | 🚧       |
| temperature   | ✅      | ✅             | 🚧       |
| processes     | ✅      | ❌             | ❌       |
| engines       | ✅      | ❌             | ❌       |
| jetson-clocks | ✅      | ❌             | ❌       |
| nvpmodel      | ✅      | ❌             | ❌       |
| swap          | ✅      | ✅             | ❌       |

## Exit Codes

- `0` - Success
- `1` - Error occurred
- `2` - Invalid arguments

## Environment Variables

- `RUST_LOG` - Set logging level (error, warn, info, debug, trace)

Example:
```bash
RUST_LOG=debug nvstats all
```

## See Also

- [README-RUST.md](README-RUST.md) - Main documentation
- [UTILITIES.md](UTILITIES.md) - Detailed utility documentation
- [MIGRATION.md](MIGRATION.md) - Migration guide from Python
- [BUILD.md](BUILD.md) - Build instructions
