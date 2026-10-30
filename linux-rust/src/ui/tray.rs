// use ksni::TrayMethods; // provides the spawn method

use ab_glyph::{Font, ScaleFont};
use ksni::{Icon, ToolTip};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, sleep};

use crate::bluetooth::aacp::{BatteryStatus, ControlCommandIdentifiers};
use crate::ui::messages::BluetoothUIMessage;
use crate::utils::{MyTheme, get_app_settings_path};

// A normal AACP setting report should arrive quickly; two seconds avoids leaving stale KDE
// controls visible without making ordinary Bluetooth latency look like a failure.
const TRAY_REQUEST_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TraySetting {
    ListeningMode,
    ConversationDetection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrayRequestKey {
    pub(crate) setting: TraySetting,
    pub(crate) request_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PendingListeningModeRequest {
    pub(crate) request_id: u64,
    pub(crate) mode: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TrayCommand {
    pub(crate) identifier: ControlCommandIdentifiers,
    pub(crate) value: Vec<u8>,
    pub(crate) pending_request_key: Option<TrayRequestKey>,
}

impl TrayCommand {
    pub(crate) fn control(identifier: ControlCommandIdentifiers, value: Vec<u8>) -> Self {
        Self {
            identifier,
            value,
            pending_request_key: None,
        }
    }

    fn listening_mode(request: PendingListeningModeRequest) -> Self {
        Self {
            identifier: ControlCommandIdentifiers::ListeningMode,
            value: vec![request.mode],
            pending_request_key: Some(TrayRequestKey {
                setting: TraySetting::ListeningMode,
                request_id: request.request_id,
            }),
        }
    }

    fn into_parts(self) -> (ControlCommandIdentifiers, Vec<u8>, Option<TrayRequestKey>) {
        (self.identifier, self.value, self.pending_request_key)
    }
}

pub(crate) async fn run_tray_command_worker<
    SendCommand,
    SendFuture,
    ClearPending,
    ClearFuture,
    Error,
>(
    mut command_rx: UnboundedReceiver<TrayCommand>,
    send_command: SendCommand,
    clear_pending: ClearPending,
) where
    SendCommand: Fn(ControlCommandIdentifiers, Vec<u8>) -> SendFuture,
    SendFuture: std::future::Future<Output = Result<(), Error>>,
    ClearPending: Fn(TrayRequestKey) -> ClearFuture + Clone + Send + 'static,
    ClearFuture: std::future::Future<Output = ()> + Send + 'static,
    Error: std::fmt::Display,
{
    while let Some(command) = command_rx.recv().await {
        let (identifier, value, pending_request_key) = command.into_parts();
        if let Some(request_key) = pending_request_key {
            let clear_pending = clear_pending.clone();
            tokio::spawn(async move {
                sleep(TRAY_REQUEST_CONFIRMATION_TIMEOUT).await;
                clear_pending(request_key).await;
            });
        }
        if let Err(error) = send_command(identifier, value).await {
            log::error!("Failed to send control command: {}", error);
            if let Some(request_key) = pending_request_key {
                clear_pending(request_key).await;
            }
        }
    }
}

#[derive(Debug)]
pub struct MyTray {
    pub conversation_detect_enabled: Option<bool>,
    pub battery_headphone: Option<u8>,
    pub battery_headphone_status: Option<BatteryStatus>,
    pub battery_l: Option<u8>,
    pub battery_l_status: Option<BatteryStatus>,
    pub battery_r: Option<u8>,
    pub battery_r_status: Option<BatteryStatus>,
    pub battery_c: Option<u8>,
    pub battery_c_status: Option<BatteryStatus>,
    pub connected: bool,
    pub listening_mode: Option<u8>,
    pub(crate) pending_listening_mode: Option<PendingListeningModeRequest>,
    pub(crate) next_listening_mode_request_id: u64,
    pub allow_off_option: Option<u8>,
    pub(crate) command_tx: Option<UnboundedSender<TrayCommand>>,
    pub ui_tx: Option<UnboundedSender<BluetoothUIMessage>>,
}

impl MyTray {
    fn displayed_listening_mode(&self) -> Option<u8> {
        self.pending_listening_mode
            .map(|request| request.mode)
            .or(self.listening_mode)
    }

    pub(crate) fn apply_listening_mode_report(&mut self, mode: u8) {
        self.listening_mode = Some(mode);
        self.pending_listening_mode = None;
    }

    pub(crate) fn apply_conversation_detection_report(&mut self, enabled: bool) {
        self.conversation_detect_enabled = Some(enabled);
    }

    pub(crate) fn clear_pending_listening_mode_request(&mut self, request_id: u64) {
        if self
            .pending_listening_mode
            .is_some_and(|request| request.request_id == request_id)
        {
            self.pending_listening_mode = None;
        }
    }

    pub(crate) fn clear_pending_request(&mut self, request_key: TrayRequestKey) {
        match request_key.setting {
            TraySetting::ListeningMode => {
                self.clear_pending_listening_mode_request(request_key.request_id);
            }
            TraySetting::ConversationDetection => {}
        }
    }

    fn queue_listening_mode_request(&mut self, mode: u8) {
        let Some(command_tx) = self.command_tx.clone() else {
            return;
        };
        let request = PendingListeningModeRequest {
            request_id: self.next_listening_mode_request_id,
            mode,
        };
        self.next_listening_mode_request_id = self.next_listening_mode_request_id.wrapping_add(1);
        let previous_request = self.pending_listening_mode.replace(request);
        if command_tx
            .send(TrayCommand::listening_mode(request))
            .is_err()
        {
            self.pending_listening_mode = previous_request;
        }
    }
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn title(&self) -> String {
        "AirPods".into()
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        let text = {
            let mut levels: Vec<u8> = Vec::new();
            if let Some(h) = self.battery_headphone {
                levels.push(h);
            } else {
                if let Some(l) = self.battery_l {
                    levels.push(l);
                }
                if let Some(r) = self.battery_r {
                    levels.push(r);
                }
                // if let Some(c) = self.battery_c {
                //     if self.battery_c_status != Some(BatteryStatus::Disconnected) {
                //         levels.push(c);
                //     }
                // }
            }
            let min_battery = levels.iter().min().copied();
            if let Some(b) = min_battery {
                format!("{}", b)
            } else {
                "?".to_string()
            }
        };
        let any_bud_charging = matches!(self.battery_l_status, Some(BatteryStatus::Charging))
            || matches!(self.battery_r_status, Some(BatteryStatus::Charging));
        let app_settings_path = get_app_settings_path();
        let settings = std::fs::read_to_string(&app_settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let text_mode = settings
            .clone()
            .and_then(|v| v.get("tray_text_mode").cloned())
            .and_then(|ttm| serde_json::from_value(ttm).ok())
            .unwrap_or(false);
        let theme = settings
            .and_then(|v| v.get("theme").cloned())
            .and_then(|theme| serde_json::from_value::<MyTheme>(theme).ok());
        let text_color = tray_text_color(theme, any_bud_charging);
        let icon = generate_icon(&text, text_mode, any_bud_charging, text_color);
        vec![icon]
    }
    fn tool_tip(&self) -> ToolTip {
        let format_component =
            |label: &str, level: Option<u8>, status: Option<BatteryStatus>| -> String {
                match level {
                    Some(level) => {
                        let suffix = if status == Some(BatteryStatus::Charging) {
                            "⚡"
                        } else {
                            ""
                        };
                        format!("{}: {}%{}", label, level, suffix)
                    }
                    None if status == Some(BatteryStatus::Disconnected) => {
                        format!("{}: -", label)
                    }
                    None => format!("{}: ?", label),
                }
            };

        let l = format_component("L", self.battery_l, self.battery_l_status);
        let r = format_component("R", self.battery_r, self.battery_r_status);
        let c = format_component("C", self.battery_c, self.battery_c_status);

        ToolTip {
            icon_name: "".to_string(),
            icon_pixmap: vec![],
            title: "Battery Status".to_string(),
            description: format!("{} {} {}", l, r, c),
        }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        if let Some(tx) = &self.ui_tx {
            let _ = tx.send(BluetoothUIMessage::OpenWindow);
        }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let allow_off = self.allow_off_option == Some(0x01);
        let options = if allow_off {
            vec![
                ("Off", 0x01),
                ("Noise Cancellation", 0x02),
                ("Transparency", 0x03),
                ("Adaptive", 0x04),
            ]
        } else {
            vec![
                ("Noise Cancellation", 0x02),
                ("Transparency", 0x03),
                ("Adaptive", 0x04),
            ]
        };
        let selected = self
            .displayed_listening_mode()
            .and_then(|mode| options.iter().position(|&(_, val)| val == mode))
            .unwrap_or(0);
        let options_clone = options.clone();
        vec![
            StandardItem {
                label: "Open Window".into(),
                icon_name: "window-new".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Some(tx) = &this.ui_tx {
                        let _ = tx.send(BluetoothUIMessage::OpenWindow);
                    }
                }),
                ..Default::default()
            }
            .into(),
            RadioGroup {
                selected,
                select: Box::new(move |this: &mut Self, current| {
                    let value = options_clone
                        .get(current)
                        .map(|&(_, val)| val)
                        .unwrap_or(0x02);
                    this.queue_listening_mode_request(value);
                }),
                options: options
                    .into_iter()
                    .map(|(label, _)| RadioItem {
                        label: label.into(),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Conversation Detection".into(),
                checked: self.conversation_detect_enabled.unwrap_or(false),
                enabled: self.conversation_detect_enabled.is_some(),
                activate: Box::new(|this: &mut Self| {
                    if let Some(tx) = &this.command_tx
                        && let Some(is_enabled) = this.conversation_detect_enabled
                    {
                        let new_state = !is_enabled;
                        let value = if !new_state { 0x02 } else { 0x01 };
                        let _ = tx.send(TrayCommand::control(
                            ControlCommandIdentifiers::ConversationDetectConfig,
                            vec![value],
                        ));
                        this.conversation_detect_enabled = Some(new_state);
                    }
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn tray_text_color(theme: Option<MyTheme>, charging: bool) -> [u8; 4] {
    match (charging, theme) {
        (true, _) => [0, 255, 0, 255],
        (false, Some(MyTheme::Light)) => [0, 0, 0, 255],
        (false, _) => [255, 255, 255, 255],
    }
}

fn generate_icon(text: &str, text_mode: bool, charging: bool, text_color: [u8; 4]) -> Icon {
    use ab_glyph::{FontRef, PxScale};
    use image::{ImageBuffer, Rgba};
    use imageproc::drawing::draw_text_mut;

    let width = 64;
    let height = 64;

    let mut img = ImageBuffer::from_fn(width, height, |_, _| Rgba([0u8, 0u8, 0u8, 0u8]));

    let font_data = include_bytes!("../../assets/font/DejaVuSans.ttf");
    let font = match FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => {
            return Icon {
                width: width as i32,
                height: height as i32,
                data: vec![0u8; (width * height * 4) as usize],
            };
        }
    };
    if !text_mode {
        let percentage = text.parse::<f32>().unwrap_or(0.0) / 100.0;

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let inner_radius = 22.0;
        let outer_radius = 28.0;

        // ring background
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > inner_radius && dist <= outer_radius {
                    img.put_pixel(x, y, Rgba([128u8, 128u8, 128u8, 255u8]));
                }
            }
        }

        // ring
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > inner_radius && dist <= outer_radius {
                    let angle = dy.atan2(dx);
                    let angle_from_top =
                        (angle + std::f32::consts::PI / 2.0).rem_euclid(2.0 * std::f32::consts::PI);
                    if angle_from_top <= percentage * 2.0 * std::f32::consts::PI {
                        img.put_pixel(x, y, Rgba([0u8, 255u8, 0u8, 255u8]));
                    }
                }
            }
        }
        if charging {
            let emoji = "⚡";
            let scale = PxScale::from(48.0);
            let color = Rgba([0u8, 255u8, 0u8, 255u8]);
            let scaled_font = font.as_scaled(scale);
            let mut emoji_width = 0.0;
            for c in emoji.chars() {
                let glyph_id = font.glyph_id(c);
                emoji_width += scaled_font.h_advance(glyph_id);
            }
            let x = ((width as f32 - emoji_width) / 2.0).max(0.0) as i32;
            let y = ((height as f32 - scale.y) / 2.0).max(0.0) as i32;
            draw_text_mut(&mut img, color, x, y, scale, &font, emoji);
        }
    } else {
        // battery text
        let scale = PxScale::from(48.0);
        let color = Rgba(text_color);

        let scaled_font = font.as_scaled(scale);
        let mut text_width = 0.0;
        for c in text.chars() {
            let glyph_id = font.glyph_id(c);
            text_width += scaled_font.h_advance(glyph_id);
        }
        let x = ((width as f32 - text_width) / 2.0).max(0.0) as i32;
        let y = ((height as f32 - scale.y) / 2.0).max(0.0) as i32;

        draw_text_mut(&mut img, color, x, y, scale, &font, text);
    }

    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in img.pixels() {
        data.push(pixel[3]);
        data.push(pixel[0]);
        data.push(pixel[1]);
        data.push(pixel[2]);
    }

    Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::task::{Context, Poll};

    type TestSendFuture = Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send + 'static>>;

    struct PendingSend {
        dropped: Arc<AtomicBool>,
    }

    impl Future for PendingSend {
        type Output = Result<(), &'static str>;

        fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }

    impl Drop for PendingSend {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    fn empty_tray() -> MyTray {
        MyTray {
            conversation_detect_enabled: None,
            battery_headphone: None,
            battery_headphone_status: None,
            battery_l: None,
            battery_l_status: None,
            battery_r: None,
            battery_r_status: None,
            battery_c: None,
            battery_c_status: None,
            connected: false,
            listening_mode: None,
            pending_listening_mode: None,
            next_listening_mode_request_id: 0,
            allow_off_option: None,
            command_tx: None,
            ui_tx: None,
        }
    }

    fn listening_mode_group(tray: &MyTray) -> ksni::menu::RadioGroup<MyTray> {
        <MyTray as ksni::Tray>::menu(tray)
            .into_iter()
            .find_map(|item| match item {
                ksni::MenuItem::RadioGroup(group) => Some(group),
                _ => None,
            })
            .expect("tray should expose a listening-mode radio group")
    }

    fn conversation_detection_item(tray: &MyTray) -> ksni::menu::CheckmarkItem<MyTray> {
        let mut items = <MyTray as ksni::Tray>::menu(tray)
            .into_iter()
            .filter_map(|item| match item {
                ksni::MenuItem::Checkmark(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            items.len(),
            1,
            "tray should expose exactly one visible checkmark"
        );
        let item = items.pop().unwrap();
        assert!(item.visible, "conversation checkmark should be visible");
        item
    }

    fn clear_pending_in(
        tray: Arc<tokio::sync::Mutex<MyTray>>,
    ) -> impl Fn(TrayRequestKey) -> Pin<Box<dyn Future<Output = ()> + Send>> + Clone {
        move |request_key| {
            let tray = Arc::clone(&tray);
            Box::pin(async move {
                tray.lock().await.clear_pending_request(request_key);
            })
        }
    }

    #[test]
    fn tray_activation_opens_window() {
        let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.ui_tx = Some(ui_tx);

        <MyTray as ksni::Tray>::activate(&mut tray, 0, 0);

        assert!(matches!(
            ui_rx.try_recv(),
            Ok(BluetoothUIMessage::OpenWindow)
        ));
    }

    #[test]
    fn listening_mode_selection_displays_only_latest_pending_request() {
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.listening_mode = Some(0x02);
        tray.next_listening_mode_request_id = u64::MAX;
        tray.command_tx = Some(command_tx);

        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 1);

        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(
            tray.pending_listening_mode,
            Some(PendingListeningModeRequest {
                request_id: u64::MAX,
                mode: 0x03,
            })
        );
        assert_eq!(
            command_rx.try_recv().unwrap(),
            TrayCommand {
                identifier: ControlCommandIdentifiers::ListeningMode,
                value: vec![0x03],
                pending_request_key: Some(TrayRequestKey {
                    setting: TraySetting::ListeningMode,
                    request_id: u64::MAX,
                }),
            }
        );
        assert_eq!(listening_mode_group(&tray).selected, 1);

        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 2);

        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(
            tray.pending_listening_mode,
            Some(PendingListeningModeRequest {
                request_id: 0,
                mode: 0x04,
            })
        );
        assert_eq!(
            command_rx.try_recv().unwrap(),
            TrayCommand {
                identifier: ControlCommandIdentifiers::ListeningMode,
                value: vec![0x04],
                pending_request_key: Some(TrayRequestKey {
                    setting: TraySetting::ListeningMode,
                    request_id: 0,
                }),
            }
        );
        assert_eq!(listening_mode_group(&tray).selected, 2);
    }

    #[test]
    fn listening_mode_report_confirms_device_state_and_clears_pending_request() {
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.listening_mode = Some(0x02);
        tray.command_tx = Some(command_tx);

        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 1);
        command_rx.try_recv().unwrap();
        tray.apply_listening_mode_report(0x04);

        assert_eq!(tray.listening_mode, Some(0x04));
        assert_eq!(tray.pending_listening_mode, None);
        assert_eq!(listening_mode_group(&tray).selected, 2);
    }

    #[test]
    fn matching_failure_rolls_back_pending_listening_mode_request() {
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.listening_mode = Some(0x02);
        tray.command_tx = Some(command_tx);

        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 1);
        let request_id = command_rx
            .try_recv()
            .unwrap()
            .pending_request_key
            .unwrap()
            .request_id;
        tray.clear_pending_listening_mode_request(request_id);

        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(tray.pending_listening_mode, None);
        assert_eq!(listening_mode_group(&tray).selected, 0);
    }

    #[test]
    fn stale_failure_does_not_roll_back_newer_pending_listening_mode_request() {
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.listening_mode = Some(0x02);
        tray.command_tx = Some(command_tx);

        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 1);
        let stale_request_id = command_rx
            .try_recv()
            .unwrap()
            .pending_request_key
            .unwrap()
            .request_id;
        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 2);
        let current_request_id = command_rx
            .try_recv()
            .unwrap()
            .pending_request_key
            .unwrap()
            .request_id;

        tray.clear_pending_listening_mode_request(stale_request_id);

        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(
            tray.pending_listening_mode,
            Some(PendingListeningModeRequest {
                request_id: current_request_id,
                mode: 0x04,
            })
        );
        assert_eq!(listening_mode_group(&tray).selected, 2);
    }

    #[test]
    fn conversation_detection_displays_pending_without_mutating_confirmed_state() {
        let (command_tx, _command_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tray = empty_tray();
        tray.conversation_detect_enabled = Some(false);
        tray.command_tx = Some(command_tx);

        let item = conversation_detection_item(&tray);
        (item.activate)(&mut tray);

        assert_eq!(tray.conversation_detect_enabled, Some(false));
        assert!(conversation_detection_item(&tray).checked);
    }

    #[test]
    fn conversation_detection_enqueue_failure_keeps_confirmed_check() {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        drop(command_rx);
        let mut tray = empty_tray();
        tray.conversation_detect_enabled = Some(false);
        tray.command_tx = Some(command_tx);

        let item = conversation_detection_item(&tray);
        (item.activate)(&mut tray);

        assert_eq!(tray.conversation_detect_enabled, Some(false));
        assert!(!conversation_detection_item(&tray).checked);
    }

    #[tokio::test(start_paused = true)]
    async fn queued_requests_expire_from_creation_and_cancel_backpressured_send() {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let tray = Arc::new(tokio::sync::Mutex::new(empty_tray()));
        {
            let mut tray = tray.lock().await;
            tray.conversation_detect_enabled = Some(false);
            tray.listening_mode = Some(0x02);
            tray.command_tx = Some(command_tx);

            let item = conversation_detection_item(&tray);
            (item.activate)(&mut tray);
            let group = listening_mode_group(&tray);
            (group.select)(&mut tray, 1);
        }

        tokio::time::advance(Duration::from_millis(1_500)).await;

        let send_dropped = Arc::new(AtomicBool::new(false));
        let dropped_for_send = Arc::clone(&send_dropped);
        let attempts = Arc::new(StdMutex::new(Vec::new()));
        let attempts_for_send = Arc::clone(&attempts);
        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_tray_command_worker(
            command_rx,
            move |identifier, value| {
                attempts_for_send.lock().unwrap().push((identifier, value));
                started_tx.send(()).unwrap();
                PendingSend {
                    dropped: Arc::clone(&dropped_for_send),
                }
            },
            clear_pending_in(Arc::clone(&tray)),
        ));

        started_rx.recv().await.unwrap();
        tokio::time::advance(Duration::from_millis(501)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let tray = tray.lock().await;
        assert_eq!(tray.conversation_detect_enabled, Some(false));
        assert_eq!(tray.pending_listening_mode, None);
        assert_eq!(attempts.lock().unwrap().len(), 1);
        assert!(send_dropped.load(Ordering::SeqCst));
        worker.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn superseding_request_cancels_stale_send_before_dispatching_latest() {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let tray = Arc::new(tokio::sync::Mutex::new(empty_tray()));
        {
            let mut tray = tray.lock().await;
            tray.listening_mode = Some(0x02);
            tray.command_tx = Some(command_tx);
            let group = listening_mode_group(&tray);
            (group.select)(&mut tray, 1);
        }

        let first_send_dropped = Arc::new(AtomicBool::new(false));
        let dropped_for_send = Arc::clone(&first_send_dropped);
        let attempts = Arc::new(StdMutex::new(Vec::new()));
        let attempts_for_send = Arc::clone(&attempts);
        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_tray_command_worker(
            command_rx,
            move |identifier, value| -> TestSendFuture {
                attempts_for_send
                    .lock()
                    .unwrap()
                    .push((identifier, value.clone()));
                started_tx.send(value[0]).unwrap();
                if value[0] == 0x03 {
                    Box::pin(PendingSend {
                        dropped: Arc::clone(&dropped_for_send),
                    })
                } else {
                    Box::pin(async { Ok(()) })
                }
            },
            clear_pending_in(Arc::clone(&tray)),
        ));

        assert_eq!(started_rx.recv().await, Some(0x03));
        {
            let mut tray = tray.lock().await;
            let group = listening_mode_group(&tray);
            (group.select)(&mut tray, 2);
        }
        let latest = tokio::time::timeout(Duration::from_millis(1), started_rx.recv())
            .await
            .expect("superseded send should be cancelled immediately");
        assert_eq!(latest, Some(0x04));

        let mut tray = tray.lock().await;
        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(listening_mode_group(&tray).selected, 2);
        assert!(first_send_dropped.load(Ordering::SeqCst));
        assert_eq!(
            attempts
                .lock()
                .unwrap()
                .iter()
                .map(|(_, value)| value[0])
                .collect::<Vec<_>>(),
            [0x03, 0x04]
        );
        tray.apply_listening_mode_report(0x04);
        worker.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn conversation_send_failure_rolls_back_only_pending_check() {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let tray = Arc::new(tokio::sync::Mutex::new(empty_tray()));
        {
            let mut tray = tray.lock().await;
            tray.conversation_detect_enabled = Some(false);
            tray.command_tx = Some(command_tx);
            let item = conversation_detection_item(&tray);
            (item.activate)(&mut tray);
            tray.command_tx = None;
        }

        run_tray_command_worker(
            command_rx,
            |_, _| async { Err::<(), _>("transport failed") },
            clear_pending_in(Arc::clone(&tray)),
        )
        .await;

        let tray = tray.lock().await;
        assert_eq!(tray.conversation_detect_enabled, Some(false));
        assert!(!conversation_detection_item(&tray).checked);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_after_original_deadline_remain_authoritative() {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let tray = Arc::new(tokio::sync::Mutex::new(empty_tray()));
        {
            let mut tray = tray.lock().await;
            tray.conversation_detect_enabled = Some(false);
            tray.listening_mode = Some(0x02);
            tray.command_tx = Some(command_tx);
            let item = conversation_detection_item(&tray);
            (item.activate)(&mut tray);
            let group = listening_mode_group(&tray);
            (group.select)(&mut tray, 1);
            tray.command_tx = None;
        }

        run_tray_command_worker(
            command_rx,
            |_, _| async { Ok::<(), &'static str>(()) },
            clear_pending_in(Arc::clone(&tray)),
        )
        .await;
        tokio::time::advance(TRAY_REQUEST_CONFIRMATION_TIMEOUT + Duration::from_millis(1)).await;
        tokio::task::yield_now().await;

        let mut tray = tray.lock().await;
        assert_eq!(tray.conversation_detect_enabled, Some(false));
        assert!(!conversation_detection_item(&tray).checked);
        assert_eq!(tray.listening_mode, Some(0x02));
        assert_eq!(tray.pending_listening_mode, None);

        tray.apply_conversation_detection_report(true);
        tray.apply_listening_mode_report(0x04);

        assert_eq!(tray.conversation_detect_enabled, Some(true));
        assert!(conversation_detection_item(&tray).checked);
        assert_eq!(tray.listening_mode, Some(0x04));
        assert_eq!(listening_mode_group(&tray).selected, 2);
    }

    #[test]
    fn tray_tooltip_shows_nonzero_disconnected_level() {
        let mut tray = empty_tray();
        tray.battery_l = Some(79);
        tray.battery_l_status = Some(BatteryStatus::Disconnected);

        let tooltip = <MyTray as ksni::Tray>::tool_tip(&tray);

        assert!(tooltip.description.contains("L: 79%"));
    }

    #[test]
    fn tray_text_color_contrasts_with_theme_and_charging_state() {
        let cases = [
            (
                "noncharging Light",
                Some(MyTheme::Light),
                false,
                [0, 0, 0, 255],
            ),
            (
                "noncharging Dark",
                Some(MyTheme::Dark),
                false,
                [255, 255, 255, 255],
            ),
            ("noncharging Auto", None, false, [255, 255, 255, 255]),
            ("charging", Some(MyTheme::Light), true, [0, 255, 0, 255]),
        ];

        for (case, theme, charging, expected) in cases {
            assert_eq!(tray_text_color(theme, charging), expected, "{case}");
        }
    }
}
