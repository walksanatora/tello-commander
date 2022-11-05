#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::derivable_impls,unused_must_use)]

mod drone;

use eframe::egui;
use std::{fs::{read_to_string,write}, sync::{Mutex, Arc}};
use futures::executor::block_on;
use std::time::Duration;
use rfd::FileDialog;

#[tokio::main]
async fn main() {
    let options = eframe::NativeOptions::default();

    let drone_mutex = Arc::new(Mutex::new(vec![]));
    let app = MyApp{
        code: "command\ntakeoff\ndelay 5\nland".to_string(),
        run: false,
        drones: drone_mutex,
        drone_idx: 0,
        pass_errors: true
    };

    eframe::run_native(
        "Drone Commander",
        options,
        Box::new(|_cc| Box::new(app)),
    );
}

struct MyApp {
    code: String, // the current code of the application
    run: bool, // whether or not to run the program on all drones
    pass_errors: bool, // whether errors should be passed or crashed
    drones: Arc<Mutex<Vec<drone::Drone>>>, // a list of drones TODO: make it be a Drone object
    drone_idx: usize // the index of the selected drone
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            code: "command\ntakeoff\ndelay 5\nland".to_string(),
            run: false,
            pass_errors: false,
            drones: Arc::new(Mutex::new(vec![])),
            drone_idx: 0
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let drones = self.drones.lock().unwrap();
        egui::TopBottomPanel::top("tp").show(ctx,|ui|{
            egui::menu::bar(ui,|ui|{
                ui.menu_button("File", |ui|{
                    if ui.button("Save").clicked() {
                        if let Some(save) = FileDialog::new().save_file() {
                            write(save,self.code.clone());
                        };
                    };
                    if ui.button("Open").clicked() {
                        if let Some(open) = FileDialog::new().pick_file() {
                            if let Ok(cde) = read_to_string(open){
                                self.code = cde
                            };
                        };
                    };
                });
                ui.menu_button("Settings",|ui|{
                    ui.menu_button("Error handling", |ui|{
                        ui.selectable_value(&mut self.pass_errors, false, "Crash");
                        ui.selectable_value(&mut self.pass_errors, true, "Pass");
                    });
                });
            })
        });
        egui::SidePanel::left("lp").show(ctx,|ui| {
            egui::ScrollArea::vertical().show(ui,|ui|{
                ui.horizontal(|ui|{
                    self.run = ui.button("RUN").clicked();
                });
                ui.collapsing("Drones",|ui|{
                    if drones.is_empty() {
                        ui.label("No Drones Found");
                    } else {
                        for (idx, drone) in drones.iter().enumerate() {
                            ui.horizontal(|ui|{
                                ui.label(idx.to_string());
                                ui.selectable_value(&mut self.drone_idx, idx, drone.id.clone());
                            });
                        }
                    }
                });
            });    
        });
        egui::CentralPanel::default().show(ctx,|ui|{
            egui::ScrollArea::vertical().show(ui, |ui|{
                let txb = egui::TextEdit::multiline(&mut self.code)
                    .code_editor();
                ui.add_sized(ui.available_size(), txb);
            });
        });
        if self.run {
            println!("running:\n{}\non: all",self.code);
            for command in self.code.split('\n') {
                //Break early if line is none or empty or comment
                if command.starts_with('#') || command.is_empty() || command  == "\n" {
                    continue
                }
                // delay command which is not a SDK command
                if command.starts_with("delay") {
                    if let Some(num) = command.split(' ').nth(2){
                        let n = num.to_string().parse::<usize>();
                        if let Ok(nm) = n {std::thread::sleep(Duration::from_secs(nm as u64))}
                        else {std::thread::sleep(Duration::from_secs(1))};
                    } else {std::thread::sleep(Duration::from_secs(1))}
                    continue
                } else if command.starts_with("await") {
                    for drone in drones.iter() {
                        drone.await_blocks();
                    }
                }

                let is_blocking = command.contains('@');

                if command.contains('>') {
                    // push command to one drones queue
                    let split: Vec<&str> = command.splitn(2,'>').collect();
                    let num = split[0].chars().filter(|x|x.is_alphanumeric()).collect::<String>().parse::<usize>().unwrap();
                    let comma: String = split[1].chars().filter(|x|{x.is_alphanumeric()||x==&' '}).collect();
                    if num < drones.len(){
                        block_on(drones[num].add_command(drone::SdkCommand{
                            cmd: comma,
                            blocking: is_blocking
                        }));
                    };
                } else {
                    // push commands to all drones queue
                    let comma: String = command.chars().filter(|x|{x.is_alphanumeric()||x==&' '}).collect();
                    for drone in drones.iter() {
                        block_on(drone.add_command(drone::SdkCommand{
                            cmd: comma.clone(),
                            blocking: is_blocking
                        }));
                    };
                };

            }
        }
    }
}