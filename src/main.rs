use std::sync::{Arc, RwLock};
use std::io::{self, Write};
use std::thread;
use ctrlc;

use mlv_screensaver::config::{Config, CurrentState};
use mlv_screensaver::interface::{DisplayInterface, KeyboardKeyPressProcessor};
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
    println!("Run with config: {:?}", config);
    ctrlc::set_handler({
        let current_state = current_state.clone();
        move || {
            println!("Exiting...");
            current_state.write().unwrap().is_running = false;
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
    let mut display = DisplayInterface::new(
        current_state.clone(),
        std::time::Duration::from_millis(200)
    );
    let mut keyboard_processor = KeyboardKeyPressProcessor::new(current_state.clone());
    let work_handler = thread::spawn(move || {auto_control.run()});
    let interface_handler = thread::spawn(move || { display.update()});

    keyboard_processor.update();
    work_handler.join().unwrap();
    interface_handler.join().unwrap();
}
