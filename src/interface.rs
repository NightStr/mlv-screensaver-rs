use std::{io::{self, Write}, sync::{Arc, RwLock}};
use crate::config::{CurrentState, MuteOptions, CurrentHpState};
use crossterm::{queue, cursor, terminal};
use indoc::indoc;

macro_rules! print_line {
    ($stdout:expr, $s:expr) => {
        queue!($stdout, cursor::MoveDown(1)).unwrap();
        queue!($stdout, cursor::MoveToColumn(0)).unwrap();
        queue!($stdout, terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
        print!("{}", $s);
    };
}


pub struct Interface {
    shared_app_state: Arc<RwLock<CurrentState>>,
    app_state: CurrentState,
    dynamic_part: String,
    static_part: String,
    stdout: io::Stdout,
    tick_rate: std::time::Duration,
}


impl Interface {
    pub fn new(
        shared_app_state: Arc<RwLock<CurrentState>>,
        tick_rate: std::time::Duration,
    ) -> Self {
        let app_state = *shared_app_state.read().unwrap();
        let dynamic_part = format!(indoc! {r#"
            Hp: {}
            OnTopReplica found: {}

            Mutted: {}
            Auto mod: {}
            Is thieveing active: {}
            "#}, 
            0, 
            app_state.on_top_replica_found,
            match app_state.is_mutted {
                MuteOptions::Mute => "Yes",
                MuteOptions::TempMute => "Temporarily",
                MuteOptions::Unmute => "No",
            },
            app_state.auto_control,
            match app_state.is_thieving_active {
                true => "Yes",
                false => "No",
            }
        );
        let static_part = format!(indoc! {r#"
        
            M|m: Mute on/off
            Esc|T|t: Temporarily mute on/off
            A|a: Auto mode on/off
            S|s: Temrorarily auto mode on/off
            B|b: Thiefing on/off
            Q|q: Quit"#
        });

        Interface {
            shared_app_state,
            tick_rate,
            app_state,
            dynamic_part,
            static_part,
            stdout: io::stdout(),
        }
    }
    
    pub fn draw(&self) {
        println!("\n{}{}", self.dynamic_part, self.static_part); 
    }

    pub fn update_app_state(&mut self) {
        self.app_state = *self.shared_app_state.read().unwrap();
    }

    pub fn update(&mut self) {
        self.draw();
        while self.app_state.is_running {
            self.update_app_state();
            let dynamic_part_lines = self.dynamic_part.lines().into_iter().count() as u16;
            let static_part_lines = self.static_part.lines().into_iter().count() as u16;

            queue!(self.stdout, cursor::MoveUp(dynamic_part_lines + static_part_lines + 1)).unwrap();

            print_line!(self.stdout, format!("Hp: {}", match self.app_state.hp {
                CurrentHpState::Hp(hp) => format!("{:.2}%", hp),
                CurrentHpState::BarNotFound => "HP bar not found".to_string(),
            }));

            print_line!(self.stdout, format!("OnTopReplica found: {}", self.app_state.on_top_replica_found));
            print_line!(self.stdout, "");
            print_line!(self.stdout, format!("Mutted: {}", match self.app_state.is_mutted {
                MuteOptions::Mute => "Yes",
                MuteOptions::TempMute => "Temporarily",
                MuteOptions::Unmute => "No",
            }));
            print_line!(self.stdout, format!("Auto mod: {}", self.app_state.auto_control));

            print_line!(self.stdout, format!("Is thieveing active: {}", match self.app_state.is_thieving_active {
                true => "Yes",
                false => "No",
            }));

            queue!(self.stdout, cursor::MoveDown(static_part_lines + 1)).unwrap();
            queue!(self.stdout, cursor::MoveToColumn(0)).unwrap();
            io::stdout().flush().unwrap();
            std::thread::sleep(self.tick_rate);
        }
    }
}
