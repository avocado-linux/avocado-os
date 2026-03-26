# C GPIO Toggle Reference Runtime

A reference runtime that demonstrates how to cross-compile a C application using the meson build system in the Avocado SDK. The app uses libgpiod v2 to enumerate GPIO chips, request a line for output, and toggle it every second — logging the state to the journal.

This shows how to:
- Write a meson-based C project for Avocado OS
- Use the SDK's meson cross-compilation wrapper
- Link against system libraries (`libgpiod`) available in the Avocado package feed
- Install a compiled binary as a systemd service

## Prerequisites

- Docker Desktop running
- `avocado` CLI installed
- Raspberry Pi 5

## Build and Run

```bash
cd avocado-os-references/c-meson

# Install SDK, extensions, and runtime dependencies
avocado install -f

# Build (cross-compiles gpio-toggle in the SDK container)
avocado build

# Provision to SD card
avocado provision -f -r dev --profile sd
```

Insert the SD card into the Pi and power on.

## Observe

SSH into the Pi or connect via serial console. Login as `root` with an empty password.

```bash
# Check the service is running
systemctl status app

# Watch GPIO toggle logs
journalctl -u app -f
```

You'll see output like:

```
gpio-toggle starting
GPIO chips:
  gpiochip0 [pinctrl-bcm2712] (54 lines)
  gpiochip4 [pinctrl-rp1] (54 lines)
Opening /dev/gpiochip0, line 17
Toggling line 17 every 1s
[1711234567] line 17 = HIGH
[1711234568] line 17 = LOW
[1711234569] line 17 = HIGH
[1711234570] line 17 = LOW
```

To verify the GPIO is toggling, attach an LED (with a resistor) or a multimeter to GPIO 17.

## Project Structure

```
c-meson/
├── avocado.yaml                        # Runtime config (targets raspberrypi5)
├── app-compile.sh                      # SDK compile step: meson setup + ninja
├── app-install.sh                      # Install step: ninja install to sysroot
├── app-clean.sh                        # Clean build artifacts
└── app/
    ├── src/
    │   ├── meson.build                 # Meson build definition
    │   └── main.c                      # The GPIO toggle application
    └── overlay/                        # Files merged into the root filesystem
        └── usr/lib/systemd/system/
            └── app.service             # Systemd unit file
```

## How It Works

### Meson cross-compilation

The Avocado SDK provides a `meson-wrapper` that automatically injects `--cross-file` and `--native-file` arguments on every `meson setup` call. This means you don't need to manually configure cross-compilation — just call `meson setup build` and the wrapper handles the rest.

The cross file tells meson to use the SDK's cross-compiler (`aarch64-avocado-linux-gcc`) and target sysroot. The compile script generates these files if they don't already exist.

With the cross file in place, `meson setup build` + `ninja -C build` cross-compiles the C application for aarch64, and `DESTDIR=... ninja -C build install` installs the binary to the extension sysroot.

### meson.build

```meson
project('gpio-toggle', 'c',
  version : '0.1.0',
  default_options : ['warning_level=2', 'c_std=c11'])

gpiod_dep = dependency('libgpiod', version : '>=2.1')

executable('gpio-toggle',
  'main.c',
  dependencies : [gpiod_dep],
  install : true)
```

The `dependency('libgpiod')` call uses pkg-config (provided by the SDK) to find libgpiod headers and link flags in the target sysroot.

### libgpiod v2 API

The application uses the libgpiod v2 API (2.1.3):

| Operation | API |
|-----------|-----|
| Enumerate chips | Scan `/dev/gpiochip*` directory entries |
| Open a chip | `gpiod_chip_open("/dev/gpiochipN")` |
| Get chip info | `gpiod_chip_get_info()` → `gpiod_chip_info_get_name/label/num_lines()` |
| Configure line | `gpiod_line_settings_new()` + `gpiod_line_config_new()` + `gpiod_request_config_new()` |
| Request line | `gpiod_chip_request_lines(chip, req_cfg, line_cfg)` |
| Set value | `gpiod_line_request_set_value(request, offset, GPIOD_LINE_VALUE_ACTIVE)` |
| Release | `gpiod_line_request_release()`, `gpiod_chip_close()` |

### Compile/install pipeline

| Script | When it runs | What it does |
|--------|-------------|-------------|
| `app-compile.sh` | `avocado build` | Generates meson cross files if needed, runs `meson setup` + `ninja` |
| `app-install.sh` | `avocado build` | Runs `ninja install` with `DESTDIR` to copy binary to sysroot |
| `app-clean.sh` | `avocado clean` | Removes `app/src/build/` |

### Extension packages

| Package | Source | Purpose |
|---------|--------|---------|
| `libgpiod` | Avocado RPM repo | GPIO library runtime (installed on target) |
| `libgpiod-dev` | Avocado RPM repo | Headers + pkg-config (installed in SDK sysroot for compilation) |

## Customization

Edit `app/src/main.c` to change the default chip or line:

```c
#define DEFAULT_CHIP "/dev/gpiochip0"
#define DEFAULT_LINE 17
#define TOGGLE_INTERVAL_S 1
```

Or pass them as command-line arguments by editing `app.service`:

```ini
ExecStart=/usr/bin/gpio-toggle /dev/gpiochip4 22
```

After any change, rebuild and reprovision:

```bash
avocado build && avocado provision -f -r dev --profile sd
```

## Troubleshooting

**"gpiod_chip_open: No such file or directory"**: No GPIO chips found. Check available chips:
```bash
ls /dev/gpiochip*
```

**"gpiod_chip_request_lines: Device or resource busy"**: Another process has the line. Check:
```bash
gpioinfo
```

**Service keeps restarting**: The line may not exist on your chip. List available lines:
```bash
gpiodetect
gpioinfo gpiochip0
```

## What's Next

- Read GPIO inputs and log state changes (edge detection with `gpiod_line_settings_set_edge_detection`)
- Add I2C sensor reading alongside GPIO control
- Publish GPIO state over MQTT (combine with the python-basic pattern)
- Add a watchdog that monitors the toggle and raises an alert if it stops
