#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use anyhow::{bail, Result};
use structopt::StructOpt;

use crate::yabai::Direction;

mod states;
mod yabai;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "yabaictl",
    about = "A yabai wrapper for better multi-display support ."
)]
enum Cli {
    RestoreSpaces {},
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
        Cli::RestoreSpaces {} => restore_spaces()?,
    }

    Ok(())
}

fn focus_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    bail!("Not implemented yet")
}

fn swap_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    bail!("Not implemented yet")
}

fn warp_window(direction: Direction) -> Result<()> {
    println!("{:?}", direction);
    bail!("Not implemented yet")
}

fn focus_space(space: u32) -> Result<()> {
    println!("{:?}", space);
    bail!("Not implemented yet")
}

fn restore_spaces() -> Result<()> {
    let states = states::query()?;
    println!("yabai query returned {:?}", states);
    states::save_yabai(states)?;

    println!("load_yabaictl returned {:?}", states::load_yabaictl()?,);
    println!("load_yabai returned {:?}", states::load_yabai()?,);
    Ok(())
}
