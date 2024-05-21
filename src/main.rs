use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::io::{self, Write};
use std::thread;

use mlv_screensaver::config::{AutoControlMode, Config, CurrentState, MuteOptions};
use mlv_screensaver::interface::Interface;
use rodio;
use ctrlc;
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use mlv_screensaver::automatization::AutoControl;

const MOUSE_COORDS: [i32; 2] = [820, 790];


fn get_config() -> Config {
    let mut config = Config::default();
    // Prompt the user for max_hp and min_hp values
    print!("Enter max_hp ({}): ", config.max_hp);
    io::stdout().flush().unwrap();
    let mut input_buffer = String::new();
    io::stdin().read_line(&mut input_buffer).unwrap();
    if !input_buffer.trim().is_empty() {
        config.max_hp = input_buffer.trim().parse().unwrap();
    }

    print!("Enter min_hp ({}): ", config.min_hp);
    io::stdout().flush().unwrap();
    
    input_buffer.clear();
    io::stdin().read_line(&mut input_buffer).unwrap();
    if !input_buffer.trim().is_empty() {
        config.min_hp = input_buffer.trim().parse().unwrap();
    }

    print!("Enter volume 0.0-1.0 ({}): ", config.volume);
    io::stdout().flush().unwrap();

    input_buffer.clear();
    io::stdin().read_line(&mut input_buffer).unwrap();
    if !input_buffer.trim().is_empty() {
        config.volume = input_buffer.trim().parse().unwrap();
    }

    config.signal_threshold = config.min_hp * 100 / config.max_hp;
    config.save_into_file();
    config
}


fn main() {
    let config = get_config();
    let current_state = Arc::new(RwLock::new(CurrentState::default()));
    let mut local_state = CurrentState::from(current_state.read().unwrap());
    println!("Run with config: {:?}", config);
    let running = Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let r = running.clone();
        move || {
            println!("Exiting...");
            r.store(false, Ordering::SeqCst);
        }
    }).expect("Error setting Ctrl-C handler");

    let mut auto_control = AutoControl::new(
        current_state.clone(),
        config,
        MOUSE_COORDS,
        "Low hp",
        "High hp",
        std::time::Duration::from_millis(1000)
    ).unwrap();
    let mut interface = Interface::new(
        current_state.clone(),
        std::time::Duration::from_millis(200)
    );
    let work_handler = thread::spawn(move || {auto_control.run()});
    let interface_handler = thread::spawn(move || {interface.update()});

    while local_state.is_running {
        local_state.update_from(&current_state.read().unwrap());
        if let Event::Key(event) = event::read().unwrap() {
            if event.kind == event::KeyEventKind::Release {
                continue;
            };
            match event.code {
                KeyCode::Char('M' | 'm' | 'Ь' | 'ь') => {
                    let is_mutted = {
                        current_state.write().unwrap().is_mutted.clone()
                    };
                    match is_mutted {
                        MuteOptions::Mute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::Unmute;
                        },
                        MuteOptions::TempMute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::Mute;
                        },
                        MuteOptions::Unmute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::Mute;
                        }
                    }
                }
                KeyCode::Char('T' | 't' | 'Е' | 'е') | KeyCode::Esc => {
                    let is_mutted = {
                        current_state.write().unwrap().is_mutted.clone()
                    };
                    match is_mutted {
                        MuteOptions::Mute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::TempMute;
                        },
                        MuteOptions::TempMute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::Unmute;
                        },
                        MuteOptions::Unmute => {
                            current_state.write().unwrap().is_mutted = MuteOptions::TempMute;
                        }
                    }
                }
                KeyCode::Char('A' | 'a' | 'Ф' | 'ф') => {
                    let auto_control = {
                        current_state.write().unwrap().auto_control.clone()
                    };
                    match auto_control {
                        AutoControlMode::Off => {
                            current_state.write().unwrap().auto_control = AutoControlMode::On;
                        },
                        AutoControlMode::On => {
                            current_state.write().unwrap().auto_control = AutoControlMode::Off;
                        },
                        AutoControlMode::Temporarily => {
                            current_state.write().unwrap().auto_control = AutoControlMode::On;
                        }
                    };
                }
                KeyCode::Char('S' | 's' | 'Ы' | 'ы') => {
                    let auto_control = {
                        current_state.write().unwrap().auto_control.clone()
                    };
                    match auto_control {
                        AutoControlMode::Off => {
                            current_state.write().unwrap().auto_control = AutoControlMode::Temporarily;
                        },
                        AutoControlMode::On => {
                            current_state.write().unwrap().auto_control = AutoControlMode::Temporarily;
                        },
                        AutoControlMode::Temporarily => {
                            current_state.write().unwrap().auto_control = AutoControlMode::Off;
                        }
                    };
                }
                KeyCode::Char('B' | 'b' | 'И' | 'и') => {
                    let is_thiving_active = {
                        current_state.read().unwrap().is_thieving_active
                    };
                    current_state.write().unwrap().is_thieving_active = !is_thiving_active;
                }
                KeyCode::Char('Q' | 'q' | 'Й' | 'й') => {
                    running.store(false, Ordering::SeqCst);
                    println!("Exiting...");
                }
                _ => {}
            }
        }
    }
    work_handler.join().unwrap();
    interface_handler.join().unwrap();
}
