# Indwell Proto v1 Hardware

Target board class: ESP32-S3 N16R8/N8R8 with PSRAM.

Baseline modules:

- OV2640 camera for on-demand still capture.
- INMP441 I2S microphone for short capture / VAD path.
- MAX98357A I2S amplifier and 8 ohm speaker.
- microSD card for memory drawers, snapshots, logs, and temporary media.
- WS2812 status LED.
- Physical button for pairing, reset, wake, and high-risk confirmation.
- USB-C power and development serial.

The Rust boundary is `indwell-hal`; ESP-IDF-specific drivers should implement those traits while keeping Agent Kernel, memory, policy, and provider behavior in Rust.
