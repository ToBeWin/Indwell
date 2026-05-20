# ESP32-S3 Firmware Scaffold

The current firmware crate is a Rust scaffold that keeps ESP-IDF-specific code behind HAL traits.

Implemented:

- `Esp32S3FirmwareConfig` for Proto v1 board assumptions.
- `FirmwareBootPlan` for required mounts, local services, and hardware checks.
- `FirmwareRuntime` with explicit state.
- Boot flow:
  - starts in `Booting`
  - writes mount check files through `Storage`
  - transitions to `Provisioning`
- Provisioning flow:
  - persists provisioning JSON through `Storage`
  - transitions to `Idle`
- Event handling:
  - delegates to the core event/state reducer
  - updates LED patterns through `Led`
- OTA apply planning:
  - validates manifest shape
  - alternates OTA slots
  - requires confirmation
  - supports rollback state at the planning level
- ESP32 partition and sdkconfig defaults.

Next implementation layer:

- ESP-IDF concrete HAL implementations:
  - WS2812 LED
  - button GPIO
  - microSD FAT mount
  - local HTTP server
  - BLE provisioning endpoint
  - OV2640 still capture
  - I2S mic/speaker
- Firmware build target configuration with `esp-idf-sys` / `esp-idf-hal` / `esp-idf-svc`.

The firmware crate is intentionally testable on the host before concrete ESP-IDF drivers are attached.
