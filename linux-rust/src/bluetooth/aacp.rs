use crate::devices::airpods::AirPodsInformation;
use crate::devices::enums::{DeviceData, DeviceInformation, DeviceType};
use crate::utils::{get_devices_path, upsert_device};
use bluer::{
    Address, AddressType, Error, Result,
    l2cap::{SeqPacket, Socket, SocketAddr},
};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Instant, sleep};

const PSM: u16 = 0x1001;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const HEADER_BYTES: [u8; 4] = [0x04, 0x00, 0x04, 0x00];
const INFORMATION_PREFIX_LEN: usize = 7;
const INFORMATION_FIELD_COUNT: usize = 11;

fn parse_address(address: &str) -> Result<Address> {
    address.parse::<Address>().map_err(Error::from)
}

fn reversed_address_bytes(address: Address) -> [u8; 6] {
    let mut bytes: [u8; 6] = address.into();
    bytes.reverse();
    bytes
}

pub mod opcodes {
    pub const SET_FEATURE_FLAGS: u8 = 0x4D;
    pub const REQUEST_NOTIFICATIONS: u8 = 0x0F;
    pub const BATTERY_INFO: u8 = 0x04;
    pub const CONTROL_COMMAND: u8 = 0x09;
    pub const EAR_DETECTION: u8 = 0x06;
    pub const CONVERSATION_AWARENESS: u8 = 0x4B;
    pub const INFORMATION: u8 = 0x1D;
    pub const RENAME: u8 = 0x1A;
    pub const PROXIMITY_KEYS_REQ: u8 = 0x30;
    pub const PROXIMITY_KEYS_RSP: u8 = 0x31;
    pub const STEM_PRESS: u8 = 0x19;
    pub const EQ_DATA: u8 = 0x53;
    pub const CONNECTED_DEVICES: u8 = 0x2E;
    pub const AUDIO_SOURCE: u8 = 0x0E;
    pub const SMART_ROUTING: u8 = 0x10;
    pub const SMART_ROUTING_RESP: u8 = 0x11;
    pub const SEND_CONNECTED_MAC: u8 = 0x14;
    pub const HEADTRACKING: u8 = 0x17;
    pub const TIPI_3: u8 = 0x0C;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlCommandStatus {
    pub identifier: ControlCommandIdentifiers,
    pub value: Vec<u8>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlCommandIdentifiers {
    MicMode = 0x01,
    ButtonSendMode = 0x05,
    VoiceTrigger = 0x12,
    SingleClickMode = 0x14,
    DoubleClickMode = 0x15,
    ClickHoldMode = 0x16,
    DoubleClickInterval = 0x17,
    ClickHoldInterval = 0x18,
    ListeningModeConfigs = 0x1A,
    OneBudAncMode = 0x1B,
    CrownRotationDirection = 0x1C,
    ListeningMode = 0x0D,
    AutoAnswerMode = 0x1E,
    ChimeVolume = 0x1F,
    VolumeSwipeInterval = 0x23,
    CallManagementConfig = 0x24,
    VolumeSwipeMode = 0x25,
    AdaptiveVolumeConfig = 0x26,
    SoftwareMuteConfig = 0x27,
    ConversationDetectConfig = 0x28,
    Ssl = 0x29,
    HearingAid = 0x2C,
    AutoAncStrength = 0x2E,
    HpsGainSwipe = 0x2F,
    HrmState = 0x30,
    InCaseToneConfig = 0x31,
    SiriMultitoneConfig = 0x32,
    HearingAssistConfig = 0x33,
    AllowOffOption = 0x34,
    StemConfig = 0x39,
    SleepDetectionConfig = 0x35,
    AllowAutoConnect = 0x36,
    EarDetectionConfig = 0x0A,
    AutomaticConnectionConfig = 0x20,
    OwnsConnection = 0x06,
}

impl ControlCommandIdentifiers {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::MicMode),
            0x05 => Some(Self::ButtonSendMode),
            0x12 => Some(Self::VoiceTrigger),
            0x14 => Some(Self::SingleClickMode),
            0x15 => Some(Self::DoubleClickMode),
            0x16 => Some(Self::ClickHoldMode),
            0x17 => Some(Self::DoubleClickInterval),
            0x18 => Some(Self::ClickHoldInterval),
            0x1A => Some(Self::ListeningModeConfigs),
            0x1B => Some(Self::OneBudAncMode),
            0x1C => Some(Self::CrownRotationDirection),
            0x0D => Some(Self::ListeningMode),
            0x1E => Some(Self::AutoAnswerMode),
            0x1F => Some(Self::ChimeVolume),
            0x23 => Some(Self::VolumeSwipeInterval),
            0x24 => Some(Self::CallManagementConfig),
            0x25 => Some(Self::VolumeSwipeMode),
            0x26 => Some(Self::AdaptiveVolumeConfig),
            0x27 => Some(Self::SoftwareMuteConfig),
            0x28 => Some(Self::ConversationDetectConfig),
            0x29 => Some(Self::Ssl),
            0x2C => Some(Self::HearingAid),
            0x2E => Some(Self::AutoAncStrength),
            0x2F => Some(Self::HpsGainSwipe),
            0x30 => Some(Self::HrmState),
            0x31 => Some(Self::InCaseToneConfig),
            0x32 => Some(Self::SiriMultitoneConfig),
            0x33 => Some(Self::HearingAssistConfig),
            0x34 => Some(Self::AllowOffOption),
            0x39 => Some(Self::StemConfig),
            0x35 => Some(Self::SleepDetectionConfig),
            0x36 => Some(Self::AllowAutoConnect),
            0x0A => Some(Self::EarDetectionConfig),
            0x20 => Some(Self::AutomaticConnectionConfig),
            0x06 => Some(Self::OwnsConnection),
            _ => None,
        }
    }
}

impl std::fmt::Display for ControlCommandIdentifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ControlCommandIdentifiers::MicMode => "Mic Mode",
            ControlCommandIdentifiers::ButtonSendMode => "Button Send Mode",
            ControlCommandIdentifiers::VoiceTrigger => "Voice Trigger",
            ControlCommandIdentifiers::SingleClickMode => "Single Click Mode",
            ControlCommandIdentifiers::DoubleClickMode => "Double Click Mode",
            ControlCommandIdentifiers::ClickHoldMode => "Click Hold Mode",
            ControlCommandIdentifiers::DoubleClickInterval => "Double Click Interval",
            ControlCommandIdentifiers::ClickHoldInterval => "Click Hold Interval",
            ControlCommandIdentifiers::ListeningModeConfigs => "Listening Mode Configs",
            ControlCommandIdentifiers::OneBudAncMode => "One Bud ANC Mode",
            ControlCommandIdentifiers::CrownRotationDirection => "Crown Rotation Direction",
            ControlCommandIdentifiers::ListeningMode => "Listening Mode",
            ControlCommandIdentifiers::AutoAnswerMode => "Auto Answer Mode",
            ControlCommandIdentifiers::ChimeVolume => "Chime Volume",
            ControlCommandIdentifiers::VolumeSwipeInterval => "Volume Swipe Interval",
            ControlCommandIdentifiers::CallManagementConfig => "Call Management Config",
            ControlCommandIdentifiers::VolumeSwipeMode => "Volume Swipe Mode",
            ControlCommandIdentifiers::AdaptiveVolumeConfig => "Adaptive Volume Config",
            ControlCommandIdentifiers::SoftwareMuteConfig => "Software Mute Config",
            ControlCommandIdentifiers::ConversationDetectConfig => "Conversation Detect Config",
            ControlCommandIdentifiers::Ssl => "SSL",
            ControlCommandIdentifiers::HearingAid => "Hearing Aid",
            ControlCommandIdentifiers::AutoAncStrength => "Auto ANC Strength",
            ControlCommandIdentifiers::HpsGainSwipe => "HPS Gain Swipe",
            ControlCommandIdentifiers::HrmState => "HRM State",
            ControlCommandIdentifiers::InCaseToneConfig => "In Case Tone Config",
            ControlCommandIdentifiers::SiriMultitoneConfig => "Siri Multitone Config",
            ControlCommandIdentifiers::HearingAssistConfig => "Hearing Assist Config",
            ControlCommandIdentifiers::AllowOffOption => "Allow Off Option",
            ControlCommandIdentifiers::StemConfig => "Stem Config",
            ControlCommandIdentifiers::SleepDetectionConfig => "Sleep Detection Config",
            ControlCommandIdentifiers::AllowAutoConnect => "Allow Auto Connect",
            ControlCommandIdentifiers::EarDetectionConfig => "Ear Detection Config",
            ControlCommandIdentifiers::AutomaticConnectionConfig => "Automatic Connection Config",
            ControlCommandIdentifiers::OwnsConnection => "Owns Connection",
        };
        write!(f, "{}", name)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ProximityKeyType {
    Irk = 0x01,
    EncKey = 0x04,
}

impl ProximityKeyType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Irk),
            0x04 => Some(Self::EncKey),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemPressType {
    SinglePress = 0x05,
    DoublePress = 0x06,
    TriplePress = 0x07,
    LongPress = 0x08,
}

impl StemPressType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x05 => Some(Self::SinglePress),
            0x06 => Some(Self::DoublePress),
            0x07 => Some(Self::TriplePress),
            0x08 => Some(Self::LongPress),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemPressBudType {
    Left = 0x01,
    Right = 0x02,
}

impl StemPressBudType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Left),
            0x02 => Some(Self::Right),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceType {
    None = 0x00,
    Call = 0x01,
    Media = 0x02,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryComponent {
    Headphone = 1,
    Left = 4,
    Right = 2,
    Case = 8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging = 1,
    NotCharging = 2,
    Disconnected = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarDetectionStatus {
    InEar = 0x00,
    OutOfEar = 0x01,
    InCase = 0x02,
    Disconnected = 0x03,
}

impl AudioSourceType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::None),
            0x01 => Some(Self::Call),
            0x02 => Some(Self::Media),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSource {
    pub mac: String,
    pub r#type: AudioSourceType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryInfo {
    pub component: BatteryComponent,
    pub level: u8,
    pub status: BatteryStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDevice {
    pub mac: String,
    pub info1: u8,
    pub info2: u8,
    pub r#type: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AACPEvent {
    BatteryInfo(Vec<BatteryInfo>),
    ControlCommand(ControlCommandStatus),
    EarDetection(Vec<EarDetectionStatus>, Vec<EarDetectionStatus>),
    ConversationalAwareness(u8),
    ProximityKeys(Vec<(u8, Vec<u8>)>),
    AudioSource(AudioSource),
    ConnectedDevices(Vec<ConnectedDevice>, Vec<ConnectedDevice>),
    OwnershipToFalseRequest,
    StemPress(StemPressType, StemPressBudType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirPodsLEKeys {
    pub irk: String,
    pub enc_key: String,
}

fn parse_information_payload(payload: &[u8]) -> Option<AirPodsInformation> {
    if !payload.starts_with(&[opcodes::INFORMATION, 0x00]) {
        return None;
    }

    // Captures carry a two-byte opcode followed by a five-byte metadata preamble.
    let mut remaining = payload.get(INFORMATION_PREFIX_LEN..)?;
    let mut fields = Vec::with_capacity(INFORMATION_FIELD_COUNT);
    for _ in 0..INFORMATION_FIELD_COUNT {
        let terminator = remaining.iter().position(|byte| *byte == 0x00)?;
        let field = std::str::from_utf8(&remaining[..terminator]).ok()?;
        fields.push(field);
        remaining = &remaining[terminator + 1..];
    }

    if fields.iter().all(|field| field.is_empty()) {
        return None;
    }

    Some(AirPodsInformation {
        name: fields[0].to_string(),
        model_number: fields[1].to_string(),
        manufacturer: fields[2].to_string(),
        serial_number: fields[3].to_string(),
        version1: fields[4].to_string(),
        version2: fields[5].to_string(),
        hardware_revision: fields[6].to_string(),
        updater_identifier: fields[7].to_string(),
        left_serial_number: fields[8].to_string(),
        right_serial_number: fields[9].to_string(),
        version3: fields[10].to_string(),
        le_keys: AirPodsLEKeys {
            irk: String::new(),
            enc_key: String::new(),
        },
    })
}

pub struct AACPManagerState {
    pub sender: Option<mpsc::Sender<Vec<u8>>>,
    // Bumped whenever the active packet channel is installed or invalidated.
    pub(crate) connection_generation: u64,
    pub control_command_status_list: Vec<ControlCommandStatus>,
    pub control_command_subscribers:
        HashMap<ControlCommandIdentifiers, Vec<mpsc::UnboundedSender<Vec<u8>>>>,
    pub owns: bool,
    pub old_connected_devices: Vec<ConnectedDevice>,
    pub connected_devices: Vec<ConnectedDevice>,
    pub audio_source: Option<AudioSource>,
    pub battery_info: Vec<BatteryInfo>,
    pub conversational_awareness_status: u8,
    pub old_ear_detection_status: Vec<EarDetectionStatus>,
    pub ear_detection_status: Vec<EarDetectionStatus>,
    event_tx: Option<mpsc::UnboundedSender<AACPEvent>>,
    pub devices: HashMap<String, DeviceData>,
    pub airpods_mac: Option<Address>,
}

impl AACPManagerState {
    fn new() -> Self {
        let devices: HashMap<String, DeviceData> = std::fs::read_to_string(get_devices_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        AACPManagerState {
            sender: None,
            connection_generation: 0,
            control_command_status_list: Vec::new(),
            control_command_subscribers: HashMap::new(),
            owns: false,
            old_connected_devices: Vec::new(),
            connected_devices: Vec::new(),
            audio_source: None,
            battery_info: Vec::new(),
            conversational_awareness_status: 0,
            old_ear_detection_status: Vec::new(),
            ear_detection_status: Vec::new(),
            event_tx: None,
            devices,
            airpods_mac: None,
        }
    }
}

#[derive(Clone)]
pub struct AACPManager {
    pub state: Arc<Mutex<AACPManagerState>>,
    tasks: Arc<Mutex<JoinSet<()>>>,
}

impl AACPManager {
    pub fn new() -> Self {
        AACPManager {
            state: Arc::new(Mutex::new(AACPManagerState::new())),
            tasks: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    pub async fn connect(&mut self, addr: Address) {
        info!("AACPManager connecting to {} on PSM {:#06X}...", addr, PSM);
        let target_sa = SocketAddr::new(addr, AddressType::BrEdr, PSM);

        {
            let mut state = self.state.lock().await;
            state.airpods_mac = Some(addr);
        }

        let socket = match Socket::new_seq_packet() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create L2CAP socket: {}", e);
                return;
            }
        };

        let seq_packet =
            match tokio::time::timeout(CONNECT_TIMEOUT, socket.connect(target_sa)).await {
                Ok(Ok(s)) => Arc::new(s),
                Ok(Err(e)) => {
                    error!("L2CAP connect failed: {}", e);
                    return;
                }
                Err(_) => {
                    error!("L2CAP connect timed out");
                    return;
                }
            };

        // Wait for connection to be fully established
        let start = Instant::now();
        loop {
            match seq_packet.peer_addr() {
                Ok(peer) if peer.cid != 0 => break,
                Ok(_) => { /* still waiting */ }
                Err(e) => {
                    if e.raw_os_error() == Some(107) {
                        // ENOTCONN
                        error!("Peer has disconnected during connection setup.");
                        return;
                    }
                    error!("Error getting peer address: {}", e);
                }
            }
            if start.elapsed() >= CONNECT_TIMEOUT {
                error!("Timed out waiting for L2CAP connection to be fully established.");
                return;
            }
            sleep(POLL_INTERVAL).await;
        }

        info!("L2CAP connection established with {}", addr);

        let (tx, rx) = mpsc::channel(128);

        let connection_generation = self.install_connection(tx).await;

        let mut tasks = self.tasks.lock().await;
        tasks.spawn(recv_thread(
            self.clone(),
            seq_packet.clone(),
            connection_generation,
        ));
        tasks.spawn(send_thread(
            self.clone(),
            rx,
            seq_packet,
            connection_generation,
        ));
    }

    async fn send_packet(&self, data: &[u8]) -> Result<()> {
        self.send_packet_for_generation(data, None).await
    }

    async fn send_packet_for_generation(
        &self,
        data: &[u8],
        expected_generation: Option<u64>,
    ) -> Result<()> {
        let (sender, connection_generation) = {
            let state = self.state.lock().await;
            if expected_generation.is_some_and(|expected| {
                expected != state.connection_generation || state.sender.is_none()
            }) {
                return Err(Error::from(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "AACP connection generation changed",
                )));
            }
            (state.sender.clone(), state.connection_generation)
        };
        let Some(sender) = sender else {
            error!("Cannot send packet, sender is not available.");
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "L2CAP stream not connected",
            )));
        };

        let permit = match sender.clone().reserve_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                error!("Failed to reserve packet channel capacity: {}", e);
                self.clear_connection_if_current(connection_generation)
                    .await;
                return Err(Error::from(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "L2CAP send channel closed",
                )));
            }
        };

        let state = self.state.lock().await;
        let sender_is_current = state.connection_generation == connection_generation
            && expected_generation.is_none_or(|expected| expected == connection_generation)
            && state
                .sender
                .as_ref()
                .is_some_and(|current| current.same_channel(&sender));
        if !sender_is_current {
            error!("Cannot send packet, the L2CAP connection changed while waiting.");
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "L2CAP connection changed while sending",
            )));
        }

        permit.send(data.to_vec());
        Ok(())
    }

    async fn install_connection(&self, sender: mpsc::Sender<Vec<u8>>) -> u64 {
        let mut state = self.state.lock().await;
        state.connection_generation = state.connection_generation.wrapping_add(1);
        state.sender = Some(sender);
        Self::clear_connection_data(&mut state);
        state.connection_generation
    }

    async fn clear_connection_if_current(&self, connection_generation: u64) -> bool {
        let mut state = self.state.lock().await;
        if state.connection_generation != connection_generation {
            return false;
        }

        state.sender = None;
        Self::clear_connection_data(&mut state);
        state.connection_generation = state.connection_generation.wrapping_add(1);
        true
    }

    fn clear_connection_data(state: &mut AACPManagerState) {
        state.owns = false;
        state.old_connected_devices.clear();
        state.connected_devices.clear();
        state.audio_source = None;
        state.control_command_status_list.clear();
        state.old_ear_detection_status.clear();
        state.ear_detection_status.clear();
    }

    async fn send_data_packet(&self, data: &[u8]) -> Result<()> {
        self.send_data_packet_for_generation(data, None).await
    }

    async fn send_data_packet_for_generation(
        &self,
        data: &[u8],
        connection_generation: Option<u64>,
    ) -> Result<()> {
        let packet = [HEADER_BYTES.as_slice(), data].concat();
        self.send_packet_for_generation(&packet, connection_generation)
            .await
    }

    pub async fn set_event_channel(&self, tx: mpsc::UnboundedSender<AACPEvent>) {
        let mut state = self.state.lock().await;
        state.event_tx = Some(tx);
    }

    pub async fn subscribe_to_control_command(
        &self,
        identifier: ControlCommandIdentifiers,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let mut state = self.state.lock().await;
        state
            .control_command_subscribers
            .entry(identifier)
            .or_default()
            .push(tx);
        // send initial value if available
        if let Some(status) = state
            .control_command_status_list
            .iter()
            .find(|s| s.identifier == identifier)
        {
            let _ = state
                .control_command_subscribers
                .get(&identifier)
                .unwrap()
                .last()
                .unwrap()
                .send(status.value.clone());
        }
    }

    pub async fn receive_packet(&self, packet: &[u8]) {
        if !packet.starts_with(&HEADER_BYTES) {
            debug!(
                "Received packet does not start with expected header: {}",
                hex::encode(packet)
            );
            return;
        }
        if packet.len() < 5 {
            debug!("Received packet too short: {}", hex::encode(packet));
            return;
        }

        let opcode = packet[4];
        let payload = &packet[4..];

        match opcode {
            opcodes::BATTERY_INFO => {
                if payload.len() < 3 {
                    error!("Battery Info packet too short: {}", hex::encode(payload));
                    return;
                }
                let count = payload[2] as usize;
                if payload.len() < 3 + count * 5 {
                    error!(
                        "Battery Info packet length mismatch: {}",
                        hex::encode(payload)
                    );
                    return;
                }
                let mut batteries = Vec::with_capacity(count);
                for i in 0..count {
                    let base_index = 3 + i * 5;
                    batteries.push(BatteryInfo {
                        component: match payload[base_index] {
                            0x01 => BatteryComponent::Headphone,
                            0x02 => BatteryComponent::Right,
                            0x04 => BatteryComponent::Left,
                            0x08 => BatteryComponent::Case,
                            _ => {
                                error!("Unknown battery component: {:#04x}", payload[base_index]);
                                continue;
                            }
                        },
                        level: payload[base_index + 2],
                        status: match payload[base_index + 3] {
                            0x01 => BatteryStatus::Charging,
                            0x02 => BatteryStatus::NotCharging,
                            0x04 => BatteryStatus::Disconnected,
                            _ => {
                                error!("Unknown battery status: {:#04x}", payload[base_index + 3]);
                                continue;
                            }
                        },
                    });
                }
                let mut state = self.state.lock().await;
                state.battery_info = batteries.clone();
                if let Some(ref tx) = state.event_tx {
                    let _ = tx.send(AACPEvent::BatteryInfo(batteries));
                }
                info!("Received Battery Info: {:?}", state.battery_info);
            }
            opcodes::CONTROL_COMMAND => {
                if payload.len() < 7 {
                    error!("Control Command packet too short: {}", hex::encode(payload));
                    return;
                }
                let identifier_byte = payload[2];
                let value_bytes = &payload[3..7];

                let last_non_zero = value_bytes.iter().rposition(|&b| b != 0);
                let value = match last_non_zero {
                    Some(i) => value_bytes[..=i].to_vec(),
                    None => vec![0],
                };

                if let Some(identifier) = ControlCommandIdentifiers::from_u8(identifier_byte) {
                    let status = ControlCommandStatus {
                        identifier,
                        value: value.clone(),
                    };
                    let mut state = self.state.lock().await;
                    if let Some(existing) = state
                        .control_command_status_list
                        .iter_mut()
                        .find(|s| s.identifier == identifier)
                    {
                        existing.value = value.clone();
                    } else {
                        state.control_command_status_list.push(status.clone());
                    }
                    if identifier == ControlCommandIdentifiers::OwnsConnection {
                        state.owns = value_bytes[0] != 0;
                    }
                    if let Some(subscribers) = state.control_command_subscribers.get(&identifier) {
                        for sub in subscribers {
                            let _ = sub.send(value.clone());
                        }
                    }
                    if let Some(ref tx) = state.event_tx {
                        let _ = tx.send(AACPEvent::ControlCommand(status));
                    }
                    info!(
                        "Received Control Command: {:?}, value: {}",
                        identifier,
                        hex::encode(&value)
                    );
                } else {
                    error!(
                        "Unknown Control Command identifier: {:#04x}",
                        identifier_byte
                    );
                }
            }
            opcodes::EAR_DETECTION => {
                if payload.len() < 4 {
                    error!("Ear Detection packet too short: {}", hex::encode(payload));
                    return;
                }
                let primary_status = payload[2];
                let secondary_status = payload[3];
                let mut statuses = Vec::new();
                statuses.push(match primary_status {
                    0x00 => EarDetectionStatus::InEar,
                    0x01 => EarDetectionStatus::OutOfEar,
                    0x02 => EarDetectionStatus::InCase,
                    0x03 => EarDetectionStatus::Disconnected,
                    _ => {
                        error!("Unknown ear detection status: {:#04x}", primary_status);
                        EarDetectionStatus::OutOfEar
                    }
                });
                statuses.push(match secondary_status {
                    0x00 => EarDetectionStatus::InEar,
                    0x01 => EarDetectionStatus::OutOfEar,
                    0x02 => EarDetectionStatus::InCase,
                    0x03 => EarDetectionStatus::Disconnected,
                    _ => {
                        error!("Unknown ear detection status: {:#04x}", secondary_status);
                        EarDetectionStatus::OutOfEar
                    }
                });
                let mut state = self.state.lock().await;
                state.old_ear_detection_status = state.ear_detection_status.clone();
                state.ear_detection_status = statuses.clone();

                if let Some(ref tx) = state.event_tx {
                    debug!(
                        "Sending Ear Detection event: old: {:?}, new: {:?}",
                        state.old_ear_detection_status, statuses
                    );
                    let _ = tx.send(AACPEvent::EarDetection(
                        state.old_ear_detection_status.clone(),
                        statuses,
                    ));
                }
                info!(
                    "Received Ear Detection Status: {:?}",
                    state.ear_detection_status
                );
            }
            opcodes::CONVERSATION_AWARENESS => {
                if packet.len() == 10 {
                    let status = packet[9];
                    let mut state = self.state.lock().await;
                    state.conversational_awareness_status = status;
                    if let Some(ref tx) = state.event_tx {
                        let _ = tx.send(AACPEvent::ConversationalAwareness(status));
                    }
                    info!("Received Conversation Awareness: {}", status);
                } else {
                    info!(
                        "Received Conversation Awareness packet with unexpected length: {}",
                        packet.len()
                    );
                }
            }
            opcodes::INFORMATION => {
                let Some(info) = parse_information_payload(payload) else {
                    error!("Malformed Information packet: {}", hex::encode(payload));
                    return;
                };
                let mut state = self.state.lock().await;
                let persisted_device = if let Some(mac) = state.airpods_mac {
                    let key = mac.to_string();
                    state.devices.get_mut(&key).map(|device_data| {
                        device_data.name = info.name.clone();
                        device_data.information = Some(DeviceInformation::AirPods(info.clone()));
                        (key, device_data.clone())
                    })
                } else {
                    None
                };
                drop(state);
                if let Some((key, device)) = persisted_device
                    && let Err(error) = upsert_device(&get_devices_path(), &key, |_| device)
                {
                    error!("Failed to save AirPods information: {error}");
                }
                info!("Received Information: {:?}", info);
            }

            opcodes::PROXIMITY_KEYS_RSP => {
                if payload.len() < 4 {
                    error!(
                        "Proximity Keys Response packet too short: {}",
                        hex::encode(payload)
                    );
                    return;
                }
                let key_count = payload[2] as usize;
                debug!("Proximity Keys Response contains {} keys.", key_count);
                let mut offset = 3;
                let mut keys = Vec::new();
                for _ in 0..key_count {
                    if offset + 3 >= payload.len() {
                        error!(
                            "Proximity Keys Response packet too short while parsing keys: {}",
                            hex::encode(payload)
                        );
                        return;
                    }
                    let key_type = payload[offset];
                    let key_length = payload[offset + 2] as usize;
                    offset += 4;
                    if offset + key_length > payload.len() {
                        error!(
                            "Proximity Keys Response packet too short for key data: {}",
                            hex::encode(payload)
                        );
                        return;
                    }
                    let key_data = payload[offset..offset + key_length].to_vec();
                    keys.push((key_type, key_data));
                    offset += key_length;
                }
                info!(
                    "Received Proximity Keys Response: {:?}",
                    keys.iter()
                        .map(|(kt, kd)| (kt, hex::encode(kd)))
                        .collect::<Vec<_>>()
                );
                let mut state = self.state.lock().await;
                for (key_type, key_data) in &keys {
                    if let Some(kt) = ProximityKeyType::from_u8(*key_type)
                        && let Some(mac) = state.airpods_mac
                    {
                        let mac_str = mac.to_string();
                        let device_data =
                            state.devices.entry(mac_str.clone()).or_insert(DeviceData {
                                name: mac_str.clone(),
                                type_: DeviceType::AirPods,
                                information: None,
                            });
                        match kt {
                            ProximityKeyType::Irk => match device_data.information.as_mut() {
                                Some(DeviceInformation::AirPods(info)) => {
                                    info.le_keys.irk = hex::encode(key_data);
                                }
                                _ => {
                                    error!("Device information is not AirPods for adding LE IRK.");
                                }
                            },
                            ProximityKeyType::EncKey => match device_data.information.as_mut() {
                                Some(DeviceInformation::AirPods(info)) => {
                                    info.le_keys.enc_key = hex::encode(key_data);
                                }
                                _ => {
                                    error!(
                                        "Device information is not AirPods for adding LE encryption key."
                                    );
                                }
                            },
                        }
                    }
                }
                let persisted_device = if let Some(mac) = state.airpods_mac {
                    let key = mac.to_string();
                    state.devices.get(&key).cloned().map(|device| (key, device))
                } else {
                    None
                };
                drop(state);
                if let Some((key, device)) = persisted_device
                    && let Err(error) = upsert_device(&get_devices_path(), &key, |_| device)
                {
                    error!("Failed to save AirPods proximity keys: {error}");
                }
            }
            opcodes::STEM_PRESS => {
                if payload.len() < 4 {
                    error!("Stem Press packet too short: {}", hex::encode(payload));
                    return;
                }
                let press_type = StemPressType::from_u8(payload[2]);
                let bud_type = StemPressBudType::from_u8(payload[3]);
                if let (Some(press), Some(bud)) = (press_type, bud_type) {
                    info!("Received Stem Press: {:?} on {:?}", press, bud);
                    let state = self.state.lock().await;
                    if let Some(ref tx) = state.event_tx {
                        let _ = tx.send(AACPEvent::StemPress(press, bud));
                    }
                } else {
                    error!(
                        "Invalid Stem Press packet - type: {:?}, bud: {:?}",
                        press_type, bud_type
                    );
                }
            }
            opcodes::AUDIO_SOURCE => {
                if payload.len() < 9 {
                    error!("Audio Source packet too short: {}", hex::encode(payload));
                    return;
                }
                let mac = format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    payload[7], payload[6], payload[5], payload[4], payload[3], payload[2]
                );
                let typ = AudioSourceType::from_u8(payload[8]).unwrap_or(AudioSourceType::None);
                let audio_source = AudioSource { mac, r#type: typ };
                let mut state = self.state.lock().await;
                state.audio_source = Some(audio_source.clone());
                if let Some(ref tx) = state.event_tx {
                    let _ = tx.send(AACPEvent::AudioSource(audio_source));
                }
                info!("Received Audio Source: {:?}", state.audio_source);
            }
            opcodes::CONNECTED_DEVICES => {
                if payload.len() < 5 {
                    error!(
                        "Connected Devices packet too short: {}",
                        hex::encode(payload)
                    );
                    return;
                }
                let count = payload[4] as usize;
                if payload.len() < 5 + count * 8 {
                    error!(
                        "Connected Devices packet length mismatch: {}",
                        hex::encode(payload)
                    );
                    return;
                }
                let mut devices = Vec::with_capacity(count);
                for i in 0..count {
                    let base = 5 + i * 8;
                    let mac = format!(
                        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        payload[base],
                        payload[base + 1],
                        payload[base + 2],
                        payload[base + 3],
                        payload[base + 4],
                        payload[base + 5]
                    );
                    let info1 = payload[base + 6];
                    let info2 = payload[base + 7];
                    devices.push(ConnectedDevice {
                        mac,
                        info1,
                        info2,
                        r#type: None,
                    });
                }
                let mut state = self.state.lock().await;
                state.old_connected_devices = state.connected_devices.clone();
                state.connected_devices = devices.clone();
                if let Some(ref tx) = state.event_tx {
                    let _ = tx.send(AACPEvent::ConnectedDevices(
                        state.old_connected_devices.clone(),
                        devices,
                    ));
                }
                info!("Received Connected Devices: {:?}", state.connected_devices);
            }
            opcodes::SMART_ROUTING_RESP => {
                if payload.len() < 2 {
                    error!(
                        "Smart Routing Response packet too short: {}",
                        hex::encode(payload)
                    );
                    return;
                }
                let packet_string = String::from_utf8_lossy(&payload[2..]);
                info!("Received Smart Routing Response: {}", packet_string);
                if packet_string.contains("SetOwnershipToFalse") {
                    info!("Received OwnershipToFalse request");
                    if let Some(ref tx) = self.state.lock().await.event_tx {
                        let _ = tx.send(AACPEvent::OwnershipToFalseRequest);
                    }
                }
            }
            opcodes::EQ_DATA => {
                debug!("Received EQ Data");
            }
            _ => debug!("Received unknown packet with opcode {:#04x}", opcode),
        }
    }

    pub async fn send_notification_request(&self) -> Result<()> {
        let opcode = [opcodes::REQUEST_NOTIFICATIONS, 0x00];
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let packet = [opcode.as_slice(), data.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_set_feature_flags_packet(&self) -> Result<()> {
        let opcode = [opcodes::SET_FEATURE_FLAGS, 0x00];
        // let data = [0xD7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let data = [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // adaptive volume is actually useful, seeing if it works
        let packet = [opcode.as_slice(), data.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_handshake(&self) -> Result<()> {
        let packet = [
            0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        self.send_packet(&packet).await
    }

    pub async fn send_proximity_keys_request(
        &self,
        key_types: Vec<ProximityKeyType>,
    ) -> Result<()> {
        let opcode = [opcodes::PROXIMITY_KEYS_REQ, 0x00];
        let mut data = Vec::with_capacity(2);
        data.push(key_types.iter().fold(0u8, |acc, kt| acc | (*kt as u8)));
        data.push(0x00);
        let packet = [opcode.as_slice(), data.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_rename_packet(&self, name: &str) -> Result<()> {
        let name_bytes = name.as_bytes();
        let size = name_bytes.len();
        let mut packet = Vec::with_capacity(6 + size);
        packet.push(opcodes::RENAME);
        packet.push(0x00);
        packet.push(0x01);
        packet.push(size as u8);
        packet.push(0x00);
        packet.extend_from_slice(name_bytes);
        self.send_data_packet(&packet).await
    }

    pub async fn send_control_command(
        &self,
        identifier: ControlCommandIdentifiers,
        value: &[u8],
    ) -> Result<()> {
        self.send_control_command_for_generation_inner(identifier, value, None)
            .await
    }

    pub(crate) async fn send_control_command_for_generation(
        &self,
        identifier: ControlCommandIdentifiers,
        value: &[u8],
        connection_generation: u64,
    ) -> Result<()> {
        self.send_control_command_for_generation_inner(
            identifier,
            value,
            Some(connection_generation),
        )
        .await
    }

    async fn send_control_command_for_generation_inner(
        &self,
        identifier: ControlCommandIdentifiers,
        value: &[u8],
        connection_generation: Option<u64>,
    ) -> Result<()> {
        let opcode = [opcodes::CONTROL_COMMAND, 0x00];
        let mut data = vec![identifier as u8];
        for i in 0..4 {
            data.push(value.get(i).copied().unwrap_or(0));
        }
        let packet = [opcode.as_slice(), data.as_slice()].concat();
        self.send_data_packet_for_generation(&packet, connection_generation)
            .await
    }

    pub async fn send_media_information_new_device(
        &self,
        self_mac_address: &str,
        target_mac_address: &str,
    ) -> Result<()> {
        let self_address = parse_address(self_mac_address)?.to_string();
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(112);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));

        buffer.extend_from_slice(&[0x68, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE5, 0x4A]);
        buffer.extend_from_slice(b"playingApp");
        buffer.push(0x42);
        buffer.extend_from_slice(b"NA");
        buffer.push(0x52);
        buffer.extend_from_slice(b"hostStreamingState");
        buffer.push(0x42);
        buffer.extend_from_slice(b"NO");
        buffer.push(0x49);
        buffer.extend_from_slice(b"btAddress");
        buffer.push(0x51);
        buffer.extend_from_slice(self_address.as_bytes());
        buffer.push(0x46);
        buffer.extend_from_slice(b"btName");
        buffer.push(0x43);
        buffer.extend_from_slice(b"Mac");
        buffer.push(0x58);
        buffer.extend_from_slice(b"otherDevice");
        buffer.extend_from_slice(b"AudioCategory");
        buffer.extend_from_slice(&[0x30, 0x64]);

        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_hijack_request(&self, target_mac_address: &str) -> Result<()> {
        self.send_hijack_request_for_generation_inner(target_mac_address, None)
            .await
    }

    pub(crate) async fn send_hijack_request_for_generation(
        &self,
        target_mac_address: &str,
        connection_generation: u64,
    ) -> Result<()> {
        self.send_hijack_request_for_generation_inner(
            target_mac_address,
            Some(connection_generation),
        )
        .await
    }

    async fn send_hijack_request_for_generation_inner(
        &self,
        target_mac_address: &str,
        connection_generation: Option<u64>,
    ) -> Result<()> {
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(106);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));
        buffer.extend_from_slice(&[0x62, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE5]);
        buffer.push(0x4A);
        buffer.extend_from_slice(b"localscore");
        buffer.extend_from_slice(&[0x30, 0x64]);
        buffer.push(0x46);
        buffer.extend_from_slice(b"reason");
        buffer.push(0x48);
        buffer.extend_from_slice(b"Hijackv2");
        buffer.push(0x51);
        buffer.extend_from_slice(b"audioRoutingScore");
        buffer.extend_from_slice(&[0x31, 0x2D, 0x01, 0x5F]);
        buffer.extend_from_slice(b"audioRoutingSetOwnershipToFalse");
        buffer.push(0x01);
        buffer.push(0x4B);
        buffer.extend_from_slice(b"remotescore");
        buffer.push(0xA5);

        while buffer.len() < 106 {
            buffer.push(0x00);
        }

        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet_for_generation(&packet, connection_generation)
            .await
    }

    pub async fn send_media_information(
        &self,
        self_mac_address: &str,
        target_mac_address: &str,
        streaming_state: bool,
    ) -> Result<()> {
        self.send_media_information_for_generation_inner(
            self_mac_address,
            target_mac_address,
            streaming_state,
            None,
        )
        .await
    }

    pub(crate) async fn send_media_information_for_generation(
        &self,
        self_mac_address: &str,
        target_mac_address: &str,
        streaming_state: bool,
        connection_generation: u64,
    ) -> Result<()> {
        self.send_media_information_for_generation_inner(
            self_mac_address,
            target_mac_address,
            streaming_state,
            Some(connection_generation),
        )
        .await
    }

    async fn send_media_information_for_generation_inner(
        &self,
        self_mac_address: &str,
        target_mac_address: &str,
        streaming_state: bool,
        connection_generation: Option<u64>,
    ) -> Result<()> {
        let self_address = parse_address(self_mac_address)?.to_string();
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(138);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));
        buffer.extend_from_slice(&[0x82, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE5, 0x4A]);
        buffer.extend_from_slice(b"PlayingApp");
        buffer.push(0x56);
        buffer.extend_from_slice(b"com.google.ios.youtube");
        buffer.push(0x52);
        buffer.extend_from_slice(b"HostStreamingState");
        buffer.push(0x42);
        buffer.extend_from_slice(if streaming_state { b"YES" } else { b"NO" });
        buffer.push(0x49);
        buffer.extend_from_slice(b"btAddress");
        buffer.push(0x51);
        buffer.extend_from_slice(self_address.as_bytes());
        buffer.extend_from_slice(b"btName");
        buffer.push(0x43);
        buffer.extend_from_slice(b"Mac");
        buffer.push(0x58);
        buffer.extend_from_slice(b"otherDevice");
        buffer.extend_from_slice(b"AudioCategory");
        buffer.extend_from_slice(&[0x31, 0x2D, 0x01]);

        while buffer.len() < 138 {
            buffer.push(0x00);
        }
        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet_for_generation(&packet, connection_generation)
            .await
    }

    pub async fn send_smart_routing_show_ui(&self, target_mac_address: &str) -> Result<()> {
        self.send_smart_routing_show_ui_for_generation_inner(target_mac_address, None)
            .await
    }

    pub(crate) async fn send_smart_routing_show_ui_for_generation(
        &self,
        target_mac_address: &str,
        connection_generation: u64,
    ) -> Result<()> {
        self.send_smart_routing_show_ui_for_generation_inner(
            target_mac_address,
            Some(connection_generation),
        )
        .await
    }

    async fn send_smart_routing_show_ui_for_generation_inner(
        &self,
        target_mac_address: &str,
        connection_generation: Option<u64>,
    ) -> Result<()> {
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(134);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));
        buffer.extend_from_slice(&[0x7E, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE6, 0x5B]);
        buffer.extend_from_slice(b"SmartRoutingKeyShowNearbyUI");
        buffer.push(0x01);
        buffer.push(0x4A);
        buffer.extend_from_slice(b"localscore");
        buffer.extend_from_slice(&[0x31, 0x2D]);
        buffer.push(0x01);
        buffer.push(0x46);
        buffer.extend_from_slice(b"reasonHhijackv2");
        buffer.push(0x51);
        buffer.extend_from_slice(b"audioRoutingScore");
        buffer.push(0xA2);
        buffer.push(0x5F);
        buffer.extend_from_slice(b"audioRoutingSetOwnershipToFalse");
        buffer.push(0x01);
        buffer.push(0x4B);
        buffer.extend_from_slice(b"remotescore");
        buffer.push(0xA2);

        while buffer.len() < 134 {
            buffer.push(0x00);
        }

        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet_for_generation(&packet, connection_generation)
            .await
    }

    pub async fn send_hijack_reversed(&self, target_mac_address: &str) -> Result<()> {
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(97);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));
        buffer.extend_from_slice(&[0x59, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE3]);
        buffer.push(0x5F);
        buffer.extend_from_slice(b"audioRoutingSetOwnershipToFalse");
        buffer.push(0x01);
        buffer.push(0x59);
        buffer.extend_from_slice(b"audioRoutingShowReverseUI");
        buffer.push(0x01);
        buffer.push(0x46);
        buffer.extend_from_slice(b"reason");
        buffer.push(0x53);
        buffer.extend_from_slice(b"ReverseBannerTapped");

        while buffer.len() < 97 {
            buffer.push(0x00);
        }

        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_add_tipi_device(
        &self,
        self_mac_address: &str,
        target_mac_address: &str,
    ) -> Result<()> {
        let self_address = parse_address(self_mac_address)?.to_string();
        let target_address = parse_address(target_mac_address)?;
        let opcode = [opcodes::SMART_ROUTING, 0x00];
        let mut buffer = Vec::with_capacity(86);
        buffer.extend_from_slice(&reversed_address_bytes(target_address));
        buffer.extend_from_slice(&[0x4E, 0x00]);
        buffer.extend_from_slice(&[0x01, 0xE5]);
        buffer.extend_from_slice(&[0x48, 0x69]);
        buffer.extend_from_slice(b"idleTime");
        buffer.extend_from_slice(&[0x08, 0x47]);
        buffer.extend_from_slice(b"newTipi");
        buffer.extend_from_slice(&[0x01, 0x49]);
        buffer.extend_from_slice(b"btAddress");
        buffer.push(0x51);
        buffer.extend_from_slice(self_address.as_bytes());
        buffer.push(0x46);
        buffer.extend_from_slice(b"btName");
        buffer.push(0x43);
        buffer.extend_from_slice(b"Mac");
        buffer.push(0x50);
        buffer.extend_from_slice(b"nearbyAudioScore");
        buffer.push(0x0E);

        let packet = [opcode.as_slice(), buffer.as_slice()].concat();
        self.send_data_packet(&packet).await
    }

    pub async fn send_some_packet(&self) -> Result<()> {
        self.send_data_packet(&[0x29, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
            .await
    }
}

async fn recv_thread(manager: AACPManager, sp: Arc<SeqPacket>, connection_generation: u64) {
    let mut buf = vec![0u8; 1024];
    loop {
        match sp.recv(&mut buf).await {
            Ok(0) => {
                info!("Remote closed the connection.");
                break;
            }
            Ok(n) => {
                let data = &buf[..n];
                debug!("Received {} bytes: {}", n, hex::encode(data));
                manager.receive_packet(data).await;
            }
            Err(e) => {
                debug!("Read error: {}", e);
                break;
            }
        }
    }
    if manager
        .clear_connection_if_current(connection_generation)
        .await
    {
        info!("Cleared AACP state for disconnected connection");
    }
}

async fn send_thread(
    manager: AACPManager,
    mut rx: mpsc::Receiver<Vec<u8>>,
    sp: Arc<SeqPacket>,
    connection_generation: u64,
) {
    while let Some(data) = rx.recv().await {
        let mut attempts = 0;
        loop {
            match sp.send(&data).await {
                Ok(_) => {
                    debug!("Sent {} bytes: {}", data.len(), hex::encode(&data));
                    break;
                }
                Err(e) if e.raw_os_error() == Some(107) && attempts < 10 => {
                    attempts += 1;
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Failed to send data: {}", e);
                    manager
                        .clear_connection_if_current(connection_generation)
                        .await;
                    return;
                }
            }
        }
    }
    info!("Send thread finished.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocol_packet(opcode: u8, body: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(HEADER_BYTES.len() + 2 + body.len());
        packet.extend_from_slice(&HEADER_BYTES);
        packet.extend_from_slice(&[opcode, 0x00]);
        packet.extend_from_slice(body);
        packet
    }

    fn information_packet(fields: &[&[u8]]) -> Vec<u8> {
        let mut packet = protocol_packet(opcodes::INFORMATION, &[0x02, 0xD5, 0x00, 0x04, 0x00]);
        for field in fields {
            packet.extend_from_slice(field);
            packet.push(0x00);
        }
        packet
    }

    fn connected_devices_packet(prefix: [u8; 2], count: u8, records: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(3 + records.len());
        body.extend_from_slice(&prefix);
        body.push(count);
        body.extend_from_slice(records);
        protocol_packet(opcodes::CONNECTED_DEVICES, &body)
    }

    fn sample_information(name: &str) -> AirPodsInformation {
        AirPodsInformation {
            name: name.to_string(),
            model_number: "A3048".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            serial_number: "SERIAL".to_string(),
            version1: "1".to_string(),
            version2: "2".to_string(),
            hardware_revision: "3".to_string(),
            updater_identifier: "updater".to_string(),
            left_serial_number: "LEFT".to_string(),
            right_serial_number: "RIGHT".to_string(),
            version3: "4".to_string(),
            le_keys: AirPodsLEKeys {
                irk: "irk".to_string(),
                enc_key: "enc".to_string(),
            },
        }
    }

    async fn manager_with_packet_capture() -> (AACPManager, mpsc::Receiver<Vec<u8>>) {
        let manager = AACPManager::new();
        let (sender, receiver) = mpsc::channel(1);
        manager.state.lock().await.sender = Some(sender);
        (manager, receiver)
    }

    async fn assert_captured_packet(receiver: &mut mpsc::Receiver<Vec<u8>>, expected_hex: &str) {
        let actual = receiver
            .recv()
            .await
            .expect("builder did not send a packet");
        let expected = hex::decode(expected_hex).expect("captured packet must be valid hex");
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn ear_detection_requires_four_payload_bytes_and_accepts_exact_boundary() {
        let manager = AACPManager::new();
        manager.state.lock().await.ear_detection_status = vec![EarDetectionStatus::OutOfEar];

        let truncated = protocol_packet(opcodes::EAR_DETECTION, &[0x00]);
        manager.receive_packet(&truncated).await;

        assert_eq!(
            manager.state.lock().await.ear_detection_status,
            vec![EarDetectionStatus::OutOfEar]
        );

        let exact = protocol_packet(opcodes::EAR_DETECTION, &[0x00, 0x02]);
        manager.receive_packet(&exact).await;

        let state = manager.state.lock().await;
        assert_eq!(
            state.old_ear_detection_status,
            vec![EarDetectionStatus::OutOfEar]
        );
        assert_eq!(
            state.ear_detection_status,
            vec![EarDetectionStatus::InEar, EarDetectionStatus::InCase]
        );
    }

    #[test]
    fn captured_information_packet_preserves_all_known_fields() {
        let packet = hex::decode(concat!(
            "040004001d0002d5000400416972506f64732050726f004133303438004170706c6520",
            "496e632e0051584e524848595850360036312e313836383034303030323030303030302e",
            "323731330036312e313836383034303030323030303030302e3237313300312e302e3000",
            "636f6d2e6170706c652e6163636573736f72792e757064617465722e6170702e37310048",
            "59394c5432454632364a59004833504c5748444a32364b3000363335373533360089312a",
            "6567a5400f84a3ca234947efd40b90d78436ae5946748d70273e66066a25893000353339",
            "35303630363400"
        ))
        .unwrap();

        let info = parse_information_payload(&packet[HEADER_BYTES.len()..]).unwrap();

        assert_eq!(info.name, "AirPods Pro");
        assert_eq!(info.model_number, "A3048");
        assert_eq!(info.manufacturer, "Apple Inc.");
        assert_eq!(info.serial_number, "QXNRHHYXP6");
        assert_eq!(info.version1, "61.1868040002000000.2713");
        assert_eq!(info.version2, "61.1868040002000000.2713");
        assert_eq!(info.hardware_revision, "1.0.0");
        assert_eq!(
            info.updater_identifier,
            "com.apple.accessory.updater.app.71"
        );
        assert_eq!(info.left_serial_number, "HY9LT2EF26JY");
        assert_eq!(info.right_serial_number, "H3PLWHDJ26K0");
        assert_eq!(info.version3, "6357536");
    }

    #[test]
    fn empty_information_fields_do_not_shift_later_fields() {
        let fields: [&[u8]; INFORMATION_FIELD_COUNT] = [
            b"AirPods",
            b"",
            b"Apple Inc.",
            b"SERIAL",
            b"version-1",
            b"version-2",
            b"hardware",
            b"updater",
            b"left",
            b"right",
            b"version-3",
        ];
        let packet = information_packet(&fields);

        let info = parse_information_payload(&packet[HEADER_BYTES.len()..]).unwrap();

        assert_eq!(info.name, "AirPods");
        assert_eq!(info.model_number, "");
        assert_eq!(info.manufacturer, "Apple Inc.");
        assert_eq!(info.serial_number, "SERIAL");
    }

    #[tokio::test]
    async fn malformed_or_dummy_only_information_does_not_overwrite_state() {
        let manager = AACPManager::new();
        let address: Address = "00:11:22:33:44:55".parse().unwrap();
        {
            let mut state = manager.state.lock().await;
            state.airpods_mac = Some(address);
            state.devices.clear();
            state.devices.insert(
                address.to_string(),
                DeviceData {
                    name: "Existing name".to_string(),
                    type_: DeviceType::AirPods,
                    information: Some(DeviceInformation::AirPods(sample_information(
                        "Existing name",
                    ))),
                },
            );
        }

        let malformed_fields: [&[u8]; INFORMATION_FIELD_COUNT] = [
            b"Replacement",
            b"A3048",
            b"\xFF",
            b"SERIAL",
            b"version-1",
            b"version-2",
            b"hardware",
            b"updater",
            b"left",
            b"right",
            b"version-3",
        ];
        manager
            .receive_packet(&information_packet(&malformed_fields))
            .await;

        let dummy_fields: [&[u8]; INFORMATION_FIELD_COUNT] = [b""; INFORMATION_FIELD_COUNT];
        manager
            .receive_packet(&information_packet(&dummy_fields))
            .await;

        let state = manager.state.lock().await;
        let device = state.devices.get(&address.to_string()).unwrap();
        assert_eq!(device.name, "Existing name");
        match device.information.as_ref() {
            Some(DeviceInformation::AirPods(info)) => {
                assert_eq!(info.name, "Existing name");
                assert_eq!(info.le_keys.irk, "irk");
                assert_eq!(info.le_keys.enc_key, "enc");
            }
            _ => panic!("existing AirPods information was overwritten"),
        }
    }

    #[tokio::test]
    async fn connected_devices_accepts_exact_five_byte_prefix_for_zero_devices() {
        let manager = AACPManager::new();
        let packet = connected_devices_packet([0x00, 0x00], 0, &[]);

        manager.receive_packet(&packet).await;

        assert!(manager.state.lock().await.connected_devices.is_empty());
    }

    #[tokio::test]
    async fn connected_devices_reads_one_device_count_from_payload_byte_four() {
        let manager = AACPManager::new();
        let packet = connected_devices_packet(
            [0x02, 0xD5],
            1,
            &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0xA1, 0xB1],
        );

        manager.receive_packet(&packet).await;

        let state = manager.state.lock().await;
        assert_eq!(
            state.connected_devices,
            vec![ConnectedDevice {
                mac: "00:11:22:33:44:55".to_string(),
                info1: 0xA1,
                info2: 0xB1,
                r#type: None,
            }]
        );
    }

    #[tokio::test]
    async fn connected_devices_parses_two_complete_records() {
        let manager = AACPManager::new();
        let packet = connected_devices_packet(
            [0x00, 0x00],
            2,
            &[
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0xA1, 0xB1, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB,
                0xA2, 0xB2,
            ],
        );

        manager.receive_packet(&packet).await;

        let state = manager.state.lock().await;
        assert_eq!(state.connected_devices.len(), 2);
        assert_eq!(state.connected_devices[0].mac, "00:11:22:33:44:55");
        assert_eq!(state.connected_devices[1].mac, "66:77:88:99:AA:BB");
        assert_eq!(state.connected_devices[1].info1, 0xA2);
        assert_eq!(state.connected_devices[1].info2, 0xB2);
    }

    #[tokio::test]
    async fn truncated_connected_devices_packet_does_not_overwrite_state() {
        let manager = AACPManager::new();
        let existing = ConnectedDevice {
            mac: "AA:BB:CC:DD:EE:FF".to_string(),
            info1: 0x01,
            info2: 0x02,
            r#type: Some("Existing".to_string()),
        };
        manager.state.lock().await.connected_devices = vec![existing.clone()];
        let packet = connected_devices_packet(
            [0x00, 0x00],
            2,
            &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0xA1, 0xB1],
        );

        manager.receive_packet(&packet).await;

        assert_eq!(manager.state.lock().await.connected_devices, vec![existing]);
    }

    #[tokio::test]
    async fn smart_routing_response_rejects_short_payload_and_emits_ownership_request() {
        let manager = AACPManager::new();
        let (sender, mut events) = mpsc::unbounded_channel();
        manager.set_event_channel(sender).await;

        let mut truncated = HEADER_BYTES.to_vec();
        truncated.push(opcodes::SMART_ROUTING_RESP);
        manager.receive_packet(&truncated).await;
        assert!(events.try_recv().is_err());

        let request = protocol_packet(
            opcodes::SMART_ROUTING_RESP,
            b"audioRoutingSetOwnershipToFalse",
        );
        manager.receive_packet(&request).await;

        assert!(matches!(
            events.try_recv(),
            Ok(AACPEvent::OwnershipToFalseRequest)
        ));
    }

    fn assert_invalid_address(error: Error) {
        assert!(
            matches!(&error.kind, bluer::ErrorKind::InvalidAddress(_)),
            "expected invalid address error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn all_smart_routing_builders_reject_malformed_target_addresses() {
        let manager = AACPManager::new();
        let valid_mac = "00:11:22:33:44:55";
        let invalid_mac = "not-a-mac";
        let short_mac = "00:11:22:33:44";

        assert_invalid_address(
            manager
                .send_media_information_new_device(valid_mac, invalid_mac)
                .await
                .unwrap_err(),
        );
        assert_invalid_address(manager.send_hijack_request(short_mac).await.unwrap_err());
        assert_invalid_address(
            manager
                .send_media_information(valid_mac, invalid_mac, true)
                .await
                .unwrap_err(),
        );
        assert_invalid_address(
            manager
                .send_smart_routing_show_ui(short_mac)
                .await
                .unwrap_err(),
        );
        assert_invalid_address(manager.send_hijack_reversed(invalid_mac).await.unwrap_err());
        assert_invalid_address(
            manager
                .send_add_tipi_device(valid_mac, short_mac)
                .await
                .unwrap_err(),
        );
    }

    #[tokio::test]
    async fn builders_with_self_addresses_reject_malformed_self_addresses() {
        let manager = AACPManager::new();
        let valid_mac = "00:11:22:33:44:55";

        assert_invalid_address(
            manager
                .send_media_information_new_device("not-a-mac", valid_mac)
                .await
                .unwrap_err(),
        );
        assert_invalid_address(
            manager
                .send_media_information("00:11:22:33:44", valid_mac, true)
                .await
                .unwrap_err(),
        );
        assert_invalid_address(
            manager
                .send_add_tipi_device("not-a-mac", valid_mac)
                .await
                .unwrap_err(),
        );
    }

    #[tokio::test]
    async fn smart_routing_builders_match_captured_packets() {
        let self_mac = "66:77:88:99:aa:bb";
        let target_mac = "00:11:22:33:44:55";

        let (manager, mut receiver) = manager_with_packet_capture().await;
        manager
            .send_media_information_new_device(self_mac, target_mac)
            .await
            .unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "040004001000554433221100680001e54a706c6179696e67417070424e4152686f737453",
                "747265616d696e675374617465424e4f496274416464726573735136363a37373a38383a",
                "39393a41413a42424662744e616d65434d6163586f74686572446576696365417564696f",
                "43617465676f72793064"
            ),
        )
        .await;

        manager.send_hijack_request(target_mac).await.unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "040004001000554433221100620001e54a6c6f63616c73636f7265306446726561736f6e",
                "4848696a61636b763251617564696f526f7574696e6753636f7265312d015f617564696f",
                "526f7574696e675365744f776e657273686970546f46616c7365014b72656d6f74657363",
                "6f7265a5"
            ),
        )
        .await;

        manager
            .send_media_information(self_mac, target_mac, true)
            .await
            .unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "040004001000554433221100820001e54a506c6179696e6741707056636f6d2e676f6f67",
                "6c652e696f732e796f757475626552486f737453747265616d696e675374617465425945",
                "53496274416464726573735136363a37373a38383a39393a41413a424262744e616d6543",
                "4d6163586f74686572446576696365417564696f43617465676f7279312d010000000000"
            ),
        )
        .await;

        manager
            .send_smart_routing_show_ui(target_mac)
            .await
            .unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "0400040010005544332211007e0001e65b536d617274526f7574696e674b657953686f77",
                "4e65617262795549014a6c6f63616c73636f7265312d0146726561736f6e4868696a6163",
                "6b763251617564696f526f7574696e6753636f7265a25f617564696f526f7574696e6753",
                "65744f776e657273686970546f46616c7365014b72656d6f746573636f7265a2"
            ),
        )
        .await;

        manager.send_hijack_reversed(target_mac).await.unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "040004001000554433221100590001e35f617564696f526f7574696e675365744f776e65",
                "7273686970546f46616c73650159617564696f526f7574696e6753686f77526576657273",
                "6555490146726561736f6e535265766572736542616e6e6572546170706564"
            ),
        )
        .await;

        manager
            .send_add_tipi_device(self_mac, target_mac)
            .await
            .unwrap();
        assert_captured_packet(
            &mut receiver,
            concat!(
                "0400040010005544332211004e0001e5486969646c6554696d6508476e65775469706901",
                "496274416464726573735136363a37373a38383a39393a41413a42424662744e616d6543",
                "4d6163506e6561726279417564696f53636f72650e"
            ),
        )
        .await;
    }

    use std::task::Poll;
    use tokio::time::timeout;

    #[tokio::test]
    async fn pending_send_releases_state_lock() {
        let manager = AACPManager::new();
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send(vec![0xAA]).await.unwrap();
        manager.install_connection(sender).await;

        let send = manager.send_handshake();
        tokio::pin!(send);
        assert!(matches!(futures::poll!(send.as_mut()), Poll::Pending));

        let state = timeout(Duration::from_millis(250), manager.state.lock())
            .await
            .expect("AACP state lock stayed held while the bounded channel was full");
        drop(state);

        assert_eq!(receiver.recv().await, Some(vec![0xAA]));
        send.await.unwrap();
        assert!(receiver.recv().await.is_some());
    }

    #[tokio::test]
    async fn disconnect_invalidates_pending_send_and_connection_state() {
        let manager = AACPManager::new();
        let (sender, receiver) = mpsc::channel(1);
        sender.send(vec![0xAA]).await.unwrap();
        let generation = manager.install_connection(sender).await;
        {
            let mut state = manager.state.lock().await;
            state.owns = true;
            state.connected_devices.push(ConnectedDevice {
                mac: "11:22:33:44:55:66".into(),
                info1: 0,
                info2: 0,
                r#type: None,
            });
            state.ear_detection_status = vec![EarDetectionStatus::InEar];
        }

        let send = manager.send_handshake();
        tokio::pin!(send);
        assert!(matches!(futures::poll!(send.as_mut()), Poll::Pending));

        assert!(manager.clear_connection_if_current(generation).await);
        drop(receiver);
        assert!(
            timeout(Duration::from_millis(250), send)
                .await
                .expect("pending send did not finish after disconnect")
                .is_err()
        );

        let state = manager.state.lock().await;
        assert!(state.sender.is_none());
        assert!(!state.owns);
        assert!(state.connected_devices.is_empty());
        assert!(state.ear_detection_status.is_empty());
        assert_ne!(state.connection_generation, generation);
    }

    #[tokio::test]
    async fn closed_send_channel_returns_error_and_invalidates_generation() {
        let manager = AACPManager::new();
        let (sender, receiver) = mpsc::channel(1);
        let generation = manager.install_connection(sender).await;
        drop(receiver);

        assert!(manager.send_handshake().await.is_err());

        let state = manager.state.lock().await;
        assert!(state.sender.is_none());
        assert_ne!(state.connection_generation, generation);
    }

    #[tokio::test]
    async fn stale_disconnect_does_not_clear_replacement_connection() {
        let manager = AACPManager::new();
        let (old_sender, _old_receiver) = mpsc::channel(1);
        let old_generation = manager.install_connection(old_sender).await;
        let (new_sender, _new_receiver) = mpsc::channel(1);
        let expected_sender = new_sender.clone();
        let new_generation = manager.install_connection(new_sender).await;

        assert!(!manager.clear_connection_if_current(old_generation).await);

        let state = manager.state.lock().await;
        assert_eq!(state.connection_generation, new_generation);
        assert!(
            state
                .sender
                .as_ref()
                .is_some_and(|sender| sender.same_channel(&expected_sender))
        );
    }
}
