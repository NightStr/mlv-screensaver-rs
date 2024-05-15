use screenshots::Screen;
use screenshots::image::{ImageBuffer, Rgba};
use std::io::{self, Write};
use rodio::Sink;
use std::io::BufReader;


const GREEN_HP: Rgba<u8> = Rgba([48, 199, 141, 255]);
const RED_HP: Rgba<u8> = Rgba([210, 106, 92, 255]);


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
    // Prompt the user for max_hp and min_hp values
    print!("Enter max_hp: ");
    io::stdout().flush().unwrap();
    let mut max_hp = String::new();
    io::stdin().read_line(&mut max_hp).unwrap();
    let max_hp: u32 = max_hp.trim().parse().unwrap();

    print!("Enter min_hp: ");
    io::stdout().flush().unwrap();
    let mut min_hp = String::new();
    io::stdin().read_line(&mut min_hp).unwrap();
    let min_hp: u32 = min_hp.trim().parse().unwrap();

    let signal_threshold = min_hp * 100 / max_hp;
    println!("Signal threshold: {}", signal_threshold);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let screens = Screen::all().unwrap();
        let screen = screens.first().unwrap();
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> = screen.capture().unwrap();
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

        if hp_percentage < signal_threshold as f32 {
            // Perform signal action
            // Perform signal action
            println!("HP below threshold! Sending signal...");

            // Load the audio file
            let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
            let sink = rodio::Sink::try_new(&handle).unwrap();
        
            let file = std::fs::File::open("alarm.wav").unwrap();
            sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());
        
            sink.sleep_until_end();
        }
    }
}
