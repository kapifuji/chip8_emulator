use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use chip8_core;
use std::{fs::File, io::Read, path::Path};

pub struct Chip8App {
    chip8_core: chip8_core::Chip8Core,
}

fn emulator_loop(mut egui_context: ResMut<EguiContext>, mut chip8_app: ResMut<Chip8App>) {
    // logging
    chip8_app.chip8_core.out_log();

    // input

    // tick CPU
    chip8_app.chip8_core.tick(None);

    // output
    let disp_data = chip8_app.chip8_core.get_display_data();
    let mut out_data = String::new();

    for row in disp_data {
        let mut row_str = String::new();
        for pixel in row {
            if pixel == true {
                row_str += "**";
            } else {
                row_str += "__";
            }
        }
        row_str += "\n";
        out_data += row_str.as_str();
    }

    egui::CentralPanel::default().show(egui_context.ctx_mut(), |ui| {
        ui.label(out_data);
    });
}

fn main() {
    let path = Path::new("./rom/IBM Logo.ch8");
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => panic!("Failed to open rom"),
    };

    let mut rom_bytes: Vec<u8> = Vec::new();
    for byte in file.bytes() {
        rom_bytes.push(byte.unwrap());
    }

    let chip8_core = chip8_core::Chip8Core::new(rom_bytes);
    chip8_core.run();

    let chip8_app = Chip8App {
        chip8_core: chip8_core,
    };

    App::new()
        .insert_resource(chip8_app)
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_system(emulator_loop)
        .run();
}
