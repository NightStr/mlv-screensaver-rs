use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use screenshots::Screen;
use screenshots::image::{ImageBuffer, Rgba};
use std::io::{self, Write};
use rodio;
use std::io::BufReader;
use std::ffi::CString;
use std::ptr::null_mut;
use winapi::shared::windef::{HWND, RECT};
use winapi::um::winuser::{FindWindowA, GetWindowRect};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Read;
use serde_json;
use ctrlc;
use std::thread;
use crossterm::{event, queue, cursor, terminal};
use crossterm::event::{Event, KeyCode};
use indoc::indoc;

const GREEN_HP: Rgba<u8> = Rgba([48, 199, 141, 255]);
const RED_HP: Rgba<u8> = Rgba([210, 106, 92, 255]);


enum NeedNotification {
    Yes(f32),
    No(f32),
    HpBarNotFound,
}


#[derive(Debug, PartialEq)]
enum MuteOptions {
    Mute,
    TempMute,
    Unmute,
}

impl Default for MuteOptions {
    fn default() -> Self {
        MuteOptions::Unmute
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Config {
    max_hp: u32,
    min_hp: u32,
    volume: f32,
    signal_threshold: u32,
}

impl Config {
    fn save_into_file(&self) {
        let config_json = serde_json::to_string(&self).expect("Failed to serialize JSON");
        let mut file = File::create("default_screenserver.json").expect("Failed to create file");
        file.write_all(config_json.as_bytes()).expect("Failed to write to file");
    }

    fn load_from_file() -> Result<Self, String> {
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


impl Drop for Config {
    fn drop(&mut self) {
        self.save_into_file();
    }
}


#[derive(Debug)]
enum CurrentHpState {
    Hp(f32),
    BarNotFound,
}

impl Default for CurrentHpState {
    fn default() -> Self {
        CurrentHpState::Hp(0.0)
    }
}


#[derive(Debug, Default)]
struct CurrentState {
    hp: CurrentHpState,
    on_top_replica_found: bool,
    is_mutted: MuteOptions,
}


fn find_hp_bar_start(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Option<[u32; 2]> {
    for w in 0..image.width() {
        for h in 0..image.height() {
            let pixel = image.get_pixel(w, h);
            if *pixel == GREEN_HP || *pixel == RED_HP{
                return Some([w, h]);
            }
        }
    }
    None
}


fn get_hp_bar(image: &ImageBuffer<Rgba<u8>, Vec<u8>>, hp_bar_coords: [u32; 2]) -> Vec<u8> {
    let hp_bar_height = hp_bar_coords[1];
    let mut hp_bar_width = hp_bar_coords[0];
    let mut hp_bar: Vec<u8> = Vec::new();
    while hp_bar_width < image.width() {
        let pixel = *image.get_pixel(hp_bar_width, hp_bar_height);
        if pixel == GREEN_HP {
            hp_bar.push(1);
        }
        if pixel == RED_HP {
            hp_bar.push(0);
        }
        hp_bar_width += 1;
    }
    hp_bar
}

fn get_geometry(window_name: &CString) -> Option<RECT> {
    let window: HWND = unsafe { FindWindowA(null_mut(), window_name.as_ptr()) };
    let mut rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    
    if !window.is_null() {
        unsafe { GetWindowRect(window, &mut rect) };
        Some(rect)
    } else {
        None
    }
}


fn get_screen_image(geometry: Option<RECT>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let screens = Screen::all().unwrap();
    let screen = screens.first().unwrap();
    
    if let Some(rect) = geometry {
        screen.capture_area(
            rect.left + 5, rect.top + 10, 
            ((rect.right - rect.left) - 10) as u32, 
            ((rect.bottom - rect.top) - 18) as u32
        ).unwrap()
    } else {
        screen.capture().unwrap()
    }
}

fn check_notification_needed(signal_threshold: f32, image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> NeedNotification {
    let hp_bar_coords = match find_hp_bar_start(&image) {
        Some(hp_bar) => hp_bar,
        None => {
            return NeedNotification::HpBarNotFound;
        }
    };
    let hp_bar = get_hp_bar(&image, hp_bar_coords);
    let hp_percentage = hp_bar.iter().filter(|&x| *x == 1).count() as f32 / hp_bar.len() as f32 * 100.0;

    if hp_percentage < signal_threshold {
        NeedNotification::Yes(hp_percentage)
    } else {
        NeedNotification::No(hp_percentage)
    }
}


fn beep_beep(volume: f32) {
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();

    let file = std::fs::File::open("alarm.wav").unwrap();
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


fn main() {
    let config = get_config();
    let current_state = Arc::new(RwLock::new(CurrentState::default()));
    println!("Run with config: {:?}", config);
    let window_name = CString::new("OnTopReplica").expect("CString::new failed");
    
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
        let r = running.clone();
        move || {
        while r.load(Ordering::SeqCst) {
            let mut sleep_duration = std::time::Duration::from_millis(1000);
            let geometry = get_geometry(&window_name);
            match &geometry {
                Some(_) => {
                    current_state_clone.write().unwrap().on_top_replica_found = true;
                },
                None => {
                    current_state_clone.write().unwrap().on_top_replica_found = false;
                }
            };
            let image = get_screen_image(geometry);
            match check_notification_needed(config.signal_threshold as f32, &image) {
                NeedNotification::Yes(hp) => {
                    current_state_clone.write().unwrap().hp = CurrentHpState::Hp(hp);
                    if current_state_clone.read().unwrap().is_mutted == MuteOptions::Unmute {
                        beep_beep(config.volume);
                        sleep_duration = std::time::Duration::from_millis(3000);
                    }
                },
                NeedNotification::No(hp) => {
                    current_state_clone.write().unwrap().hp = CurrentHpState::Hp(hp);
                    if current_state_clone.read().unwrap().is_mutted == MuteOptions::TempMute {
                        current_state_clone.write().unwrap().is_mutted = MuteOptions::Unmute;
                    }
                },
                NeedNotification::HpBarNotFound => {
                    current_state_clone.write().unwrap().hp = CurrentHpState::BarNotFound;
                }
            };
            std::thread::sleep(sleep_duration);
        }
    }});
    let interface_handler = thread::spawn({
        let current_state_clone = current_state.clone();
        let r = running.clone();
        let mut stdout = io::stdout();
        {
            println!("");
            let current_state = current_state_clone.read().unwrap();
            println!(indoc! {
                    r#"
                    Hp: {}
                    OnTopReplica found: {}
                    Mutted: {}

                    M|m: Mute
                    Esc|T|t: Temporarily mute
                    U|u: Unmute
                    Q|q: Quit
                    "#
                }, 
                match current_state.hp {
                    CurrentHpState::Hp(hp) => format!("{:.2}%", hp),
                    CurrentHpState::BarNotFound => "HP bar not found".to_string(),
                }, 
                current_state.on_top_replica_found,
                match current_state.is_mutted {
                    MuteOptions::Mute => "Yes",
                    MuteOptions::TempMute => "Temporarily",
                    MuteOptions::Unmute => "No",
                }
            );
        }
        move || {
            while r.load(Ordering::SeqCst) {
                queue!(stdout, cursor::MoveUp(9)).unwrap();
                queue!(stdout, cursor::MoveToColumn(0)).unwrap();
                queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
                print!("Hp: {}", match current_state_clone.read().unwrap().hp {
                    CurrentHpState::Hp(hp) => format!("{:.2}%", hp),
                    CurrentHpState::BarNotFound => "HP bar not found".to_string(),
                });
                queue!(stdout, cursor::MoveDown(1)).unwrap();
                queue!(stdout, cursor::MoveToColumn(0)).unwrap();
                queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
                print!("OnTopReplica found: {}", current_state_clone.read().unwrap().on_top_replica_found);
                queue!(stdout, cursor::MoveDown(1)).unwrap();
                queue!(stdout, cursor::MoveToColumn(0)).unwrap();
                queue!(stdout, terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
                print!("Mutted: {}", 
                    match current_state_clone.read().unwrap().is_mutted {
                        MuteOptions::Mute => "Yes",
                        MuteOptions::TempMute => "Temporarily",
                        MuteOptions::Unmute => "No",
                    }
                );
                queue!(stdout, cursor::MoveDown(7)).unwrap();
                queue!(stdout, cursor::MoveToColumn(0)).unwrap();
                std::io::stdout().flush().unwrap();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    });

    while running.load(Ordering::SeqCst) {
        if let Event::Key(event) = event::read().unwrap() {
            // println!("{:?}", event);
            match event.code {
                KeyCode::Char('M' | 'm') => {
                    current_state.write().unwrap().is_mutted = MuteOptions::Mute;
                }
                KeyCode::Char('T' | 't') | KeyCode::Esc => {
                    current_state.write().unwrap().is_mutted = MuteOptions::TempMute;
                }
                KeyCode::Char('U' | 'u') => {
                    current_state.write().unwrap().is_mutted = MuteOptions::Unmute;
                }
                KeyCode::Char('Q' | 'q') => {
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
