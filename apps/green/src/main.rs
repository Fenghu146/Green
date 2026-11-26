// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod attach;
mod autosave;
#[cfg(test)]
mod bench;
mod blocks;
mod capture;
mod clip;
mod compare;
mod count;
mod desk;
mod engine;
mod export;
mod finding;
mod folder;
mod keys;
mod licenses;
mod look;
mod output;
mod outside;
mod paths;
mod places;
mod plugins;
mod portable;
mod remote;
mod rename;
mod restore;
mod side;
mod text;
mod tools;
mod toolbar;
mod view;
mod viewstate;

use ely_platform::AppId;

fn main() {
    portable::adopt();
    if let Some(code) = capture::command_line() {
        std::process::exit(code);
    }
    if let Some(code) = remote::command_line() {
        std::process::exit(code);
    }
    ely_ui::run(AppId::Mark, |_, _| {}, desk::open, None);
}
