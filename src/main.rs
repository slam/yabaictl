#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use anyhow::{bail, Result};
use std::convert::TryInto;
use structopt::StructOpt;

use crate::yabai::{SpaceArg, WindowArg, WindowOp};

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
        #[structopt(parse(try_from_str = parse_space_arg),
         help="[a space number, next, prev, recent]")]
        space: SpaceArg,
    },
    FocusWindow {
        #[structopt(possible_values = &WindowArg::variants(), case_insensitive = true)]
        direction: WindowArg,
    },
    SwapWindow {
        #[structopt(possible_values = &WindowArg::variants(), case_insensitive = true)]
        direction: WindowArg,
    },
    WarpWindow {
        #[structopt(possible_values = &WindowArg::variants(), case_insensitive = true)]
        direction: WindowArg,
    },
}

fn main() -> Result<()> {
    match Cli::from_args() {
        Cli::FocusWindow { direction } => yabai::operate_window(WindowOp::Focus, direction)?,
        Cli::SwapWindow { direction } => yabai::operate_window(WindowOp::Swap, direction)?,
        Cli::WarpWindow { direction } => yabai::operate_window(WindowOp::Warp, direction)?,
        Cli::FocusSpace { space } => yabai::focus_space(space)?,
        Cli::RestoreSpaces {} => yabai::restore_spaces()?,
    }

    Ok(())
}

fn parse_space_arg(src: &str) -> Result<SpaceArg> {
    match src {
        "next" => return Ok(SpaceArg::Next),
        "prev" => return Ok(SpaceArg::Prev),
        "recent" => return Ok(SpaceArg::Recent),
        _ => {
            let space = u32::from_str_radix(src, 10)?;
            if space == 0 || space > yabai::NUM_SPACES {
                bail!("Space {} out of range", space);
            }
            return Ok(SpaceArg::Space(space.try_into()?));
        }
    }
}
