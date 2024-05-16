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

const GREEN_HP: Rgba<u8> = Rgba([48, 199, 141, 255]);
const RED_HP: Rgba<u8> = Rgba([210, 106, 92, 255]);


#[derive(Debug, Serialize, Deserialize)]
struct Config {
    max_hp: u32,
    min_hp: u32,
    signal_threshold: u32,
}

impl Config {
    fn save_into_file(&self) {
        let config_json = serde_json::to_string(&self).expect("Failed to serialize JSON");
        let mut file = File::create("default_screenserver.json").expect("Failed to create file");
        file.write_all(config_json.as_bytes()).expect("Failed to write to file");
    }
}

impl Default for Config {
    fn default() -> Self {
        if let Ok(mut file) = File::open("default_screenserver.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents).expect("Failed to read file");
            let config: Config = serde_json::from_str(&contents).expect("Failed to parse JSON");
            config
        } else {
            Config {
                max_hp: 0,
                min_hp: 0,
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


fn main() {
    let mut config = Config::default();
    // Prompt the user for max_hp and min_hp values
    print!("Enter max_hp ({}): ", config.max_hp);
    io::stdout().flush().unwrap();
    let mut max_hp = String::new();
    io::stdin().read_line(&mut max_hp).unwrap();
    let max_hp = max_hp.trim();
    if !max_hp.is_empty() {
        config.max_hp = max_hp.parse().unwrap();
    }

    print!("Enter min_hp ({}): ", config.min_hp);
    io::stdout().flush().unwrap();
    let mut min_hp = String::new();
    io::stdin().read_line(&mut min_hp).unwrap();
    let min_hp = min_hp.trim();
    if !min_hp.is_empty() {
        config.min_hp = min_hp.parse().unwrap();
    }
    config.save_into_file();

    let signal_threshold = config.min_hp * 100 / config.max_hp;
    println!("Signal threshold: {}", signal_threshold);
    let window_name = CString::new("OnTopReplica").expect("CString::new failed");
    let mut notified: u8 = 0;
    loop {
        let window: HWND = unsafe { FindWindowA(null_mut(), window_name.as_ptr()) };
        let mut rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        
        if window.is_null() {
            println!("Window '{}' not found", window_name.to_str().unwrap());
        } else {
            unsafe { GetWindowRect(window, &mut rect) };
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let screens = Screen::all().unwrap();
        let screen = screens.first().unwrap();
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> = if !window.is_null() {
            screen.capture_area(
                rect.left, rect.top, 
                (rect.right - rect.left) as u32, 
                (rect.bottom - rect.top) as u32
            ).unwrap()
        } else {
            screen.capture().unwrap()
        };

        let hp_bar_coords = match find_hp_bar_start(&image) {
            Some(hp_bar) => hp_bar,
            None => {
                println!("HP bar not found");
                continue;
            }
        };
        let hp_bar = get_hp_bar(&image, hp_bar_coords);
        let hp_percentage = hp_bar.iter().filter(|&x| *x == 1).count() as f32 / hp_bar.len() as f32 * 100.0;
        println!("HP percentage: {}", hp_percentage);

        if hp_percentage < signal_threshold as f32 && notified < 3 {
            println!("HP below threshold! Sending signal...");
            notified += 1;

            let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
            let sink = rodio::Sink::try_new(&handle).unwrap();
        
            let file = std::fs::File::open("alarm.wav").unwrap();
            sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());
        
            sink.sleep_until_end();
        } else if hp_percentage >= signal_threshold as f32{
            notified = 0;
        }
    }
}
