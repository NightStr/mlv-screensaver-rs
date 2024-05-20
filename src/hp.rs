use std::{ffi::CString, ptr::null_mut};

use screenshots::{image::{ImageBuffer, Rgba}, Screen};
use winapi::{shared::windef::{HWND, RECT}, um::winuser::{FindWindowA, GetWindowRect}};

use crate::config::CurrentHpState;

const GREEN_HP: Rgba<u8> = Rgba([48, 199, 141, 255]);
const RED_HP: Rgba<u8> = Rgba([210, 106, 92, 255]);

pub struct HpBarFinder {
    window_name: CString,
    geometry: Option<RECT>,
}

impl HpBarFinder {
    pub fn new(window_name: &str) -> Self {
        HpBarFinder { 
            window_name: CString::new(window_name).unwrap(),
            geometry: None,
        }
    }
    
    fn get_geometry(&self) -> Option<RECT> {
        let window: HWND = unsafe { FindWindowA(null_mut(), self.window_name.as_ptr()) };
        let mut rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        
        if !window.is_null() {
            unsafe { GetWindowRect(window, &mut rect) };
            Some(rect)
        } else {
            None
        }
    }
    
    fn find_hp_bar_start(&self, image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Option<[u32; 2]> {
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

    fn get_screen_image(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let screens = Screen::all().unwrap();
        let screen = screens.first().unwrap();
        
        if let Some(rect) = self.geometry {
            screen.capture_area(
                rect.left + 5, rect.top + 10, 
                ((rect.right - rect.left) - 10) as u32, 
                ((rect.bottom - rect.top) - 18) as u32
            ).unwrap()
        } else {
            screen.capture().unwrap()
        }
    }

    pub fn window_was_found(&self) -> bool {
        self.geometry.is_some()
    }
    
    fn get_hp_bar(&mut self) -> Option<Vec<u8>> {
        self.geometry = self.get_geometry();
        let image = self.get_screen_image();
        
        let bar_start;
        if let Some(coords) = self.find_hp_bar_start(&image) {
            bar_start = coords;
        } else {
            return None;
        }
        let hp_bar_height = bar_start[1];
        let mut hp_bar_width = bar_start[0];
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
        Some(hp_bar)
    }

    pub fn get_hp(&mut self) -> CurrentHpState {
        match self.get_hp_bar() {
            Some(hp_bar) => {
                CurrentHpState::Hp(hp_bar.iter().filter(|&x| *x == 1).count() as f32 / hp_bar.len() as f32 * 100.0)
            }
            None => CurrentHpState::BarNotFound,
        }
    }
}
