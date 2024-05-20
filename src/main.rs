use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::io::{self, Write, BufReader};
use std::fs::File;
use std::thread;

use mlv_screensaver::config::{AutoControlMode, Config, CurrentHpState, CurrentState, MuteOptions};
use mlv_screensaver::interface::Interface;
use mlv_screensaver::hp::HpBarFinder;
use rodio;
use ctrlc;
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use enigo::{Button, Coordinate, Enigo, Mouse, Settings, Direction::{Press, Release}};

const MOUSE_COORDS: [i32; 2] = [820, 790];


fn beep_beep(volume: f32, file: File) {
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();

    sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());

    sink.set_volume(volume);
    sink.sleep_until_end();
}


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


fn move_mouse_and_click(x: i32, y: i32, mouse_button: Button, sleep_duration: std::time::Duration) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let start = std::time::Instant::now();
    while std::time::Instant::now() - start < sleep_duration {
        thread::yield_now();
    }
    enigo.move_mouse(x, y, Coordinate::Abs).unwrap();
    enigo.button(mouse_button, Press).unwrap();
    thread::sleep(std::time::Duration::from_millis(20));
    enigo.button(mouse_button, Release).unwrap();
}


fn main() {
    let config = get_config();
    let current_state = Arc::new(RwLock::new(CurrentState::default()));
    println!("Run with config: {:?}", config);
    
    let running = Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let r = running.clone();
        move || {
            println!("Exiting...");
            r.store(false, Ordering::SeqCst);
        }
    }).expect("Error setting Ctrl-C handler");

    let work_handler = thread::spawn({
        let current_state_clone = current_state.clone();
        let mut current_state_local = *current_state_clone.read().unwrap();
        let r = running.clone();
        let mut hp_founder = HpBarFinder::new("OnTopReplica");
        move || {
        let mut hight_hp_notified = false;
        while r.load(Ordering::SeqCst) {
            let mut sleep_duration = std::time::Duration::from_millis(1000);
            let current_hp = hp_founder.get_hp();
            current_state_clone.write().unwrap().on_top_replica_found = hp_founder.window_was_found();
            
            current_state_local.update_from(&current_state_clone.read().unwrap());
            if let CurrentHpState::Hp(hp) = &current_hp {
                if *hp < config.signal_threshold as f32 {
                    hight_hp_notified = false;
                    match current_state_local.auto_control {
                        AutoControlMode::On if current_state_local.is_thieving_active == true => {
                            current_state_clone.write().unwrap().is_thieving_active = false;
                            move_mouse_and_click(
                                MOUSE_COORDS[0], MOUSE_COORDS[1], Button::Left, 
                                std::time::Duration::from_secs(3)
                            );
                        }
                        AutoControlMode::Temporarily if current_state_local.is_thieving_active == true => {
                            current_state_clone.write().unwrap().is_thieving_active = false;
                            move_mouse_and_click(
                                MOUSE_COORDS[0], MOUSE_COORDS[1], Button::Left, 
                                std::time::Duration::from_secs(3)
                            );
                            current_state_clone.write().unwrap().auto_control = AutoControlMode::Off;
                        }
                        _ => {},
                    };
                    if current_state_local.is_mutted == MuteOptions::Unmute {
                        beep_beep(config.volume, std::fs::File::open("low_hp.wav").unwrap());
                        sleep_duration = std::time::Duration::from_millis(3000);
                    }
                } else if *hp > config.signal_threshold as f32 && current_state_local.is_mutted == MuteOptions::TempMute {
                    current_state_clone.write().unwrap().is_mutted = MuteOptions::Unmute;
                } else if *hp >= 99.0 {
                    match current_state_local.auto_control {
                        AutoControlMode::On | AutoControlMode::Temporarily if current_state_local.is_thieving_active == false => {
                            current_state_clone.write().unwrap().is_thieving_active = true;
                            move_mouse_and_click(
                                MOUSE_COORDS[0], MOUSE_COORDS[1], Button::Left, std::time::Duration::default()
                            );
                        }
                        _ => {},
                    };
                    if hight_hp_notified == false {
                        hight_hp_notified = true;
                        beep_beep(config.volume, std::fs::File::open("hight_hp.wav").unwrap());
                    }
                }
            };
            current_state_clone.write().unwrap().hp = current_hp;
            std::thread::sleep(sleep_duration);
        }
    }});
    let interface_handler = thread::spawn({
        let mut interface = Interface::new(current_state.clone());
        interface.draw();
        let r = running.clone();
        move || {
            while r.load(Ordering::SeqCst) {
                interface.update();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    });

    while running.load(Ordering::SeqCst) {
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
                KeyCode::Char('G' | 'g' | 'П' | 'п') => {
                    thread::sleep(std::time::Duration::from_millis(2000));
                    move_mouse_and_click(MOUSE_COORDS[0], MOUSE_COORDS[1], Button::Left, std::time::Duration::default());
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
