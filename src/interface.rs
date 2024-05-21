use std::{io::{self, Write}, sync::{Arc, RwLock}};
use crate::config::{CurrentState, MuteOptions, CurrentHpState, AutoControlMode};
use crossterm::{queue, cursor, terminal, event};
use crossterm::event::{Event, KeyCode, KeyEvent};
use indoc::indoc;

macro_rules! print_line {
    ($stdout:expr, $s:expr) => {
        queue!($stdout, cursor::MoveDown(1)).unwrap();
        queue!($stdout, cursor::MoveToColumn(0)).unwrap();
        queue!($stdout, terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
        print!("{}", $s);
    };
}


pub struct DisplayInterface {
    shared_app_state: Arc<RwLock<CurrentState>>,
    app_state: CurrentState,
    dynamic_part: String,
    static_part: String,
    stdout: io::Stdout,
    tick_rate: std::time::Duration,
}


impl DisplayInterface {
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

        DisplayInterface {
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


pub struct KeyboardKeyPressProcessor {
    shared_app_state: Arc<RwLock<CurrentState>>,
    app_state: CurrentState,
}


impl KeyboardKeyPressProcessor {
    pub fn new(shared_app_state: Arc<RwLock<CurrentState>>) -> Self {
        let app_state = *shared_app_state.read().unwrap();
        KeyboardKeyPressProcessor {
            shared_app_state,
            app_state,
        }
    }

    fn update_app_state(&mut self) {
        self.app_state.update_from(&self.shared_app_state.read().unwrap());
    }

    fn process_event(&self, event: KeyEvent) {
        if event.kind == event::KeyEventKind::Release {
            return;
        };

        match event.code {
            KeyCode::Char('M' | 'm' | 'Ь' | 'ь') => {
                let is_mutted = {
                    self.shared_app_state.write().unwrap().is_mutted.clone()
                };
                match is_mutted {
                    MuteOptions::Mute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::Unmute;
                    },
                    MuteOptions::TempMute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::Mute;
                    },
                    MuteOptions::Unmute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::Mute;
                    }
                }
            }
            KeyCode::Char('T' | 't' | 'Е' | 'е') | KeyCode::Esc => {
                let is_mutted = {
                    self.shared_app_state.write().unwrap().is_mutted.clone()
                };
                match is_mutted {
                    MuteOptions::Mute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::TempMute;
                    },
                    MuteOptions::TempMute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::Unmute;
                    },
                    MuteOptions::Unmute => {
                        self.shared_app_state.write().unwrap().is_mutted = MuteOptions::TempMute;
                    }
                }
            }
            KeyCode::Char('A' | 'a' | 'Ф' | 'ф') => {
                let auto_control = {
                    self.shared_app_state.write().unwrap().auto_control.clone()
                };
                match auto_control {
                    AutoControlMode::Off => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::On;
                    },
                    AutoControlMode::On => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::Off;
                    },
                    AutoControlMode::Temporarily => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::On;
                    }
                };
            }
            KeyCode::Char('S' | 's' | 'Ы' | 'ы') => {
                let auto_control = {
                    self.shared_app_state.write().unwrap().auto_control.clone()
                };
                match auto_control {
                    AutoControlMode::Off => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::Temporarily;
                    },
                    AutoControlMode::On => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::Temporarily;
                    },
                    AutoControlMode::Temporarily => {
                        self.shared_app_state.write().unwrap().auto_control = AutoControlMode::Off;
                    }
                };
            }
            KeyCode::Char('B' | 'b' | 'И' | 'и') => {
                let is_thiving_active = {
                    self.shared_app_state.read().unwrap().is_thieving_active
                };
                self.shared_app_state.write().unwrap().is_thieving_active = !is_thiving_active;
            }
            KeyCode::Char('Q' | 'q' | 'Й' | 'й') => {
                self.shared_app_state.write().unwrap().is_running = false;
                println!("Exiting...");
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        while self.app_state.is_running {
            if let Event::Key(event) = event::read().unwrap() {
                self.update_app_state();
                self.process_event(event);
            }
        }
    }
}
