use anyhow::Result;

use structopt::clap::arg_enum;
use structopt::StructOpt;

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

fn update_spaces() -> Result<()> {
    println!("update_spaces");
    Ok(())
}
