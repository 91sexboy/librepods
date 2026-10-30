use crate::bluetooth::aacp::{AACPManager, ControlCommandIdentifiers, EarDetectionStatus};
use dbus::arg::RefArg;
use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet};
use libpulse_binding::def::Retval;
use libpulse_binding::mainloop::standard::Mainloop;
use libpulse_binding::operation::State as OperationState;
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct OwnedCardProfileInfo {
    name: Option<String>,
}

#[derive(Clone, Debug)]
struct OwnedCardInfo {
    index: u32,
    proplist: Proplist,
    profiles: Vec<OwnedCardProfileInfo>,
}

#[derive(Clone, Debug)]
struct OwnedSinkInfo {
    name: Option<String>,
    proplist: Proplist,
    volume: ChannelVolumes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileOperationError {
    NoConnectedDevice,
    AudioDeviceNotFound,
    ConnectedDeviceChanged,
    WirePlumberRestartFailed,
    A2dpProfileUnavailable,
    NoPreferredA2dpProfile,
    ProfileWorkerStopped,
    ProfileSwitchFailed(String),
}

pub type ProfileOperationResult = std::result::Result<(), ProfileOperationError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoutingPacketKind {
    MediaInformation,
    ShowUi,
    HijackRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaybackTakeoverFailure {
    ProfileActivation(ProfileOperationError),
    OwnershipSend(String),
    RoutingSend {
        device_mac: String,
        packet: RoutingPacketKind,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaybackTakeoverOutcome {
    Completed,
    SkippedNotInEar,
    AbortedDisconnected,
    AbortedConnectionChanged,
    AbortedEarStateChanged,
    Failed(PlaybackTakeoverFailure),
}

fn normalized_profile_name(profile: &str) -> String {
    profile.replace('_', "-")
}

fn is_a2dp_sink_profile(profile: &str) -> bool {
    let profile = normalized_profile_name(profile);
    profile == "a2dp-sink"
        || profile
            .strip_prefix("a2dp-sink-")
            .is_some_and(|codec| !codec.is_empty())
}

fn preferred_a2dp_profile<'a>(profiles: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let profiles: Vec<_> = profiles.into_iter().collect();

    ["a2dp-sink-sbc-xq", "a2dp-sink-sbc", "a2dp-sink"]
        .into_iter()
        .find_map(|preferred| {
            profiles
                .iter()
                .copied()
                .find(|profile| normalized_profile_name(profile) == preferred)
        })
        .or_else(|| {
            profiles
                .iter()
                .copied()
                .find(|profile| is_a2dp_sink_profile(profile))
        })
}

fn normalized_mac(value: &str) -> String {
    value
        .chars()
        .filter(|character| !matches!(*character, ':' | '-' | '_'))
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn pulse_property_matches_mac(property: &str, mac: &str) -> bool {
    let normalized_target_mac = normalized_mac(mac);
    !normalized_target_mac.is_empty()
        && normalized_mac(property).contains(normalized_target_mac.as_str())
}

struct MediaControllerState {
    connected_device_mac: String,
    local_mac: String,
    is_playing: bool,
    paused_by_app_services: Vec<String>,
    device_index: Option<u32>,
    cached_a2dp_profile: String,
    old_in_ear_data: Vec<bool>,
    user_played_the_media: bool,
    i_paused_the_media: bool,
    ear_detection_enabled: bool,
    disconnect_when_not_wearing: bool,
    conv_original_volume: Option<u32>,
    conv_conversation_started: bool,
    playback_listener_running: bool,
}

impl MediaControllerState {
    fn new() -> Self {
        MediaControllerState {
            connected_device_mac: String::new(),
            local_mac: String::new(),
            is_playing: false,
            paused_by_app_services: Vec::new(),
            device_index: None,
            cached_a2dp_profile: String::new(),
            old_in_ear_data: vec![false, false],
            user_played_the_media: false,
            i_paused_the_media: false,
            ear_detection_enabled: true,
            disconnect_when_not_wearing: true,
            conv_original_volume: None,
            conv_conversation_started: false,
            playback_listener_running: false,
        }
    }
}

#[derive(Clone)]
pub struct MediaController {
    state: Arc<Mutex<MediaControllerState>>,
}

impl MediaController {
    pub fn new(connected_mac: String, local_mac: String) -> Self {
        let mut state = MediaControllerState::new();
        state.connected_device_mac = connected_mac;
        state.local_mac = local_mac;
        MediaController {
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub async fn start_playback_listener(&self, aacp_manager: AACPManager) {
        let mut state = self.state.lock().await;
        if state.playback_listener_running {
            debug!("Playback listener already running");
            return;
        }
        state.playback_listener_running = true;
        drop(state);

        let controller_clone = self.clone();
        tokio::spawn(async move {
            controller_clone.playback_listener_loop(aacp_manager).await;
        });
    }

    async fn playback_listener_loop(&self, aacp_manager: AACPManager) {
        info!("Starting playback listener loop");
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let is_playing = tokio::task::spawn_blocking(|| Self::check_if_playing())
                .await
                .unwrap_or(false);

            let mut state = self.state.lock().await;
            let was_playing = state.is_playing;
            state.is_playing = is_playing;
            let local_mac = state.local_mac.clone();
            drop(state);

            if !was_playing && is_playing {
                match self
                    .handle_playback_started(
                        &aacp_manager,
                        &local_mac,
                        self.activate_a2dp_profile(),
                    )
                    .await
                {
                    PlaybackTakeoverOutcome::Completed => {}
                    PlaybackTakeoverOutcome::SkippedNotInEar => {
                        info!("Media playback started but buds not in ear, skipping takeover");
                    }
                    PlaybackTakeoverOutcome::AbortedDisconnected => {
                        warn!("Playback takeover aborted because AACP disconnected");
                    }
                    PlaybackTakeoverOutcome::AbortedConnectionChanged => {
                        warn!("Playback takeover aborted because the AACP connection changed");
                    }
                    PlaybackTakeoverOutcome::AbortedEarStateChanged => {
                        info!("Playback takeover aborted because the buds left the ear");
                    }
                    PlaybackTakeoverOutcome::Failed(failure) => {
                        error!("Playback takeover failed: {:?}", failure);
                    }
                }
            }
        }
    }

    async fn handle_playback_started(
        &self,
        aacp_manager: &AACPManager,
        local_mac: &str,
        activate_a2dp: impl std::future::Future<Output = ProfileOperationResult>,
    ) -> PlaybackTakeoverOutcome {
        let connection_generation = {
            let aacp_state = aacp_manager.state.lock().await;
            if aacp_state.sender.is_none() {
                return PlaybackTakeoverOutcome::AbortedDisconnected;
            }
            if !aacp_state
                .ear_detection_status
                .contains(&EarDetectionStatus::InEar)
            {
                return PlaybackTakeoverOutcome::SkippedNotInEar;
            }
            aacp_state.connection_generation
        };

        info!("Media playback started, activating A2DP before taking ownership");
        if let Err(error) = activate_a2dp.await {
            return PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::ProfileActivation(
                error,
            ));
        }

        {
            let aacp_state = aacp_manager.state.lock().await;
            if aacp_state.connection_generation != connection_generation {
                return PlaybackTakeoverOutcome::AbortedConnectionChanged;
            }
            if aacp_state.sender.is_none() {
                return PlaybackTakeoverOutcome::AbortedDisconnected;
            }
            if !aacp_state
                .ear_detection_status
                .contains(&EarDetectionStatus::InEar)
            {
                return PlaybackTakeoverOutcome::AbortedEarStateChanged;
            }
        }

        if let Err(error) = aacp_manager
            .send_control_command_for_generation(
                ControlCommandIdentifiers::OwnsConnection,
                &[0x01],
                connection_generation,
            )
            .await
        {
            return PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::OwnershipSend(
                error.to_string(),
            ));
        }

        let connected_devices = {
            let aacp_state = aacp_manager.state.lock().await;
            if aacp_state.connection_generation != connection_generation {
                return PlaybackTakeoverOutcome::AbortedConnectionChanged;
            }
            if aacp_state.sender.is_none() {
                return PlaybackTakeoverOutcome::AbortedDisconnected;
            }
            if !aacp_state
                .ear_detection_status
                .contains(&EarDetectionStatus::InEar)
            {
                return PlaybackTakeoverOutcome::AbortedEarStateChanged;
            }
            aacp_state.connected_devices.clone()
        };

        info!("Ownership queued, sending smart-routing takeover packets");
        for device in connected_devices {
            if device.mac == local_mac {
                continue;
            }

            if let Err(error) = aacp_manager
                .send_media_information_for_generation(
                    local_mac,
                    &device.mac,
                    true,
                    connection_generation,
                )
                .await
            {
                return PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::RoutingSend {
                    device_mac: device.mac,
                    packet: RoutingPacketKind::MediaInformation,
                    error: error.to_string(),
                });
            }
            if let Err(error) = aacp_manager
                .send_smart_routing_show_ui_for_generation(&device.mac, connection_generation)
                .await
            {
                return PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::RoutingSend {
                    device_mac: device.mac,
                    packet: RoutingPacketKind::ShowUi,
                    error: error.to_string(),
                });
            }
            if let Err(error) = aacp_manager
                .send_hijack_request_for_generation(&device.mac, connection_generation)
                .await
            {
                return PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::RoutingSend {
                    device_mac: device.mac,
                    packet: RoutingPacketKind::HijackRequest,
                    error: error.to_string(),
                });
            }
        }

        debug!("Completed playback takeover process");
        PlaybackTakeoverOutcome::Completed
    }

    fn check_if_playing() -> bool {
        let conn = match Connection::new_session() {
            Ok(c) => c,
            Err(_) => return false,
        };
        Self::list_mpris_services(&conn).iter().any(|service| {
            let proxy = conn.with_proxy(service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));
            proxy
                .get::<String>("org.mpris.MediaPlayer2.Player", "PlaybackStatus")
                .map(|s| s == "Playing")
                .unwrap_or(false)
        })
    }

    fn is_kdeconnect_service(service: &str) -> bool {
        service.starts_with("org.mpris.MediaPlayer2.kdeconnect.mpris_")
    }

    pub async fn handle_ear_detection(
        &self,
        old_statuses: Vec<EarDetectionStatus>,
        new_statuses: Vec<EarDetectionStatus>,
    ) {
        debug!(
            "Entering handle_ear_detection with old_statuses: {:?}, new_statuses: {:?}",
            old_statuses, new_statuses
        );

        let old_in_ear_data: Vec<bool> = old_statuses
            .iter()
            .map(|s| *s == EarDetectionStatus::InEar)
            .collect();
        let new_in_ear_data: Vec<bool> = new_statuses
            .iter()
            .map(|s| *s == EarDetectionStatus::InEar)
            .collect();

        let in_ear = new_in_ear_data.iter().all(|&b| b);
        let old_all_out = old_in_ear_data.iter().all(|&b| !b);
        let new_has_at_least_one_in = new_in_ear_data.iter().any(|&b| b);
        let new_all_out = new_in_ear_data.iter().all(|&b| !b);

        debug!(
            "Computed states: in_ear={}, old_all_out={}, new_has_at_least_one_in={}, new_all_out={}",
            in_ear, old_all_out, new_has_at_least_one_in, new_all_out
        );

        {
            let state = self.state.lock().await;
            if !state.ear_detection_enabled {
                debug!("Ear detection disabled, skipping");
                return;
            }
        }

        if new_has_at_least_one_in && old_all_out {
            debug!("Condition met: buds inserted, activating A2DP and checking play state");
            if let Err(error) = self.activate_a2dp_profile().await {
                error!("Failed to activate A2DP after ear insertion: {:?}", error);
            }
            {
                let mut state = self.state.lock().await;
                if state.is_playing {
                    state.user_played_the_media = true;
                    debug!("Set user_played_the_media to true as media was playing");
                }
            }
        } else if new_all_out {
            debug!("Condition met: buds removed, pausing media");
            self.pause().await;
            let disconnect_when_not_wearing = self.state.lock().await.disconnect_when_not_wearing;
            if disconnect_when_not_wearing {
                debug!("Disconnect when not wearing enabled, deactivating A2DP");
                if let Err(error) = self.deactivate_a2dp_profile().await {
                    error!("Failed to deactivate A2DP after ear removal: {:?}", error);
                }
            }
        }

        let reset_user_played = (old_in_ear_data.iter().any(|&b| !b)
            && new_in_ear_data.iter().all(|&b| b))
            || (new_in_ear_data.iter().any(|&b| !b) && old_in_ear_data.iter().all(|&b| b));
        if reset_user_played {
            debug!("Transition detected, resetting user_played_the_media");
            let mut state = self.state.lock().await;
            state.user_played_the_media = false;
        }

        info!(
            "Ear Detection - old_in_ear_data: {:?}, new_in_ear_data: {:?}",
            old_in_ear_data, new_in_ear_data
        );

        let mut old_sorted = old_in_ear_data.clone();
        old_sorted.sort();
        let mut new_sorted = new_in_ear_data.clone();
        new_sorted.sort();
        if new_sorted != old_sorted {
            debug!("Ear data changed, checking resume/pause logic");
            if in_ear {
                debug!("Resuming media as buds are in ear");
                self.resume().await;
                self.state.lock().await.i_paused_the_media = false;
            } else if !old_all_out {
                debug!("Pausing media as buds are not fully in ear");
                self.pause().await;
                self.state.lock().await.i_paused_the_media = true;
            } else {
                debug!("Playing media");
                self.resume().await;
                self.state.lock().await.i_paused_the_media = false;
            }
        }

        {
            let mut state = self.state.lock().await;
            state.old_in_ear_data = new_in_ear_data;
            debug!("Updated old_in_ear_data to {:?}", state.old_in_ear_data);
        }
    }

    pub async fn activate_a2dp_profile(&self) -> ProfileOperationResult {
        debug!("Entering activate_a2dp_profile");
        let (mac, cached_device_index) = {
            let state = self.state.lock().await;
            (state.connected_device_mac.clone(), state.device_index)
        };

        if mac.is_empty() {
            warn!("Connected device MAC is empty, cannot activate A2DP profile");
            return Err(ProfileOperationError::NoConnectedDevice);
        }

        let device_index = self
            .resolve_device_index(&mac, cached_device_index, self.get_audio_device_index(&mac))
            .await?;

        if !self.is_a2dp_profile_available().await {
            warn!("A2DP profile not available, attempting to restart WirePlumber");
            if !self.restart_wire_plumber().await {
                error!("Could not restart WirePlumber, A2DP profile unavailable");
                return Err(ProfileOperationError::WirePlumberRestartFailed);
            }

            self.refresh_device_index_and_recheck(
                &mac,
                self.get_audio_device_index(&mac),
                self.is_a2dp_profile_available(),
            )
            .await?;
        }

        let preferred_profile = self.get_preferred_a2dp_profile().await;
        if preferred_profile.is_empty() {
            error!("No suitable A2DP profile found");
            return Err(ProfileOperationError::NoPreferredA2dpProfile);
        }

        let current_device_index = {
            let state = self.state.lock().await;
            if state.connected_device_mac != mac {
                return Err(ProfileOperationError::ConnectedDeviceChanged);
            }
            state.device_index
        };
        let current_device_index =
            current_device_index.ok_or(ProfileOperationError::AudioDeviceNotFound)?;
        if current_device_index != device_index {
            debug!(
                "Audio device index changed from {} to {} during activation",
                device_index, current_device_index
            );
        }

        info!("Activating A2DP profile for AirPods: {}", preferred_profile);
        let profile_name = preferred_profile.clone();
        let success = tokio::task::spawn_blocking(move || {
            set_card_profile_sync(current_device_index, &profile_name)
        })
        .await
        .map_err(|_| ProfileOperationError::ProfileWorkerStopped)?;

        if !success {
            warn!("Failed to activate A2DP profile: {}", preferred_profile);
            return Err(ProfileOperationError::ProfileSwitchFailed(
                preferred_profile,
            ));
        }

        info!("Successfully activated A2DP profile: {}", preferred_profile);
        Ok(())
    }

    async fn resolve_device_index<Lookup>(
        &self,
        expected_mac: &str,
        cached_device_index: Option<u32>,
        lookup: Lookup,
    ) -> Result<u32, ProfileOperationError>
    where
        Lookup: std::future::Future<Output = Option<u32>>,
    {
        if let Some(device_index) = cached_device_index {
            return Ok(device_index);
        }

        warn!("Device index not found, trying to get it.");
        let device_index = lookup
            .await
            .ok_or(ProfileOperationError::AudioDeviceNotFound)?;
        let mut state = self.state.lock().await;
        if state.connected_device_mac != expected_mac {
            return Err(ProfileOperationError::ConnectedDeviceChanged);
        }
        state.device_index = Some(device_index);
        Ok(device_index)
    }

    async fn refresh_device_index_and_recheck<Lookup, Recheck>(
        &self,
        expected_mac: &str,
        lookup: Lookup,
        recheck: Recheck,
    ) -> Result<u32, ProfileOperationError>
    where
        Lookup: std::future::Future<Output = Option<u32>>,
        Recheck: std::future::Future<Output = bool>,
    {
        let refreshed_device_index = lookup.await;
        {
            let mut state = self.state.lock().await;
            if state.connected_device_mac != expected_mac {
                return Err(ProfileOperationError::ConnectedDeviceChanged);
            }
            state.device_index = refreshed_device_index;
        }

        debug!(
            "Updated device_index after WirePlumber restart: {:?}",
            refreshed_device_index
        );
        let refreshed_device_index =
            refreshed_device_index.ok_or(ProfileOperationError::AudioDeviceNotFound)?;

        if !recheck.await {
            error!("A2DP profile still not available after WirePlumber restart");
            return Err(ProfileOperationError::A2dpProfileUnavailable);
        }

        Ok(refreshed_device_index)
    }

    async fn pause(&self) {
        debug!("Pausing playback");

        let paused_services = tokio::task::spawn_blocking(|| {
            let conn = match Connection::new_session() {
                Ok(c) => c,
                Err(_) => return vec![],
            };
            let mut paused_services = Vec::new();

            for service in Self::list_mpris_services(&conn) {
                debug!("Checking playback status for service: {}", service);
                let proxy =
                    conn.with_proxy(&service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));

                if let Ok(playback_status) =
                    proxy.get::<String>("org.mpris.MediaPlayer2.Player", "PlaybackStatus")
                    && playback_status == "Playing"
                {
                    if proxy
                        .method_call::<(), _, &str, &str>(
                            "org.mpris.MediaPlayer2.Player",
                            "Pause",
                            (),
                        )
                        .is_ok()
                    {
                        info!("Paused playback for: {}", service);
                        paused_services.push(service);
                    } else {
                        error!("Failed to pause {}", service);
                    }
                }
            }

            paused_services
        })
        .await
        .unwrap();

        if !paused_services.is_empty() {
            info!("Paused {} media player(s) via DBus", paused_services.len());
            let mut state = self.state.lock().await;
            state.paused_by_app_services = paused_services;
            state.is_playing = false;
        } else {
            info!("No playing media players found to pause");
        }
    }

    pub async fn pause_all_media(&self) {
        debug!("Pausing all media (without tracking for resume)");

        let paused_count = tokio::task::spawn_blocking(|| {
            let conn = match Connection::new_session() {
                Ok(c) => c,
                Err(_) => return 0,
            };
            let mut paused_count = 0;

            for service in Self::list_mpris_services(&conn) {
                let proxy =
                    conn.with_proxy(&service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));
                if let Ok(playback_status) =
                    proxy.get::<String>("org.mpris.MediaPlayer2.Player", "PlaybackStatus")
                    && playback_status == "Playing"
                {
                    if proxy
                        .method_call::<(), _, &str, &str>(
                            "org.mpris.MediaPlayer2.Player",
                            "Pause",
                            (),
                        )
                        .is_ok()
                    {
                        info!("Paused playback for: {}", service);
                        paused_count += 1;
                    } else {
                        error!("Failed to pause {}", service);
                    }
                }
            }
            paused_count
        })
        .await
        .unwrap();

        if paused_count > 0 {
            info!(
                "Paused {} media player(s) due to ownership loss",
                paused_count
            );
            self.state.lock().await.is_playing = false;
        } else {
            debug!("No playing media players found to pause");
        }
    }

    async fn resume(&self) {
        debug!("Resuming playback");
        let services = self.state.lock().await.paused_by_app_services.clone();

        if services.is_empty() {
            debug!("No services to resume");
            return;
        }

        let resumed_count = tokio::task::spawn_blocking(move || {
            let conn = match Connection::new_session() {
                Ok(c) => c,
                Err(_) => return 0,
            };
            let mut resumed_count = 0;
            for service in services {
                let proxy =
                    conn.with_proxy(&service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));
                if proxy
                    .method_call::<(), _, &str, &str>("org.mpris.MediaPlayer2.Player", "Play", ())
                    .is_ok()
                {
                    info!("Resumed playback for: {}", service);
                    resumed_count += 1;
                } else {
                    warn!("Failed to resume {}", service);
                }
            }
            resumed_count
        })
        .await
        .unwrap();

        if resumed_count > 0 {
            info!("Resumed {} media player(s) via DBus", resumed_count);
            self.state.lock().await.paused_by_app_services.clear();
        } else {
            error!("Failed to resume any media players via DBus");
        }
    }

    pub async fn next_track(&self) {
        info!("Skipping to next track");
        self.mpris_command("Next").await;
    }

    pub async fn previous_track(&self) {
        info!("Going to previous track");
        self.mpris_command("Previous").await;
    }

    async fn is_a2dp_profile_available(&self) -> bool {
        let device_index = {
            let state = self.state.lock().await;
            state.device_index
        };
        let index = match device_index {
            Some(index) => index,
            None => {
                debug!("Device index is None, returning false");
                return false;
            }
        };

        tokio::task::spawn_blocking(move || {
            let available = get_card_info_list_sync()
                .iter()
                .find(|c| c.index == index)
                .map(|card| {
                    preferred_a2dp_profile(card.profiles.iter().filter_map(|p| p.name.as_deref()))
                        .is_some()
                })
                .unwrap_or(false);
            debug!("A2DP profile available: {}", available);
            available
        })
        .await
        .unwrap_or(false)
    }

    async fn get_preferred_a2dp_profile(&self) -> String {
        let state = self.state.lock().await;
        let device_index = state.device_index;
        let cached_profile = state.cached_a2dp_profile.clone();
        drop(state);

        let index = match device_index {
            Some(i) => i,
            None => return String::new(),
        };

        if !cached_profile.is_empty() && self.is_profile_available(index, &cached_profile).await {
            debug!("Using cached A2DP profile: {}", cached_profile);
            return cached_profile;
        }

        let profile = tokio::task::spawn_blocking(move || {
            get_card_info_list_sync()
                .iter()
                .find(|card| card.index == index)
                .and_then(|card| {
                    preferred_a2dp_profile(
                        card.profiles
                            .iter()
                            .filter_map(|profile| profile.name.as_deref()),
                    )
                })
                .map(str::to_owned)
        })
        .await
        .unwrap_or(None);

        if let Some(profile) = profile {
            info!("Selected best available A2DP profile: {}", profile);
            self.state.lock().await.cached_a2dp_profile = profile.clone();
            return profile;
        }
        debug!("No suitable profile found");
        String::new()
    }

    async fn is_profile_available(&self, card_index: u32, profile: &str) -> bool {
        let profile_name = profile.to_string();
        tokio::task::spawn_blocking(move || {
            let available = get_card_info_list_sync()
                .iter()
                .find(|c| c.index == card_index)
                .map(|card| {
                    card.profiles
                        .iter()
                        .any(|p| p.name.as_ref() == Some(&profile_name))
                })
                .unwrap_or(false);
            debug!("Profile {} available: {}", profile_name, available);
            available
        })
        .await
        .unwrap_or(false)
    }

    async fn restart_wire_plumber(&self) -> bool {
        info!("Restarting WirePlumber to rediscover A2DP profiles");
        let result = Command::new("systemctl")
            .args(["--user", "restart", "wireplumber"])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                info!("WirePlumber restarted successfully");
                tokio::time::sleep(Duration::from_secs(2)).await;
                true
            }
            _ => {
                error!("Failed to restart WirePlumber. Do you use wireplumber?");
                false
            }
        }
    }

    async fn get_audio_device_index(&self, mac: &str) -> Option<u32> {
        if mac.is_empty() {
            return None;
        }
        let mac_clone = mac.to_string();

        tokio::task::spawn_blocking(move || {
            for card in get_card_info_list_sync() {
                if let Some(device_string) = card.proplist.get_str("device.string")
                    && pulse_property_matches_mac(&device_string, &mac_clone)
                {
                    info!(
                        "Found audio device index for MAC {}: {}",
                        mac_clone, card.index
                    );
                    return Some(card.index);
                }
            }
            error!(
                "No matching Bluetooth card found for MAC address: {}",
                mac_clone
            );
            None
        })
        .await
        .unwrap_or(None)
    }

    pub async fn deactivate_a2dp_profile(&self) -> ProfileOperationResult {
        debug!("Entering deactivate_a2dp_profile");
        let (mac, cached_device_index) = {
            let state = self.state.lock().await;
            (state.connected_device_mac.clone(), state.device_index)
        };

        if mac.is_empty() {
            warn!("Connected device MAC is empty, cannot deactivate A2DP profile");
            return Err(ProfileOperationError::NoConnectedDevice);
        }

        self.resolve_device_index(&mac, cached_device_index, self.get_audio_device_index(&mac))
            .await?;

        let device_index = {
            let state = self.state.lock().await;
            if state.connected_device_mac != mac {
                return Err(ProfileOperationError::ConnectedDeviceChanged);
            }
            state
                .device_index
                .ok_or(ProfileOperationError::AudioDeviceNotFound)?
        };

        info!("Deactivating A2DP profile for AirPods by setting to off");
        let success = tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                set_card_profile_sync(device_index, "off")
            }))
            .unwrap_or_else(|e| {
                warn!("Panic in set_card_profile_sync: {:?}", e);
                false
            })
        })
        .await
        .map_err(|_| ProfileOperationError::ProfileWorkerStopped)?;

        if !success {
            warn!("Failed to deactivate A2DP profile");
            return Err(ProfileOperationError::ProfileSwitchFailed("off".into()));
        }

        info!("Successfully deactivated A2DP profile");
        Ok(())
    }

    pub async fn handle_conversational_awareness(&self, status: u8) {
        debug!(
            "Entering handle_conversational_awareness with status: {}",
            status
        );

        let mac = self.state.lock().await.connected_device_mac.clone();
        if mac.is_empty() {
            debug!("No connected device MAC, skipping conversational awareness");
            return;
        }

        let sink = match get_sink_name_by_mac(&mac).await {
            Some(s) => s,
            None => {
                warn!(
                    "Could not find sink for MAC {}, skipping conversational awareness",
                    mac
                );
                return;
            }
        };

        let current_volume_opt = tokio::task::spawn_blocking({
            let sink = sink.clone();
            move || get_sink_volume_percent_by_name_sync(&sink)
        })
        .await
        .unwrap_or(None);

        match status {
            1 => {
                let original = current_volume_opt.unwrap_or(0);
                debug!("Conversation start (1). Current volume: {}", original);
                {
                    let mut state = self.state.lock().await;
                    if !state.conv_conversation_started {
                        state.conv_original_volume = Some(original);
                        state.conv_conversation_started = true;
                    } else {
                        debug!(
                            "Conversation already started; not overwriting conv_original_volume"
                        );
                    }
                }
                if original > 25 {
                    let sink_clone = sink.clone();
                    tokio::task::spawn_blocking(move || transition_sink_volume(&sink_clone, 25))
                        .await
                        .unwrap_or(false);
                    info!(
                        "Conversation start: lowered volume to 25% (original {})",
                        original
                    );
                } else {
                    debug!("Original volume {} <= 25, not reducing to 25", original);
                }
            }
            2 => {
                let original = self.state.lock().await.conv_original_volume;
                if let Some(orig) = original {
                    debug!("Conversation reduce (2). Original: {}", orig);
                    if orig > 15 {
                        let sink_clone = sink.clone();
                        tokio::task::spawn_blocking(move || {
                            transition_sink_volume(&sink_clone, 15)
                        })
                        .await
                        .unwrap_or(false);
                        info!(
                            "Conversation reduce: lowered volume to 15% (original {})",
                            orig
                        );
                    } else {
                        debug!("Original {} <= 15, not reducing to 15", orig);
                    }
                } else {
                    debug!("No original volume known for status 2, skipping");
                }
            }
            3 => {
                let (conv_started, conv_original) = {
                    let state = self.state.lock().await;
                    (state.conv_conversation_started, state.conv_original_volume)
                };
                if !conv_started {
                    debug!("Received status 3 but conversation was not started; ignoring increase");
                    return;
                }
                if let Some(orig) = conv_original {
                    let target = orig.min(25);
                    let sink_clone = sink.clone();
                    tokio::task::spawn_blocking(move || {
                        transition_sink_volume(&sink_clone, target)
                    })
                    .await
                    .unwrap_or(false);
                    info!(
                        "Conversation partial increase (3): set volume to {} (original {})",
                        target, orig
                    );
                } else if let Some(orig_from_current) = current_volume_opt {
                    let target = orig_from_current.min(25);
                    let sink_clone = sink.clone();
                    tokio::task::spawn_blocking(move || {
                        transition_sink_volume(&sink_clone, target)
                    })
                    .await
                    .unwrap_or(false);
                    info!(
                        "Conversation partial increase (3) with fallback current: set volume to {} (measured {})",
                        target, orig_from_current
                    );
                } else {
                    debug!("No original volume known for status 3, skipping");
                }
            }
            4 | 6 | 7 => {
                debug!("Conversation end ({}), restoring volume if needed", status);
                self.restore_volume_if_needed(&sink).await;
            }
            _ => {
                debug!("Conversation status ({}), ignoring", status);
            }
        }
    }

    async fn mpris_command(&self, command: &'static str) {
        tokio::task::spawn_blocking(move || {
            let conn = match Connection::new_session() {
                Ok(c) => c,
                Err(_) => return,
            };

            let services = Self::list_mpris_services(&conn);
            let mut playing = None;
            let mut fallback = None;

            for service in &services {
                let proxy =
                    conn.with_proxy(service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));
                if let Ok(status) =
                    proxy.get::<String>("org.mpris.MediaPlayer2.Player", "PlaybackStatus")
                {
                    if status == "Playing" && playing.is_none() {
                        playing = Some(service);
                    }
                }
                if fallback.is_none() {
                    fallback = Some(service);
                }
            }

            if let Some(service) = playing.or(fallback) {
                let proxy =
                    conn.with_proxy(service, "/org/mpris/MediaPlayer2", Duration::from_secs(5));
                if proxy
                    .method_call::<(), _, &str, &str>("org.mpris.MediaPlayer2.Player", command, ())
                    .is_ok()
                {
                    info!("Sent {} to: {}", command, service);
                } else {
                    debug!("Failed to send {} to: {}", command, service);
                }
            }
        })
        .await
        .unwrap();
    }

    async fn restore_volume_if_needed(&self, sink: &str) {
        let maybe_original = {
            let mut state = self.state.lock().await;
            if state.conv_conversation_started {
                let orig = state.conv_original_volume;
                state.conv_original_volume = None;
                state.conv_conversation_started = false;
                orig
            } else {
                return;
            }
        };

        if let Some(orig) = maybe_original {
            let sink = sink.to_string();
            tokio::task::spawn_blocking(move || transition_sink_volume(&sink, orig))
                .await
                .unwrap_or(false);
        }
    }

    fn list_mpris_services(conn: &Connection) -> Vec<String> {
        let proxy = conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_secs(5),
        );

        let (names,): (Vec<String>,) =
            match proxy.method_call("org.freedesktop.DBus", "ListNames", ()) {
                Ok(n) => n,
                Err(_) => return vec![],
            };

        names
            .into_iter()
            .filter(|s| s.starts_with("org.mpris.MediaPlayer2.") && !Self::is_kdeconnect_service(s))
            .collect()
    }
}

// --- PulseAudio helpers ---

fn pulse_connect() -> Option<(Mainloop, Context)> {
    let mut mainloop = Mainloop::new()?;
    let mut context = Context::new(&mainloop, "LibrePods")?;
    context
        .connect(None, ContextFlagSet::NOAUTOSPAWN, None)
        .ok()?;
    loop {
        mainloop.iterate(false);
        match context.get_state() {
            libpulse_binding::context::State::Ready => break,
            libpulse_binding::context::State::Failed
            | libpulse_binding::context::State::Terminated => return None,
            _ => {}
        }
    }
    Some((mainloop, context))
}

fn get_card_info_list_sync() -> Vec<OwnedCardInfo> {
    let (mut mainloop, context) = match pulse_connect() {
        Some(c) => c,
        None => return vec![],
    };

    let introspector = context.introspect();
    let cards: Rc<RefCell<Vec<OwnedCardInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let op = introspector.get_card_info_list({
        let cards = cards.clone();
        move |result| {
            if let ListResult::Item(item) = result {
                let profiles = item
                    .profiles
                    .iter()
                    .map(|p| OwnedCardProfileInfo {
                        name: p.name.as_ref().map(|n| n.to_string()),
                    })
                    .collect();
                cards.borrow_mut().push(OwnedCardInfo {
                    index: item.index,
                    proplist: item.proplist.clone(),
                    profiles,
                });
            }
        }
    });

    while op.get_state() == OperationState::Running {
        mainloop.iterate(false);
    }
    mainloop.quit(Retval(0));
    Rc::try_unwrap(cards).unwrap().into_inner()
}

fn get_sink_volume_percent_by_name_sync(sink_name: &str) -> Option<u32> {
    let (mut mainloop, context) = pulse_connect()?;
    let introspector = context.introspect();
    let sink_info: Rc<RefCell<Option<OwnedSinkInfo>>> = Rc::new(RefCell::new(None));
    let op = introspector.get_sink_info_by_name(sink_name, {
        let sink_info = sink_info.clone();
        move |result: ListResult<&SinkInfo>| {
            if let ListResult::Item(item) = result {
                *sink_info.borrow_mut() = Some(OwnedSinkInfo {
                    name: item.name.as_ref().map(|s| s.to_string()),
                    proplist: item.proplist.clone(),
                    volume: item.volume,
                });
            }
        }
    });
    while op.get_state() == OperationState::Running {
        mainloop.iterate(false);
    }
    mainloop.quit(Retval(0));

    let borrowed = sink_info.borrow();
    let info = borrowed.as_ref()?;
    let channels = info.volume.len();
    if channels == 0 {
        return None;
    }
    let total: f64 = info.volume.get().iter().map(|v| v.0 as f64).sum();
    let percent = ((total / channels as f64 / Volume::NORMAL.0 as f64) * 100.0).round() as u32;
    Some(percent)
}

fn set_card_profile_sync(card_index: u32, profile_name: &str) -> bool {
    let (mut mainloop, context) = match pulse_connect() {
        Some(c) => c,
        None => return false,
    };
    let mut introspector = context.introspect();
    let op = introspector.set_card_profile_by_index(card_index, profile_name, None);
    while op.get_state() == OperationState::Running {
        mainloop.iterate(false);
    }
    mainloop.quit(Retval(0));
    true
}

pub fn transition_sink_volume(sink_name: &str, target_volume: u32) -> bool {
    let (mut mainloop, context) = match pulse_connect() {
        Some(c) => c,
        None => return false,
    };
    let mut introspector = context.introspect();
    let sink_info: Rc<RefCell<Option<OwnedSinkInfo>>> = Rc::new(RefCell::new(None));
    let op = introspector.get_sink_info_by_name(sink_name, {
        let sink_info = sink_info.clone();
        move |result: ListResult<&SinkInfo>| {
            if let ListResult::Item(item) = result {
                *sink_info.borrow_mut() = Some(OwnedSinkInfo {
                    name: item.name.as_ref().map(|s| s.to_string()),
                    proplist: item.proplist.clone(),
                    volume: item.volume,
                });
            }
        }
    });
    while op.get_state() == OperationState::Running {
        mainloop.iterate(false);
    }

    if let Some(info) = sink_info.borrow().as_ref() {
        let channels = info.volume.len();
        let mut new_volumes = ChannelVolumes::default();
        let raw =
            (((target_volume as f64) / 100.0) * Volume::NORMAL.0.as_f64().unwrap()).round() as u32;
        new_volumes.set(channels, Volume(raw));

        let op = introspector.set_sink_volume_by_name(sink_name, &new_volumes, None);
        while op.get_state() == OperationState::Running {
            mainloop.iterate(false);
        }
        mainloop.quit(Retval(0));
        true
    } else {
        error!("Sink not found: {}", sink_name);
        false
    }
}

async fn get_sink_name_by_mac(mac: &str) -> Option<String> {
    if mac.is_empty() {
        return None;
    }
    let mac_clone = mac.to_string();

    tokio::task::spawn_blocking(move || {
        let (mut mainloop, context) = pulse_connect()?;
        let introspector = context.introspect();
        let sink_list: Rc<RefCell<Vec<OwnedSinkInfo>>> = Rc::new(RefCell::new(Vec::new()));
        let op = introspector.get_sink_info_list({
            let sink_list = sink_list.clone();
            move |result: ListResult<&SinkInfo>| {
                if let ListResult::Item(item) = result {
                    sink_list.borrow_mut().push(OwnedSinkInfo {
                        name: item.name.as_ref().map(|s| s.to_string()),
                        proplist: item.proplist.clone(),
                        volume: item.volume,
                    });
                }
            }
        });
        while op.get_state() == OperationState::Running {
            mainloop.iterate(false);
        }
        mainloop.quit(Retval(0));

        for sink in sink_list.borrow().iter() {
            let matches_mac = ["device.string", "bluez.path"]
                .into_iter()
                .filter_map(|property| sink.proplist.get_str(property))
                .any(|property| pulse_property_matches_mac(&property, &mac_clone));
            if matches_mac && let Some(name) = &sink.name {
                info!("Found sink name for MAC {}: {}", mac_clone, name);
                return Some(name.to_string());
            }
        }
        error!("No matching sink found for MAC address: {}", mac_clone);
        None
    })
    .await
    .unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bluetooth::aacp::{ConnectedDevice, opcodes};
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    const CONNECTED_MAC: &str = "AA:BB:CC:DD:EE:FF";
    const LOCAL_MAC: &str = "11:22:33:44:55:66";
    const REMOTE_MAC: &str = "77:88:99:AA:BB:CC";

    fn controller() -> MediaController {
        MediaController::new(CONNECTED_MAC.into(), LOCAL_MAC.into())
    }

    async fn takeover_manager(capacity: usize) -> (AACPManager, mpsc::Receiver<Vec<u8>>) {
        let manager = AACPManager::new();
        let (packet_tx, packet_rx) = mpsc::channel(capacity);
        {
            let mut state = manager.state.lock().await;
            state.sender = Some(packet_tx);
            state.connection_generation = 7;
            state.ear_detection_status = vec![EarDetectionStatus::InEar];
            state.connected_devices = vec![ConnectedDevice {
                mac: REMOTE_MAC.into(),
                info1: 0,
                info2: 0,
                r#type: None,
            }];
        }
        (manager, packet_rx)
    }

    #[tokio::test]
    async fn device_index_lookup_completes_without_media_state_guard() {
        let controller = controller();
        let lookup_controller = controller.clone();

        let device_index = timeout(
            Duration::from_millis(250),
            controller.resolve_device_index(CONNECTED_MAC, None, async move {
                let state = lookup_controller.state.lock().await;
                drop(state);
                Some(21)
            }),
        )
        .await
        .expect("device lookup waited on a MediaController state guard")
        .unwrap();

        assert_eq!(device_index, 21);
        assert_eq!(controller.state.lock().await.device_index, Some(21));
    }

    #[tokio::test]
    async fn restart_refresh_and_recheck_complete_without_media_state_guard() {
        let controller = controller();
        let lookup_controller = controller.clone();
        let recheck_controller = controller.clone();

        let device_index = timeout(
            Duration::from_millis(250),
            controller.refresh_device_index_and_recheck(
                CONNECTED_MAC,
                async move {
                    let state = lookup_controller.state.lock().await;
                    drop(state);
                    Some(42)
                },
                async move {
                    let state = recheck_controller.state.lock().await;
                    assert_eq!(state.device_index, Some(42));
                    true
                },
            ),
        )
        .await
        .expect("restart refresh or profile recheck retained the MediaController state guard")
        .unwrap();

        assert_eq!(device_index, 42);
    }

    #[tokio::test]
    async fn takeover_aborts_when_connection_generation_changes_during_activation() {
        let controller = controller();
        let (manager, mut packet_rx) = takeover_manager(4).await;
        let activation_manager = manager.clone();

        let outcome = controller
            .handle_playback_started(&manager, LOCAL_MAC, async move {
                activation_manager.state.lock().await.connection_generation += 1;
                Ok::<(), ProfileOperationError>(())
            })
            .await;

        assert_eq!(outcome, PlaybackTakeoverOutcome::AbortedConnectionChanged);
        assert!(packet_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn takeover_aborts_when_ear_state_changes_during_activation() {
        let controller = controller();
        let (manager, mut packet_rx) = takeover_manager(4).await;
        let activation_manager = manager.clone();

        let outcome = controller
            .handle_playback_started(&manager, LOCAL_MAC, async move {
                activation_manager.state.lock().await.ear_detection_status =
                    vec![EarDetectionStatus::OutOfEar];
                Ok::<(), ProfileOperationError>(())
            })
            .await;

        assert_eq!(outcome, PlaybackTakeoverOutcome::AbortedEarStateChanged);
        assert!(packet_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn takeover_reports_profile_activation_failure_without_sending() {
        let controller = controller();
        let (manager, mut packet_rx) = takeover_manager(4).await;

        let outcome = controller
            .handle_playback_started(&manager, LOCAL_MAC, async {
                Err(ProfileOperationError::WirePlumberRestartFailed)
            })
            .await;

        assert_eq!(
            outcome,
            PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::ProfileActivation(
                ProfileOperationError::WirePlumberRestartFailed,
            ),)
        );
        assert!(packet_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn takeover_reports_closed_ownership_channel() {
        let controller = controller();
        let (manager, packet_rx) = takeover_manager(4).await;
        drop(packet_rx);

        let outcome = controller
            .handle_playback_started(&manager, LOCAL_MAC, async {
                Ok::<(), ProfileOperationError>(())
            })
            .await;

        assert!(matches!(
            outcome,
            PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::OwnershipSend(_))
        ));
    }

    #[tokio::test]
    async fn takeover_reports_routing_send_failure() {
        let controller = controller();
        let (manager, mut packet_rx) = takeover_manager(1).await;
        let takeover = tokio::spawn(async move {
            controller
                .handle_playback_started(&manager, LOCAL_MAC, async {
                    Ok::<(), ProfileOperationError>(())
                })
                .await
        });

        let ownership = timeout(Duration::from_millis(250), packet_rx.recv())
            .await
            .expect("timed out waiting for ownership packet")
            .expect("ownership packet channel closed");
        assert_eq!(ownership[4], opcodes::CONTROL_COMMAND);
        drop(packet_rx);

        let outcome = timeout(Duration::from_millis(250), takeover)
            .await
            .expect("routing send did not observe the closed channel")
            .expect("takeover task failed");
        assert!(matches!(
            outcome,
            PlaybackTakeoverOutcome::Failed(PlaybackTakeoverFailure::RoutingSend { .. })
        ));
    }

    #[tokio::test]
    async fn takeover_queues_ownership_before_exact_routing_packet_order() {
        let controller = controller();
        let (manager, mut packet_rx) = takeover_manager(4).await;

        let outcome = timeout(
            Duration::from_millis(250),
            controller.handle_playback_started(&manager, LOCAL_MAC, async {
                Ok::<(), ProfileOperationError>(())
            }),
        )
        .await
        .expect("playback takeover deadlocked while sending packets");
        assert_eq!(outcome, PlaybackTakeoverOutcome::Completed);

        let mut packets = Vec::new();
        for _ in 0..4 {
            packets.push(
                timeout(Duration::from_millis(250), packet_rx.recv())
                    .await
                    .expect("timed out waiting for takeover packet")
                    .expect("takeover packet channel closed"),
            );
        }

        assert_eq!(
            packets[0],
            vec![
                0x04,
                0x00,
                0x04,
                0x00,
                opcodes::CONTROL_COMMAND,
                0x00,
                ControlCommandIdentifiers::OwnsConnection as u8,
                0x01,
                0x00,
                0x00,
                0x00,
            ]
        );
        assert_eq!(
            packets[1..]
                .iter()
                .map(|packet| (packet[4], packet[12]))
                .collect::<Vec<_>>(),
            vec![
                (opcodes::SMART_ROUTING, 0x82),
                (opcodes::SMART_ROUTING, 0x7E),
                (opcodes::SMART_ROUTING, 0x62),
            ]
        );
        assert!(packet_rx.try_recv().is_err());
    }

    #[test]
    fn preferred_a2dp_profile_returns_hyphenated_name_exactly() {
        let profiles = ["off", "headset-head-unit", "a2dp-sink"];

        assert_eq!(preferred_a2dp_profile(profiles), Some("a2dp-sink"));
    }

    #[test]
    fn preferred_a2dp_profile_returns_underscored_name_exactly() {
        let profiles = ["off", "headset_head_unit", "a2dp_sink"];

        assert_eq!(preferred_a2dp_profile(profiles), Some("a2dp_sink"));
    }

    #[test]
    fn preferred_a2dp_profile_returns_mixed_separator_codec_name_exactly() {
        let profiles = ["off", "a2dp-sink-sbc_xq"];

        assert_eq!(preferred_a2dp_profile(profiles), Some("a2dp-sink-sbc_xq"));
    }

    #[test]
    fn preferred_a2dp_profile_preserves_codec_preference_order() {
        let profiles = [
            "a2dp_sink_aac",
            "a2dp_sink",
            "a2dp-sink-sbc",
            "a2dp-sink-sbc_xq",
        ];

        assert_eq!(preferred_a2dp_profile(profiles), Some("a2dp-sink-sbc_xq"));
    }

    #[test]
    fn preferred_a2dp_profile_falls_back_to_another_sink_codec() {
        let profiles = ["off", "a2dp_sink_ldac"];

        assert_eq!(preferred_a2dp_profile(profiles), Some("a2dp_sink_ldac"));
    }

    #[test]
    fn preferred_a2dp_profile_rejects_non_sink_profiles() {
        let profiles = [
            "off",
            "headset_head_unit",
            "a2dp-source-aac",
            "not-a2dp-sink-aac",
            "a2dp_sink_",
        ];

        assert_eq!(preferred_a2dp_profile(profiles), None);
    }

    #[test]
    fn pulse_property_matches_mac_in_device_string() {
        assert!(pulse_property_matches_mac(
            "bluez_card.28-2D_7f-AA_bb-CC",
            "28:2d:7F:aa:BB:cc"
        ));
    }

    #[test]
    fn pulse_property_matches_mac_in_bluez_path() {
        assert!(pulse_property_matches_mac(
            "/org/bluez/hci0/dev_28_2D_7F_AA_BB_CC",
            "28-2d-7f-aa-bb-cc"
        ));
    }

    #[test]
    fn pulse_property_matches_mac_rejects_another_device() {
        assert!(!pulse_property_matches_mac(
            "bluez_card.28_2D_7F_AA_BB_CD",
            "28:2d:7f:aa:bb:cc"
        ));
    }

    #[test]
    fn pulse_property_matches_mac_rejects_empty_mac() {
        assert!(!pulse_property_matches_mac(
            "bluez_card.28_2D_7F_AA_BB_CC",
            ""
        ));
    }
}
