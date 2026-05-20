# ESP-IDF Driver Map

This map is the implementation contract for concrete ESP32-S3 firmware drivers.

Current Rust scaffold:

- `driver_manifest(config)` returns all Proto v1 peripheral bindings.
- Each binding declares the planned ESP-IDF driver family.
- Each binding declares the `indwell-hal` trait or event source it must implement.

Proto v1 bindings:

- WS2812 LED -> `indwell_hal::Led`
- Button GPIO -> button event source
- INMP441 I2S mic -> `indwell_hal::Microphone`
- MAX98357A I2S speaker -> `indwell_hal::Speaker`
- OV2640 camera -> `indwell_hal::Camera`
- SDMMC/FATFS microSD -> `indwell_hal::Storage`
- Wi-Fi STA, BLE provisioning, HTTP server, mDNS -> local control plane

Concrete driver code should live behind ESP-IDF feature gates so host tests remain fast and portable.
