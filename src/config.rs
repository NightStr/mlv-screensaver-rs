use serde::{Deserialize, Serialize};
use serde_json;
use std::{fmt::Display, fs::File, io::{Read, Write}};


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Config {
    pub max_hp: u32,
    pub min_hp: u32,
    pub volume: f32,
    pub signal_threshold: u32,
}

impl Config {
    pub fn save_into_file(&self) {
        let config_json = serde_json::to_string(&self).expect("Failed to serialize JSON");
        let mut file = File::create("default_screenserver.json").expect("Failed to create file");
        file.write_all(config_json.as_bytes()).expect("Failed to write to file");
    }

    pub fn load_from_file() -> Result<Self, String> {
        let mut file = File::open("default_screenserver.json").map_err(|e| e.to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
        let config: Config = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        if let Ok(config) = Config::load_from_file() {
            config
        } else {
            Config {
                max_hp: 0,
                min_hp: 0,
                volume: 1.0,
                signal_threshold: 0,
            }
        }
    }
}


#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CurrentHpState {
    Hp(f32),
    BarNotFound,
}

impl Default for CurrentHpState {
    fn default() -> Self {
        CurrentHpState::Hp(0.0)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum MuteOptions {
    Mute,
    TempMute,
    #[default]
    Unmute,
}


#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum AutoControlMode {
    On,
    #[default]
    Off,
    Temporarily
}

impl Display for AutoControlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoControlMode::On => write!(f, "On"),
            AutoControlMode::Off => write!(f, "Off"),
            AutoControlMode::Temporarily => write!(f, "Temporarily"),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct CurrentState {
    pub hp: CurrentHpState,
    pub on_top_replica_found: bool,
    pub is_muted: MuteOptions,
    pub auto_control: AutoControlMode,
    pub is_thieving_active: bool,
    pub is_running: bool,
}

impl Default for CurrentState {
    fn default() -> Self {
        CurrentState {
            hp: CurrentHpState::default(),
            on_top_replica_found: false,
            is_muted: MuteOptions::default(),
            auto_control: AutoControlMode::default(),
            is_thieving_active: false,
            is_running: true,
        }
    }
}

impl From <&CurrentState> for CurrentState {
    fn from(other: &CurrentState) -> Self {
        CurrentState {
            hp: other.hp,
            on_top_replica_found: other.on_top_replica_found,
            is_muted: other.is_muted,
            auto_control: other.auto_control,
            is_thieving_active: other.is_thieving_active,
            is_running: other.is_running,
        }
    }
}

impl CurrentState {
    pub fn update_from(&mut self, other: &CurrentState) {
        self.hp = other.hp;
        self.on_top_replica_found = other.on_top_replica_found;
        self.is_muted = other.is_muted;
        self.auto_control = other.auto_control;
        self.is_thieving_active = other.is_thieving_active;
        self.is_running = other.is_running;
    }
}

