use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use chip8_core;
use std::{fs::File, io::Read, path::Path};

struct Chip8App {
    chip8_core: chip8_core::Chip8Core,
}

struct KeyInput {
    key: Option<chip8_core::Key>,
}

fn input_keys(mut key_input: ResMut<KeyInput>, keyboard_input: Res<Input<KeyCode>>) {
    key_input.key = if keyboard_input.pressed(KeyCode::W) {
        Some(chip8_core::Key::TWO)
    } else if keyboard_input.pressed(KeyCode::A) {
        Some(chip8_core::Key::FOUR)
    } else if keyboard_input.pressed(KeyCode::X) {
        Some(chip8_core::Key::EIGHT)
    } else if keyboard_input.pressed(KeyCode::D) {
        Some(chip8_core::Key::SIX)
    } else if keyboard_input.pressed(KeyCode::S) {
        Some(chip8_core::Key::FIVE)
    } else {
        None
    };
}

fn display(mut egui_context: ResMut<EguiContext>, chip8_app: Res<Chip8App>) {
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

fn tick_emulator(mut chip8_app: ResMut<Chip8App>, key_input: Res<KeyInput>) {
    // chip8_app.chip8_core.out_log(); // logging

    chip8_app.chip8_core.tick(key_input.key);
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

    let key_input = KeyInput { key: None };

    App::new()
        .insert_resource(chip8_app)
        .insert_resource(key_input)
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_system(tick_emulator)
        .add_system(display)
        .add_system(input_keys)
        .run();
}
