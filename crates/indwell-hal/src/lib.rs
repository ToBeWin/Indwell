use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedPattern {
    Booting,
    Provisioning,
    Idle,
    Listening,
    Thinking,
    Speaking,
    Error,
    Sleep,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub path: String,
    pub mime_type: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageFrame {
    pub path: String,
    pub width: u16,
    pub height: u16,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor: String,
    pub value: SensorValue,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorValue {
    TemperatureCelsius(f32),
    LightLux(f32),
    ButtonPressed { duration_ms: u32 },
    Motion { stable: bool },
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Error)]
pub enum HalError {
    #[error("device unavailable: {0}")]
    Unavailable(String),
    #[error("operation failed: {0}")]
    OperationFailed(String),
}

pub trait Led {
    fn set_pattern(&mut self, pattern: LedPattern) -> Result<(), HalError>;
}

pub trait Microphone {
    fn capture_short_audio(&mut self, max_duration_ms: u32) -> Result<AudioClip, HalError>;
}

pub trait Speaker {
    fn play(&mut self, clip: AudioClip) -> Result<(), HalError>;
}

pub trait Camera {
    fn capture_still(&mut self) -> Result<ImageFrame, HalError>;
}

pub trait SensorBus {
    fn read(&mut self, sensor: &str) -> Result<SensorReading, HalError>;
}

pub trait Storage {
    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), HalError>;
    fn read(&mut self, path: &str) -> Result<Vec<u8>, HalError>;
    fn remove(&mut self, path: &str) -> Result<(), HalError>;
}

#[derive(Debug, Default)]
pub struct MockHal {
    pub last_led_pattern: Option<LedPattern>,
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Led for MockHal {
    fn set_pattern(&mut self, pattern: LedPattern) -> Result<(), HalError> {
        self.last_led_pattern = Some(pattern);
        Ok(())
    }
}

impl Microphone for MockHal {
    fn capture_short_audio(&mut self, max_duration_ms: u32) -> Result<AudioClip, HalError> {
        Ok(AudioClip {
            path: "data/host-sim/audio/latest.wav".to_string(),
            mime_type: "audio/wav".to_string(),
            duration_ms: max_duration_ms.min(5000),
        })
    }
}

impl Speaker for MockHal {
    fn play(&mut self, _clip: AudioClip) -> Result<(), HalError> {
        Ok(())
    }
}

impl Camera for MockHal {
    fn capture_still(&mut self) -> Result<ImageFrame, HalError> {
        Ok(ImageFrame {
            path: "data/host-sim/camera/latest.jpg".to_string(),
            width: 640,
            height: 480,
            mime_type: "image/jpeg".to_string(),
        })
    }
}

impl SensorBus for MockHal {
    fn read(&mut self, sensor: &str) -> Result<SensorReading, HalError> {
        let value = match sensor {
            "temperature" => SensorValue::TemperatureCelsius(24.2),
            "light" | "ambient_light" => SensorValue::LightLux(320.0),
            "imu" => SensorValue::Motion { stable: true },
            _ => SensorValue::Text("simulated".to_string()),
        };
        Ok(SensorReading {
            sensor: sensor.to_string(),
            value,
            observed_at_ms: 0,
        })
    }
}

impl Storage for MockHal {
    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), HalError> {
        self.files
            .entry(path.to_string())
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }

    fn read(&mut self, path: &str) -> Result<Vec<u8>, HalError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| HalError::Unavailable(format!("file not found: {path}")))
    }

    fn remove(&mut self, path: &str) -> Result<(), HalError> {
        self.files.remove(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera, Led, LedPattern, MockHal, SensorBus, Storage};

    #[test]
    fn mock_hal_tracks_led_and_camera() {
        let mut hal = MockHal::default();
        hal.set_pattern(LedPattern::Thinking).unwrap();
        assert_eq!(hal.last_led_pattern, Some(LedPattern::Thinking));

        let frame = hal.capture_still().unwrap();
        assert_eq!(frame.mime_type, "image/jpeg");
    }

    #[test]
    fn mock_hal_reads_known_sensor() {
        let mut hal = MockHal::default();
        let reading = SensorBus::read(&mut hal, "temperature").unwrap();
        assert_eq!(reading.sensor, "temperature");
    }

    #[test]
    fn mock_hal_storage_appends_reads_and_removes() {
        let mut hal = MockHal::default();
        hal.append("/sdcard/indwell/log.jsonl", b"one").unwrap();
        hal.append("/sdcard/indwell/log.jsonl", b"two").unwrap();
        assert_eq!(
            Storage::read(&mut hal, "/sdcard/indwell/log.jsonl").unwrap(),
            b"onetwo"
        );

        hal.remove("/sdcard/indwell/log.jsonl").unwrap();
        assert!(Storage::read(&mut hal, "/sdcard/indwell/log.jsonl").is_err());
    }
}
