use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::convert::TryInto;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use structopt::clap::arg_enum;

use crate::states::{self, Display, Space, Window, YabaiStates, YabaictlStates};

pub const NUM_SPACES: u32 = 10;
const YABAI_FAILURE_BYTE: u8 = 0x07;

arg_enum! {
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub enum WindowArg {
        North,
        East,
        South,
        West,
    }
}

impl WindowArg {
    pub fn as_str(&self) -> &'static str {
        match *self {
            WindowArg::North => "north",
            WindowArg::East => "east",
            WindowArg::South => "south",
            WindowArg::West => "west",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WindowOp {
    Focus,
    Swap,
    Warp,
}

impl WindowOp {
    pub fn as_str(&self) -> &'static str {
        match *self {
            WindowOp::Focus => "--focus",
            WindowOp::Swap => "--swap",
            WindowOp::Warp => "--warp",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SpaceArg {
    Next,
    Prev,
    Recent,
    Space(u32),
}

#[derive(Debug)]
pub enum QueryDomain {
    Windows,
    Spaces,
    Displays,
}

impl QueryDomain {
    pub fn as_str(&self) -> &'static str {
        match *self {
            QueryDomain::Windows => "--windows",
            QueryDomain::Spaces => "--spaces",
            QueryDomain::Displays => "--displays",
        }
    }
}

pub fn yabai_message(msgs: &[&str]) -> Result<String> {
    let mut command = String::new();
    for msg in msgs.iter() {
        command.push_str(msg);
        command.push('\0');
    }
    command.push('\0');

    let user = std::env::var("USER")?;
    let path = PathBuf::from(format!("/tmp/yabai_{}.socket", user));

    let start = Instant::now();
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::new(1, 0)))?;
    stream.set_write_timeout(Some(Duration::new(1, 0)))?;

    stream.write_all(command.as_bytes())?;

    let sleep = Duration::from_millis(1);
    thread::sleep(sleep);

    let mut buffer = Vec::new();
    let read = stream.read_to_end(&mut buffer)?;
    let duration = start.elapsed();
    eprintln!("{:?} {:?}", msgs, duration);

    if read == 0 {
        return Ok("".to_string());
    }
    if buffer[0] == YABAI_FAILURE_BYTE {
        bail!("{}", String::from_utf8(buffer[1..].to_vec())?);
    }
    let s = String::from_utf8(buffer)?;

    Ok(s)
}

pub fn yabai_query<T>(param: QueryDomain) -> Result<T>
where
    T: DeserializeOwned,
{
    let command = &["query", param.as_str()];
    loop {
        let raw = yabai_message(command)?;
        if raw == "" {
            // Retry the query if yabai returns an empty string.
            //
            // We might be sending commands too fast to yabai. It
            // might not be able to handle the rapid fire series
            // of commands straight into the unix socket.
            eprintln!("{:?} returned an empty string, retrying", command);
            continue;
        }
        let json: T = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to deserialize JSON: {}", raw))?;
        return Ok(json);
    }
}

pub fn query() -> Result<YabaiStates> {
    let windows: Vec<Window> =
        yabai_query(QueryDomain::Windows).context("Failed to query yabai for the window states")?;
    let displays: Vec<Display> = yabai_query(QueryDomain::Displays)
        .context("Failed to query yabai for the display states")?;
    let spaces: Vec<Space> =
        yabai_query(QueryDomain::Spaces).context("Failed to query yabai for the space states")?;
    let states = YabaiStates {
        windows,
        displays,
        spaces,
    };
    Ok(states)
}

fn label_space(space_index: u32, label: &str) -> Result<()> {
    yabai_message(&["space", &space_index.to_string(), "--label", label])?;
    Ok(())
}

fn move_window_to_space(window_id: &u32, space: &str) -> Result<()> {
    yabai_message(&["window", &window_id.to_string(), "--space", space])?;
    Ok(())
}

fn focus_space_by_label(label_index: u32) -> Result<()> {
    yabai_message(&["space", "--focus", &format!("s{}", label_index)])?;
    Ok(())
}

fn neighbor_space(states: &YabaiStates, direction: WindowArg) -> Option<&Space> {
    let focused_space = states.focused_space().expect("No focused space found");
    let label_index = focused_space.label_index().expect("Invalid space label");

    // My main window is on the right
    let next_label_index = match direction {
        WindowArg::East => {
            if label_index % states.num_displays() == 0 {
                label_index - 1
            } else {
                label_index + 1
            }
        }
        WindowArg::West => {
            if label_index % states.num_displays() == 0 {
                label_index - 1
            } else {
                label_index + 1
            }
        }
        _ => {
            return None;
        }
    };

    states.find_space_by_label_index(next_label_index)
}

fn ensure_spaces(states: &YabaiStates) -> Result<YabaiStates> {
    // Add one for the unused Desktop 1. See comments in ensure_labels() for
    // more details.
    let target = NUM_SPACES + 1;

    if states.num_spaces() < target {
        for _i in states.num_spaces()..NUM_SPACES + 1 {
            yabai_message(&["space", "--create"])?;
        }
    } else if states.num_spaces() > target {
        for _i in target + 1..=states.num_spaces() {
            yabai_message(&["space", &(target + 1).to_string(), "--destroy"])?;
        }
    }
    Ok(query()?)
}

fn ensure_labels(states: &YabaiStates) -> Result<YabaiStates> {
    // Desktop 1 is reserved. We don't put anything there because of this apple
    // issue:
    //
    // https://github.com/koekeishiya/yabai/discussions/238#discussioncomment-193399
    label_space(1, "reserved")?;

    match states.num_displays() {
        1 => {
            // One monitor is easy. Just label Desktop 2 as s1, D3 as s2, D4 as
            // s3, and so on. (Again, as mentioned above, we leave Desktop 1
            // unused to get around a quirk in MacOS).
            for i in 1..states.num_spaces() {
                label_space((i + 1).try_into()?, &format!("s{}", i))?;
            }
        }
        2 => {
            // This is the arrangement for two monitors with the one on the
            // right as primary:
            //
            // Right monitor:
            //
            // reserved s2 s4 s6 s8 s10 <= yabai space labels
            // Desktop1 D2 D3 D4 D5 D6  <= MacOS Desktop
            //
            // Left monitor:
            //
            // s1 s3 s5 s7  s9
            // D7 D8 D9 D10 D11
            //
            // With this arrangement, s1 and s2 form a single composite desktop,
            // so are s3 and s4, s5 and s6, and so on.
            //
            // The `focus_space` subcommand would switch two monitors in unison
            // as a single desktop.
            for i in 1..states.num_spaces() {
                if i <= NUM_SPACES / 2 {
                    label_space((i + 1).try_into()?, &format!("s{}", i * 2))?;
                } else {
                    label_space(
                        (i + 1).try_into()?,
                        &format!("s{}", (i - NUM_SPACES / 2) * 2 - 1),
                    )?;
                }
            }
        }
        _ => {
            bail!(
                "Don't know how to handle {} monitors",
                states.num_displays()
            );
        }
    }
    Ok(query()?)
}

fn reorganize_spaces(states: &YabaiStates) -> Result<YabaiStates> {
    let old_states = states::load_yabai()?;

    for space in old_states.spaces.iter() {
        for window_id in space.windows.iter() {
            if space.label == "reserved" {
                move_window_to_space(window_id, "s1")?;
            } else {
                if states
                    .find_window_id_in_space(&space.label, window_id)
                    .is_none()
                {
                    move_window_to_space(window_id, &space.label)?;
                }
            }
        }
    }

    Ok(query()?)
}

pub fn restore_spaces() -> Result<()> {
    let states = query()?;
    let states = ensure_spaces(&states)?;
    let states = ensure_labels(&states)?;
    let states = reorganize_spaces(&states)?;
    states::save_yabai(&states)?;
    Ok(())
}

pub fn focus_space(space: SpaceArg) -> Result<()> {
    let states = query()?;
    let focused_space = states.focused_space().expect("No focused space found");
    let label_index = focused_space.label_index().expect("Invalid space label");
    let label_index = match space {
        SpaceArg::Recent => {
            let ctl = states::load_yabaictl()?;
            if ctl.recent > states.num_spaces() {
                bail!(
                    "recent space {} > number of spaces {}",
                    ctl.recent,
                    states.num_spaces()
                )
            }
            ctl.recent
        }
        SpaceArg::Next => {
            let index = label_index + states.num_displays();
            if index > NUM_SPACES {
                index % NUM_SPACES
            } else {
                index
            }
        }
        SpaceArg::Prev => {
            if label_index <= states.num_displays() {
                states.num_spaces() - 1 /* reserved */ - (states.num_displays() - label_index)
            } else {
                label_index - states.num_displays()
            }
        }
        SpaceArg::Space(number) => number,
    };
    match states.num_displays() {
        1 => {
            focus_space_by_label(label_index)?;
        }
        2 => {
            // This is to bring both desktops to focus
            if label_index % 2 == 0 {
                focus_space_by_label(label_index - 1)?;
            } else {
                focus_space_by_label(label_index + 1)?;
            }
            focus_space_by_label(label_index)?;
        }
        _ => {
            bail!(
                "Don't know how to handle {} monitors",
                states.num_displays()
            );
        }
    }

    let ctl = &YabaictlStates {
        recent: label_index,
    };
    states::save_yabaictl(ctl)?;
    let states = query()?;
    states::save_yabai(&states)?;
    Ok(())
}

pub fn operate_window(op: WindowOp, direction: WindowArg) -> Result<()> {
    let r = yabai_message(&["window", op.as_str(), direction.as_str()]);
    match r {
        Err(e) => {
            match direction {
                WindowArg::East => {}
                WindowArg::West => {}
                _ => {
                    return Err(e);
                }
            }
            let e_str = e.to_string();
            let expected1 = format!(
                "could not locate a {}ward managed window.\n",
                direction.as_str()
            );
            // This is the error is the space has no windows
            let expected2 = "could not locate the selected window.\n";
            if e_str != expected1 && e_str != expected2 {
                return Err(e);
            }

            let states = query()?;
            match states.num_displays() {
                1 => {
                    let space = states.focused_space().expect("No focused space found");
                    let next_window = match direction {
                        WindowArg::East => space.first_window,
                        WindowArg::West => space.last_window,
                        _ => {
                            return Err(e);
                        }
                    };
                    yabai_message(&["window", op.as_str(), &next_window.to_string()])?;
                }
                2 => {
                    let neighbor_space = neighbor_space(&states, direction);
                    let neighbor_space = match neighbor_space {
                        None => {
                            return Err(e);
                        }
                        Some(space) => space,
                    };

                    match op {
                        WindowOp::Focus => {
                            let space = match neighbor_space.windows.len() {
                                0 => states.focused_space().expect("No focused space found"),
                                _ => neighbor_space,
                            };
                            let next_window = match direction {
                                WindowArg::East => space.first_window,
                                WindowArg::West => space.last_window,
                                _ => {
                                    return Err(e);
                                }
                            };
                            yabai_message(&["window", op.as_str(), &next_window.to_string()])?;
                        }
                        WindowOp::Swap | WindowOp::Warp => {
                            if neighbor_space.windows.len() == 0 {
                                // If the neighbor space is empty, just send the
                                // window there
                                yabai_message(&["window", "--space", &neighbor_space.label])?;
                            } else {
                                let next_window = match direction {
                                    WindowArg::East => neighbor_space.first_window,
                                    WindowArg::West => neighbor_space.last_window,
                                    _ => {
                                        return Err(e);
                                    }
                                };
                                yabai_message(&["window", op.as_str(), &next_window.to_string()])?;
                            }

                            yabai_message(&["space", "--focus", &neighbor_space.label])?;
                        }
                    };
                }
                _ => {
                    bail!(
                        "Don't know how to handle {} monitors",
                        states.num_displays()
                    );
                }
            }
        }
        Ok(_) => {}
    }
    let states = query()?;
    states::save_yabai(&states)?;
    Ok(())
}
