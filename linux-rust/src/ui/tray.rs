// use ksni::TrayMethods; // provides the spawn method

use ab_glyph::{Font, ScaleFont};
use ksni::{Icon, ToolTip};
use tokio::sync::mpsc::UnboundedSender;

use crate::bluetooth::aacp::{BatteryStatus, ControlCommandIdentifiers};
use crate::ui::messages::BluetoothUIMessage;
use crate::utils::get_app_settings_path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PendingListeningModeRequest {
    pub(crate) request_id: u64,
    pub(crate) mode: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TrayCommand {
    pub(crate) identifier: ControlCommandIdentifiers,
    pub(crate) value: Vec<u8>,
    pub(crate) listening_mode_request_id: Option<u64>,
}

impl TrayCommand {
    pub(crate) fn control(identifier: ControlCommandIdentifiers, value: Vec<u8>) -> Self {
        Self {
            identifier,
            value,
            listening_mode_request_id: None,
        }
    }

    pub(crate) fn into_parts(self) -> (ControlCommandIdentifiers, Vec<u8>, Option<u64>) {
        (self.identifier, self.value, self.listening_mode_request_id)
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

    pub(crate) fn clear_pending_listening_mode_request(&mut self, request_id: u64) {
        if self
            .pending_listening_mode
            .is_some_and(|request| request.request_id == request_id)
        {
            self.pending_listening_mode = None;
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
        let icon = generate_icon(&text, text_mode, any_bud_charging);
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
                    if let Some(tx) = &this.command_tx {
                        let value = options_clone
                            .get(current)
                            .map(|&(_, val)| val)
                            .unwrap_or(0x02);
                        if tx
                            .send(TrayCommand::control(
                                ControlCommandIdentifiers::ListeningMode,
                                vec![value],
                            ))
                            .is_ok()
                        {
                            this.listening_mode = Some(value);
                        }
                    }
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

fn generate_icon(text: &str, text_mode: bool, charging: bool) -> Icon {
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
        let color = if charging {
            Rgba([0u8, 255u8, 0u8, 255u8])
        } else {
            Rgba([255u8, 255u8, 255u8, 255u8])
        };

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
                listening_mode_request_id: Some(u64::MAX),
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
                listening_mode_request_id: Some(0),
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
            .listening_mode_request_id
            .unwrap();
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
            .listening_mode_request_id
            .unwrap();
        let group = listening_mode_group(&tray);
        (group.select)(&mut tray, 2);
        let current_request_id = command_rx
            .try_recv()
            .unwrap()
            .listening_mode_request_id
            .unwrap();

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
    fn tray_tooltip_shows_nonzero_disconnected_level() {
        let mut tray = empty_tray();
        tray.battery_l = Some(79);
        tray.battery_l_status = Some(BatteryStatus::Disconnected);

        let tooltip = <MyTray as ksni::Tray>::tool_tip(&tray);

        assert!(tooltip.description.contains("L: 79%"));
    }
}
