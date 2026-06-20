use indwell_core::{DeviceState, Event, StateTransitionError};
use indwell_hal::{HalError, Led, LedPattern, Storage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Esp32S3FirmwareConfig {
    pub device_id: String,
    pub mdns_name: String,
    pub memory_mount: String,
    pub pwa_mount: String,
    pub button_gpio: u8,
    pub ws2812_gpio: u8,
    pub camera_model: String,
    pub i2s_mic_bclk_gpio: u8,
    pub i2s_mic_ws_gpio: u8,
    pub i2s_mic_data_gpio: u8,
    pub i2s_speaker_bclk_gpio: u8,
    pub i2s_speaker_ws_gpio: u8,
    pub i2s_speaker_data_gpio: u8,
    pub sdmmc_cmd_gpio: u8,
    pub sdmmc_clk_gpio: u8,
    pub sdmmc_d0_gpio: u8,
}

impl Default for Esp32S3FirmwareConfig {
    fn default() -> Self {
        Self {
            device_id: "indwell-proto-v1".to_string(),
            mdns_name: "indwell.local".to_string(),
            memory_mount: "/sdcard/indwell".to_string(),
            pwa_mount: "/sdcard/indwell/console".to_string(),
            button_gpio: 0,
            ws2812_gpio: 48,
            camera_model: "ov2640".to_string(),
            i2s_mic_bclk_gpio: 4,
            i2s_mic_ws_gpio: 5,
            i2s_mic_data_gpio: 6,
            i2s_speaker_bclk_gpio: 15,
            i2s_speaker_ws_gpio: 16,
            i2s_speaker_data_gpio: 17,
            sdmmc_cmd_gpio: 38,
            sdmmc_clk_gpio: 39,
            sdmmc_d0_gpio: 40,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverManifest {
    pub led: DriverBinding,
    pub button: DriverBinding,
    pub microphone: DriverBinding,
    pub speaker: DriverBinding,
    pub camera: DriverBinding,
    pub storage: DriverBinding,
    pub network: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverBinding {
    pub driver: String,
    pub pins: Vec<String>,
    pub implements: Vec<String>,
}

pub fn driver_manifest(config: &Esp32S3FirmwareConfig) -> DriverManifest {
    DriverManifest {
        led: DriverBinding {
            driver: "esp-idf-rmt-ws2812".to_string(),
            pins: vec![format!("gpio{}", config.ws2812_gpio)],
            implements: vec!["indwell_hal::Led".to_string()],
        },
        button: DriverBinding {
            driver: "esp-idf-gpio-input".to_string(),
            pins: vec![format!("gpio{}", config.button_gpio)],
            implements: vec!["button event source".to_string()],
        },
        microphone: DriverBinding {
            driver: "esp-idf-i2s-inmp441".to_string(),
            pins: vec![
                format!("bclk:gpio{}", config.i2s_mic_bclk_gpio),
                format!("ws:gpio{}", config.i2s_mic_ws_gpio),
                format!("data:gpio{}", config.i2s_mic_data_gpio),
            ],
            implements: vec!["indwell_hal::Microphone".to_string()],
        },
        speaker: DriverBinding {
            driver: "esp-idf-i2s-max98357a".to_string(),
            pins: vec![
                format!("bclk:gpio{}", config.i2s_speaker_bclk_gpio),
                format!("ws:gpio{}", config.i2s_speaker_ws_gpio),
                format!("data:gpio{}", config.i2s_speaker_data_gpio),
            ],
            implements: vec!["indwell_hal::Speaker".to_string()],
        },
        camera: DriverBinding {
            driver: format!("esp32-camera-{}", config.camera_model),
            pins: vec!["board-profile:proto-v1".to_string()],
            implements: vec!["indwell_hal::Camera".to_string()],
        },
        storage: DriverBinding {
            driver: "esp-idf-fatfs-sdmmc".to_string(),
            pins: vec![
                format!("cmd:gpio{}", config.sdmmc_cmd_gpio),
                format!("clk:gpio{}", config.sdmmc_clk_gpio),
                format!("d0:gpio{}", config.sdmmc_d0_gpio),
            ],
            implements: vec!["indwell_hal::Storage".to_string()],
        },
        network: vec![
            "esp-idf-wifi-sta".to_string(),
            "esp-idf-ble-provisioning".to_string(),
            "esp-idf-http-server".to_string(),
            "esp-idf-mdns".to_string(),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareBootPlan {
    pub initial_state: DeviceState,
    pub required_mounts: Vec<String>,
    pub network_services: Vec<String>,
    pub hardware_checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalServiceManifest {
    pub mdns_name: String,
    pub http_routes: Vec<LocalHttpRoute>,
    pub websocket_paths: Vec<String>,
    pub static_mounts: Vec<StaticMount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalHttpRoute {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub owner_auth_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticMount {
    pub url_prefix: String,
    pub storage_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoicePipelinePlan {
    pub wake_sources: Vec<String>,
    pub vad: String,
    pub capture_format: String,
    pub max_capture_ms: u32,
    pub auth_factors: Vec<String>,
    pub asr_runtime: String,
    pub tts_runtime: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeripheralInitPlan {
    pub order: Vec<String>,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EspIdfRuntimeScaffold {
    pub board: Esp32S3FirmwareConfig,
    pub tasks: Vec<FirmwareTask>,
    pub services: Vec<FirmwareService>,
    pub storage_layout: FirmwareStorageLayout,
    pub ota: FirmwareOtaPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareTask {
    pub name: String,
    pub responsibility: String,
    pub stack_bytes: u32,
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareService {
    pub name: String,
    pub driver: String,
    pub startup: StartupMode,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupMode {
    BootRequired,
    AfterProvisioning,
    LazyOnDemand,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareStorageLayout {
    pub nvs_namespace: String,
    pub memory_root: String,
    pub config_path: String,
    pub paired_devices_path: String,
    pub run_log_path: String,
    pub temp_media_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareOtaPlan {
    pub manifest_path: String,
    pub trusted_keys_path: String,
    pub download_path: String,
    pub active_slot_probe: String,
    pub apply_driver: String,
    pub rollback_policy: String,
}

pub fn boot_plan(config: &Esp32S3FirmwareConfig) -> FirmwareBootPlan {
    FirmwareBootPlan {
        initial_state: DeviceState::Booting,
        required_mounts: vec![config.memory_mount.clone(), config.pwa_mount.clone()],
        network_services: vec![
            "ble-provisioning".to_string(),
            "local-http".to_string(),
            "local-websocket".to_string(),
            "mdns".to_string(),
        ],
        hardware_checks: vec![
            format!("button:gpio{}", config.button_gpio),
            format!("ws2812:gpio{}", config.ws2812_gpio),
            "i2s-mic".to_string(),
            "i2s-speaker".to_string(),
            format!("camera:{}", config.camera_model),
            "microsd".to_string(),
        ],
    }
}

pub fn local_service_manifest(config: &Esp32S3FirmwareConfig) -> LocalServiceManifest {
    LocalServiceManifest {
        mdns_name: config.mdns_name.clone(),
        http_routes: vec![
            route("GET", "/health", "health", false),
            route("POST", "/v1/channel/input", "channel_input", true),
            route("GET", "/v1/channels/policies", "channel_policies", true),
            route("GET", "/v1/memory/export", "memory_export", true),
            route("POST", "/v1/memory/search", "memory_search", true),
            route("POST", "/v1/memory/metabolize", "memory_metabolize", true),
            route("POST", "/v1/provisioning", "provisioning_save", false),
            route("GET", "/v1/providers", "providers_get", true),
            route("PUT", "/v1/providers", "providers_save", true),
            route("PUT", "/v1/secrets/:key_ref", "secret_put", true),
            route("POST", "/v1/pairing/challenge", "pairing_challenge", false),
            route("POST", "/v1/pairing/complete", "pairing_complete", false),
            route(
                "POST",
                "/v1/auth/passphrase/challenge",
                "passphrase_challenge",
                true,
            ),
            route(
                "POST",
                "/v1/auth/passphrase/verify",
                "passphrase_verify",
                false,
            ),
            route("GET", "/v1/ota/manifest", "ota_manifest", true),
            route("POST", "/v1/ota/check", "ota_check", true),
            route("GET", "/v1/runs", "runs_list", true),
            route("GET", "/v1/runs/:id/entries", "runs_entries", true),
            route("GET", "/v1/tools", "tools_list", true),
            route("POST", "/v1/tools/:tool/check", "tool_check", true),
            route("POST", "/v1/tools/:tool/execute", "tool_execute", true),
        ],
        websocket_paths: vec!["/v1/ws/events".to_string(), "/v1/ws/audio".to_string()],
        static_mounts: vec![StaticMount {
            url_prefix: "/".to_string(),
            storage_path: config.pwa_mount.clone(),
        }],
    }
}

pub fn voice_pipeline_plan() -> VoicePipelinePlan {
    VoicePipelinePlan {
        wake_sources: vec![
            "button-to-talk".to_string(),
            "local-wake-word".to_string(),
            "mobile-trigger".to_string(),
        ],
        vad: "local-energy-vad-with-esp-sr-or-microwakeword-upgrade".to_string(),
        capture_format: "pcm16 mono 16khz short-window".to_string(),
        max_capture_ms: 15_000,
        auth_factors: vec![
            "paired-phone".to_string(),
            "dynamic-passphrase".to_string(),
            "optional-voiceprint-advisory-only".to_string(),
        ],
        asr_runtime: "provider-or-phone-local-asr".to_string(),
        tts_runtime: "provider-or-phone-local-tts".to_string(),
    }
}

pub fn peripheral_init_plan(config: &Esp32S3FirmwareConfig) -> PeripheralInitPlan {
    PeripheralInitPlan {
        order: vec![
            "nvs-key-store".to_string(),
            format!("fatfs-sdmmc:{}", config.memory_mount),
            "wifi-sta-or-provisioning-ap".to_string(),
            "mdns".to_string(),
            "http-server".to_string(),
            "ble-provisioning".to_string(),
            "ws2812-led".to_string(),
            "button-event-source".to_string(),
            "i2s-microphone".to_string(),
            "i2s-speaker".to_string(),
            format!("camera:{}", config.camera_model),
        ],
        fallbacks: vec![
            "if camera init fails, disable device.camera.capture and keep core runtime online"
                .to_string(),
            "if audio init fails, keep PWA and chat channels online".to_string(),
            "if microSD mount fails, enter provisioning/error and do not call providers"
                .to_string(),
        ],
    }
}

pub fn esp_idf_runtime_scaffold(config: &Esp32S3FirmwareConfig) -> EspIdfRuntimeScaffold {
    EspIdfRuntimeScaffold {
        board: config.clone(),
        tasks: vec![
            task(
                "event-loop",
                "dispatch core events and state transitions",
                8192,
                8,
            ),
            task(
                "net-control",
                "wifi station, mDNS, local HTTP and WebSocket services",
                12288,
                7,
            ),
            task(
                "audio-in",
                "button/wake/VAD and short PCM capture",
                12288,
                6,
            ),
            task(
                "audio-out",
                "I2S speaker playback for TTS or tones",
                8192,
                5,
            ),
            task(
                "vision",
                "lazy OV2640 capture and temporary JPEG retention",
                12288,
                4,
            ),
            task(
                "memory-flush",
                "append JSONL records and compact snapshots on microSD",
                8192,
                4,
            ),
            task(
                "ota-worker",
                "download, verify, write alternate slot, and confirm rollback",
                12288,
                3,
            ),
        ],
        services: vec![
            service("nvs", "esp-idf-nvs", StartupMode::BootRequired, []),
            service(
                "sdmmc-fatfs",
                "esp-idf-fatfs-sdmmc",
                StartupMode::BootRequired,
                ["nvs"],
            ),
            service(
                "wifi",
                "esp-idf-wifi-sta",
                StartupMode::AfterProvisioning,
                ["nvs"],
            ),
            service(
                "ble-provisioning",
                "esp-idf-ble-gatt",
                StartupMode::BootRequired,
                ["nvs"],
            ),
            service(
                "mdns",
                "esp-idf-mdns",
                StartupMode::AfterProvisioning,
                ["wifi"],
            ),
            service(
                "http-server",
                "esp-idf-http-server",
                StartupMode::AfterProvisioning,
                ["wifi", "sdmmc-fatfs"],
            ),
            service(
                "websocket",
                "esp-idf-http-server-ws",
                StartupMode::AfterProvisioning,
                ["http-server"],
            ),
            service(
                "ws2812",
                "esp-idf-rmt-ws2812",
                StartupMode::BootRequired,
                [],
            ),
            service(
                "button",
                "esp-idf-gpio-input",
                StartupMode::BootRequired,
                [],
            ),
            service("i2s-mic", "esp-idf-i2s-inmp441", StartupMode::Optional, []),
            service(
                "i2s-speaker",
                "esp-idf-i2s-max98357a",
                StartupMode::Optional,
                [],
            ),
            service(
                "camera",
                "esp32-camera-ov2640",
                StartupMode::LazyOnDemand,
                ["sdmmc-fatfs"],
            ),
            service(
                "ota",
                "esp-idf-ota",
                StartupMode::AfterProvisioning,
                ["wifi", "sdmmc-fatfs"],
            ),
        ],
        storage_layout: FirmwareStorageLayout {
            nvs_namespace: "indwell".to_string(),
            memory_root: format!("{}/memory", config.memory_mount),
            config_path: format!("{}/config/provisioning.json", config.memory_mount),
            paired_devices_path: format!("{}/pairing/devices.json", config.memory_mount),
            run_log_path: format!("{}/runs/runs.jsonl", config.memory_mount),
            temp_media_path: format!("{}/tmp/media", config.memory_mount),
        },
        ota: FirmwareOtaPlan {
            manifest_path: format!("{}/ota/manifest.json", config.memory_mount),
            trusted_keys_path: format!("{}/ota/trust_keys.json", config.memory_mount),
            download_path: format!("{}/ota/firmware.bin", config.memory_mount),
            active_slot_probe: "esp_ota_get_running_partition".to_string(),
            apply_driver: "esp_ota_begin/write/end + esp_ota_set_boot_partition".to_string(),
            rollback_policy: "mark valid after first boot health check; rollback on failed boot"
                .to_string(),
        },
    }
}

fn task(
    name: impl Into<String>,
    responsibility: impl Into<String>,
    stack_bytes: u32,
    priority: u8,
) -> FirmwareTask {
    FirmwareTask {
        name: name.into(),
        responsibility: responsibility.into(),
        stack_bytes,
        priority,
    }
}

fn service<const N: usize>(
    name: impl Into<String>,
    driver: impl Into<String>,
    startup: StartupMode,
    depends_on: [&str; N],
) -> FirmwareService {
    FirmwareService {
        name: name.into(),
        driver: driver.into(),
        startup,
        depends_on: depends_on.into_iter().map(ToString::to_string).collect(),
    }
}

fn route(
    method: impl Into<String>,
    path: impl Into<String>,
    handler: impl Into<String>,
    owner_auth_required: bool,
) -> LocalHttpRoute {
    LocalHttpRoute {
        method: method.into(),
        path: path.into(),
        handler: handler.into(),
        owner_auth_required,
    }
}

pub fn apply_state_led<L: Led>(led: &mut L, state: DeviceState) -> Result<(), FirmwareError> {
    let pattern = match state {
        DeviceState::Booting => LedPattern::Booting,
        DeviceState::Provisioning => LedPattern::Provisioning,
        DeviceState::Idle => LedPattern::Idle,
        DeviceState::Listening | DeviceState::Authenticating => LedPattern::Listening,
        DeviceState::Thinking | DeviceState::Observing | DeviceState::Updating => {
            LedPattern::Thinking
        }
        DeviceState::Speaking => LedPattern::Speaking,
        DeviceState::Error => LedPattern::Error,
        DeviceState::Sleep => LedPattern::Sleep,
    };
    led.set_pattern(pattern)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningConfig {
    pub ssid: String,
    pub provider_kind: String,
    pub provider_model: String,
    pub api_key_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareRuntime {
    pub config: Esp32S3FirmwareConfig,
    pub state: DeviceState,
    pub boot_plan: FirmwareBootPlan,
    pub last_provisioning: Option<ProvisioningConfig>,
}

impl FirmwareRuntime {
    pub fn new(config: Esp32S3FirmwareConfig) -> Self {
        let boot_plan = boot_plan(&config);
        Self {
            config,
            state: DeviceState::Booting,
            boot_plan,
            last_provisioning: None,
        }
    }

    pub fn transition_to<L: Led>(
        &mut self,
        led: &mut L,
        next: DeviceState,
    ) -> Result<(), FirmwareError> {
        self.state.transition_to(next)?;
        self.state = next;
        apply_state_led(led, next)
    }

    pub fn boot<L, S>(&mut self, led: &mut L, storage: &mut S) -> Result<(), FirmwareError>
    where
        L: Led,
        S: Storage,
    {
        apply_state_led(led, DeviceState::Booting)?;
        for mount in &self.boot_plan.required_mounts {
            storage.append(&format!("{mount}/.indwell_mount_check"), b"mounted=true\n")?;
        }
        self.transition_to(led, DeviceState::Provisioning)
    }

    pub fn complete_provisioning<L, S>(
        &mut self,
        led: &mut L,
        storage: &mut S,
        provisioning: ProvisioningConfig,
    ) -> Result<(), FirmwareError>
    where
        L: Led,
        S: Storage,
    {
        let path = format!("{}/config/provisioning.json", self.config.memory_mount);
        let bytes = serde_json::to_vec(&provisioning)?;
        storage.append(&path, &bytes)?;
        storage.append(&path, b"\n")?;
        self.last_provisioning = Some(provisioning);
        self.transition_to(led, DeviceState::Idle)
    }

    pub fn handle_event<L: Led>(
        &mut self,
        led: &mut L,
        event: &Event,
    ) -> Result<Option<DeviceState>, FirmwareError> {
        let Some(next) = self.state.next_for_event(event) else {
            return Ok(None);
        };
        self.transition_to(led, next)?;
        Ok(Some(next))
    }
}

#[derive(Debug, Error)]
pub enum FirmwareError {
    #[error("hardware error: {0}")]
    Hal(#[from] HalError),
    #[error("state transition denied: {0:?} -> {1:?}")]
    StateTransition(DeviceState, DeviceState),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<StateTransitionError> for FirmwareError {
    fn from(err: StateTransitionError) -> Self {
        Self::StateTransition(err.from, err.to)
    }
}

#[cfg(test)]
mod tests {
    use indwell_core::DeviceState;
    use indwell_hal::{LedPattern, MockHal};

    use super::{
        apply_state_led, boot_plan, driver_manifest, esp_idf_runtime_scaffold,
        local_service_manifest, peripheral_init_plan, voice_pipeline_plan, Esp32S3FirmwareConfig,
        FirmwareRuntime, ProvisioningConfig, StartupMode,
    };

    #[test]
    fn default_boot_plan_matches_proto_v1_services() {
        let plan = boot_plan(&Esp32S3FirmwareConfig::default());

        assert_eq!(plan.initial_state, DeviceState::Booting);
        assert!(plan
            .network_services
            .contains(&"ble-provisioning".to_string()));
        assert!(plan.hardware_checks.contains(&"microsd".to_string()));
    }

    #[test]
    fn driver_manifest_maps_proto_v1_peripherals_to_hal_traits() {
        let manifest = driver_manifest(&Esp32S3FirmwareConfig::default());

        assert_eq!(manifest.led.implements, ["indwell_hal::Led"]);
        assert!(manifest
            .microphone
            .implements
            .contains(&"indwell_hal::Microphone".to_string()));
        assert!(manifest
            .network
            .contains(&"esp-idf-http-server".to_string()));
    }

    #[test]
    fn local_service_manifest_exposes_proto_v1_control_plane() {
        let config = Esp32S3FirmwareConfig::default();
        let manifest = local_service_manifest(&config);

        assert_eq!(manifest.mdns_name, "indwell.local");
        assert!(manifest
            .http_routes
            .iter()
            .any(|route| route.method == "POST" && route.path == "/v1/channel/input"));
        assert!(manifest
            .http_routes
            .iter()
            .any(|route| route.path == "/v1/provisioning" && !route.owner_auth_required));
        assert_eq!(manifest.static_mounts[0].storage_path, config.pwa_mount);
    }

    #[test]
    fn voice_and_peripheral_plans_capture_proto_v1_deployment_order() {
        let voice = voice_pipeline_plan();
        assert!(voice.wake_sources.contains(&"button-to-talk".to_string()));
        assert_eq!(voice.max_capture_ms, 15_000);
        assert!(voice
            .auth_factors
            .contains(&"dynamic-passphrase".to_string()));

        let peripherals = peripheral_init_plan(&Esp32S3FirmwareConfig::default());
        assert_eq!(peripherals.order[0], "nvs-key-store");
        assert!(peripherals
            .order
            .iter()
            .any(|step| step.starts_with("fatfs-sdmmc:")));
        assert!(peripherals
            .fallbacks
            .iter()
            .any(|fallback| fallback.contains("camera init fails")));
    }

    #[test]
    fn esp_idf_runtime_scaffold_names_real_driver_boundaries() {
        let config = Esp32S3FirmwareConfig::default();
        let scaffold = esp_idf_runtime_scaffold(&config);

        assert!(scaffold
            .tasks
            .iter()
            .any(|task| task.name == "ota-worker" && task.stack_bytes >= 8192));
        assert!(scaffold.services.iter().any(|service| {
            service.name == "http-server"
                && service.startup == StartupMode::AfterProvisioning
                && service.depends_on.contains(&"wifi".to_string())
        }));
        assert!(scaffold.services.iter().any(
            |service| service.name == "camera" && service.startup == StartupMode::LazyOnDemand
        ));
        assert_eq!(
            scaffold.storage_layout.paired_devices_path,
            "/sdcard/indwell/pairing/devices.json"
        );
        assert_eq!(
            scaffold.ota.active_slot_probe,
            "esp_ota_get_running_partition"
        );
    }

    #[test]
    fn state_led_mapping_is_explicit() {
        let mut hal = MockHal::default();
        apply_state_led(&mut hal, DeviceState::Speaking).unwrap();

        assert_eq!(hal.last_led_pattern, Some(LedPattern::Speaking));
    }

    #[test]
    fn firmware_runtime_boots_into_provisioning_and_persists_mount_checks() {
        let mut runtime = FirmwareRuntime::new(Esp32S3FirmwareConfig::default());
        let mut led = MockHal::default();
        let mut storage = MockHal::default();

        runtime.boot(&mut led, &mut storage).unwrap();

        assert_eq!(runtime.state, DeviceState::Provisioning);
        assert_eq!(led.last_led_pattern, Some(LedPattern::Provisioning));
        assert!(storage
            .files
            .contains_key("/sdcard/indwell/.indwell_mount_check"));
    }

    #[test]
    fn firmware_runtime_completes_provisioning_to_idle() {
        let mut runtime = FirmwareRuntime::new(Esp32S3FirmwareConfig::default());
        let mut led = MockHal::default();
        let mut storage = MockHal::default();
        runtime.boot(&mut led, &mut storage).unwrap();

        runtime
            .complete_provisioning(
                &mut led,
                &mut storage,
                ProvisioningConfig {
                    ssid: "home".to_string(),
                    provider_kind: "mock".to_string(),
                    provider_model: "mock:phase0".to_string(),
                    api_key_ref: Some("key_llm_main".to_string()),
                },
            )
            .unwrap();

        assert_eq!(runtime.state, DeviceState::Idle);
        assert_eq!(led.last_led_pattern, Some(LedPattern::Idle));
        assert!(runtime.last_provisioning.is_some());
    }

    #[test]
    fn firmware_runtime_handles_events_through_state_machine() {
        let mut runtime = FirmwareRuntime::new(Esp32S3FirmwareConfig::default());
        runtime.state = DeviceState::Idle;
        let mut led = MockHal::default();

        let next = runtime
            .handle_event(
                &mut led,
                &indwell_core::Event::WakeWordDetected { score: 0.9 },
            )
            .unwrap();

        assert_eq!(next, Some(DeviceState::Listening));
        assert_eq!(runtime.state, DeviceState::Listening);
        assert_eq!(led.last_led_pattern, Some(LedPattern::Listening));
    }
}
