#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use structopt::clap::arg_enum;
use structopt::StructOpt;

#[derive(Serialize, Deserialize, Debug)]
struct YabaictlStates {
    recent: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct YabaiStates {
    spaces: Vec<Space>,
    displays: Vec<Display>,
    windows: Vec<Window>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Space {
    id: u32,
    label: String,
    index: u32,
    display: u32,
    windows: Vec<u32>,
    r#type: String,
    visible: u32,
    focused: u32,
    #[serde(rename = "native-fullscreen")]
    native_fullscreen: u32,
    #[serde(rename = "first-window")]
    first_window: u32,
    #[serde(rename = "last-window")]
    last_window: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Display {
    id: u32,
    uuid: String,
    index: u32,
    spaces: Vec<u32>,
    frame: Frame,
    location: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Frame {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Window {
    id: u32,
    pid: u32,
    app: String,
    title: String,
    frame: Frame,
    level: u32,
    role: String,
    movable: u32,
    resizable: u32,
    display: u32,
    space: u32,
    visible: u32,
    focused: u32,
    split: String,
    floating: u32,
    sticky: u32,
    minimized: u32,
    topmost: u32,
    opacity: f32,
    shadow: u32,
    border: u32,
    #[serde(rename = "stack-index")]
    stack_index: u32,
    #[serde(rename = "zoom-parent")]
    zoom_parent: u32,
    #[serde(rename = "zoom-fullscreen")]
    zoom_fullscreen: u32,
    #[serde(rename = "native-fullscreen")]
    native_fullscreen: u32,
}

arg_enum! {
    #[derive(Debug)]
    enum Direction {
        North,
        East,
        South,
        West,
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "yabaictl",
    about = "A yabai wrapper for better multi-display support ."
)]
enum Cli {
    UpdateSpaces {},
    FocusSpace {
        space: u32,
    },
    FocusWindow {
        #[structopt(possible_values = &Direction::variants(), case_insensitive = true)]
        direction: Direction,
    },
    SwapWindow {
        #[structopt(possible_values = &Direction::variants(), case_insensitive = true)]
        direction: Direction,
    },
    WarpWindow {
        #[structopt(possible_values = &Direction::variants(), case_insensitive = true)]
        direction: Direction,
    },
}

fn main() -> Result<()> {
    match Cli::from_args() {
        Cli::FocusWindow { direction } => focus_window(direction)?,
        Cli::SwapWindow { direction } => swap_window(direction)?,
        Cli::WarpWindow { direction } => warp_window(direction)?,
        Cli::FocusSpace { space } => focus_space(space)?,
        Cli::UpdateSpaces {} => update_spaces()?,
    }

    Ok(())
}

fn focus_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    Ok(())
}

fn swap_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    Ok(())
}

fn warp_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    Ok(())
}

fn focus_space(space: u32) -> Result<()> {
    println!("{:?}", space);
    Ok(())
}

fn load_yabaictl_states() -> Result<YabaictlStates> {
    let file =
        File::open("/Users/slam/.cache/yabaictl").context("Failed to load yabaictl states")?;
    let reader = BufReader::new(file);
    let yabaictl: YabaictlStates = serde_json::from_reader(reader)?;
    Ok(yabaictl)
}

fn load_yabai_states() -> Result<YabaiStates> {
    let file = File::open("/Users/slam/.cache/yabai").context("Failed to load yabai states")?;
    let reader = BufReader::new(file);
    let yabai: YabaiStates = serde_json::from_reader(reader)?;
    Ok(yabai)
}

fn update_spaces() -> Result<()> {
    println!(
        "update_spaces {:?} {:?}",
        load_yabaictl_states()?,
        load_yabai_states()?
    );
    Ok(())
}
