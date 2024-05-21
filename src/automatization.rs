use std::sync::{Arc, RwLock};
use std::thread;
use enigo::{Button, Coordinate, Enigo, Mouse, Settings, Direction::{Press, Release}};
use rodio;

use crate::config::{AutoControlMode, Config, CurrentHpState, CurrentState, MuteOptions};
use crate::hp::HpBarFinder;

pub struct Notifier{
    low_hp_alert: &'static str,
    high_hp_alert: &'static str,
    volume: f32
}

impl Notifier {
    pub fn new(volume: f32, low_hp_alert: &'static str, high_hp_alert: &'static str) -> Self {
        Notifier{
            low_hp_alert,
            high_hp_alert,
            volume
        }
    }

    fn notify(&self, file: std::fs::File) -> Result<(), &str> {
        let (_stream, handle) = rodio::OutputStream::try_default().expect(
            "Failed to get default output stream"
        );
        let sink = rodio::Sink::try_new(&handle).unwrap();
        sink.append(
            rodio::Decoder::new(
                std::io::BufReader::new(file)
            ).expect("Failed to create decoder")
        );

        sink.set_volume(self.volume);
        sink.sleep_until_end();
        Ok(())
    }

    pub fn low_hp_notify(&mut self) -> Result<(), &str> {
        self.notify(
            std::fs::File::open(self.low_hp_alert).expect("Failed to open low hp file")
        )
    }

    pub fn high_hp_notify(&mut self) -> Result<(), &str> {
        self.notify(
            std::fs::File::open(self.high_hp_alert).expect("Failed to open high hp file")
        )
    }
}

pub struct AutoClicker{
    enigo: Enigo
}

impl AutoClicker {
    pub fn new() -> Result<Self, &'static str> {
        Ok(AutoClicker{
            enigo: Enigo::new(&Settings::default()).expect("Failed to create enigo")
        })
    }

    pub fn click(&mut self, x: i32, y: i32, mouse_button: Button, sleep_duration: std::time::Duration) {
        thread::sleep(sleep_duration);
        self.enigo.move_mouse(x, y, Coordinate::Abs).unwrap();
        self.enigo.button(mouse_button, Press).unwrap();
        thread::sleep(std::time::Duration::from_millis(20));
        self.enigo.button(mouse_button, Release).unwrap();
    }
}


pub struct AutoControl {
    auto_clicker: AutoClicker,
    notifier: Notifier,
    hp_bar_finder: HpBarFinder,
    config: Config,
    shared_app_state: Arc<RwLock<CurrentState>>,
    app_state: CurrentState,
    high_hp_notified: bool,
    tick_rate: std::time::Duration,
    thieving_switch_button_coords: [i32; 2],
}

impl AutoControl {
    pub fn new(
        shared_app_state: Arc<RwLock<CurrentState>>,
        config: Config,
        thieving_switch_button_coords: [i32; 2],
        low_hp_alert: &'static str,
        high_hp_alert: &'static str,
        tick_rate: std::time::Duration
    ) -> Result<Self, &'static str> {
        let auto_clicker = AutoClicker::new()?;
        let notifier = Notifier::new(config.volume, low_hp_alert, high_hp_alert);
        let hp_bar_finder = HpBarFinder::new("Old School RuneScape");
        let app_state = *shared_app_state.read().unwrap();

        Ok(AutoControl{
            auto_clicker,
            notifier,
            hp_bar_finder,
            config,
            shared_app_state,
            app_state,
            high_hp_notified: false,
            tick_rate,
            thieving_switch_button_coords
        })
    }

    pub fn stop_thieving(&mut self) {
        if self.app_state.auto_control == AutoControlMode::Temporarily && self.app_state.is_thieving_active == true {
            self.shared_app_state.write().unwrap().auto_control = AutoControlMode::Off;
        }
        if (
            self.app_state.auto_control == AutoControlMode::Temporarily ||
                self.app_state.auto_control == AutoControlMode::On
        ) && self.app_state.is_thieving_active == true {
            self.auto_clicker.click(
                self.thieving_switch_button_coords[0],
                self.thieving_switch_button_coords[1],
                Button::Left,
                std::time::Duration::from_secs(3)
            );
            self.shared_app_state.write().unwrap().is_thieving_active = false;
            self.app_state.is_thieving_active = false;
        }

    }

    fn start_thieving(&mut self) {
        match self.app_state.auto_control {
            AutoControlMode::On | AutoControlMode::Temporarily if self.app_state.is_thieving_active == false => {
                self.shared_app_state.write().unwrap().is_thieving_active = true;
                self.app_state.is_thieving_active = true;
                self.auto_clicker.click(
                    self.thieving_switch_button_coords[0],
                    self.thieving_switch_button_coords[1],
                    Button::Left,
                    std::time::Duration::default()
                );
            }
            _ => {},
        };
    }

    pub fn run(&mut self) {
        while self.app_state.is_running {
            let mut sleep_duration = self.tick_rate;
            self.app_state.update_from(&self.shared_app_state.read().unwrap());
            let current_hp = self.hp_bar_finder.get_hp();

            if let CurrentHpState::Hp(hp) = &current_hp {
                if *hp >= 99.0 {
                    self.start_thieving();
                    if self.high_hp_notified == false {
                        self.high_hp_notified = true;
                        self.notifier.high_hp_notify().unwrap();
                    }
                }
                if *hp < self.config.signal_threshold as f32 {
                    self.stop_thieving();
                    self.high_hp_notified = false;
                    if self.app_state.is_muted == MuteOptions::Unmute {
                        self.notifier.low_hp_notify().unwrap();
                        sleep_duration = std::time::Duration::from_millis(3000);
                    }
                }
                if *hp < self.config.signal_threshold as f32 &&
                    self.app_state.is_muted == MuteOptions::TempMute
                {
                    self.shared_app_state.write().unwrap().is_muted = MuteOptions::Unmute;
                }
            };
            self.shared_app_state.write().unwrap().hp = current_hp;
            thread::sleep(sleep_duration);
        }
    }
}