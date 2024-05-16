use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

const GREEN_HP: Rgba<u8> = Rgba([48, 199, 141, 255]);
const RED_HP: Rgba<u8> = Rgba([210, 106, 92, 255]);


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

fn get_screen_image(window_name: &CString) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let window: HWND = unsafe { FindWindowA(null_mut(), window_name.as_ptr()) };
    let mut rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    
    if window.is_null() {
        println!("Window '{}' not found", window_name.to_str().unwrap());
    } else {
        unsafe { GetWindowRect(window, &mut rect) };
    }
    let screens = Screen::all().unwrap();
    let screen = screens.first().unwrap();
    
    if !window.is_null() {
        screen.capture_area(
            rect.left, rect.top, 
            (rect.right - rect.left) as u32, 
            (rect.bottom - rect.top) as u32
        ).unwrap()
    } else {
        screen.capture().unwrap()
    }
}

fn check_notification_needed(signal_threshold: f32, image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> bool {
    let hp_bar_coords = match find_hp_bar_start(&image) {
        Some(hp_bar) => hp_bar,
        None => {
            println!("HP bar not found");
            return false;
        }
    };
    let hp_bar = get_hp_bar(&image, hp_bar_coords);
    let hp_percentage = hp_bar.iter().filter(|&x| *x == 1).count() as f32 / hp_bar.len() as f32 * 100.0;
    println!("HP percentage: {}", hp_percentage);

    if hp_percentage < signal_threshold {
        true
    } else {
        false
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
    println!("Run with config: {:?}", config);
    let window_name = CString::new("OnTopReplica").expect("CString::new failed");
    
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("Exiting...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut notified: u8 = 0;
    while running.load(Ordering::SeqCst) {
        let mut sleep_duration = std::time::Duration::from_millis(1000);
        let image = get_screen_image(&window_name);
        let need_notification = check_notification_needed(config.signal_threshold as f32, &image);
        if need_notification && notified < 3 {
            println!("HP below threshold! Sending signal...");
            notified += 1;
            beep_beep(config.volume);
            sleep_duration = std::time::Duration::from_millis(3000);
        } else if !need_notification {
            notified = 0;
        }
        std::thread::sleep(sleep_duration);
    }
}
