use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
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
    Extra,
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

    loop {
        let start = Instant::now();
        let mut stream = UnixStream::connect(path.as_path())?;

        // Adjust timeouts to 10s. When a display is added or removed, yabai
        // could take a few seconds to return.
        stream.set_read_timeout(Some(Duration::new(10, 0)))?;
        stream.set_write_timeout(Some(Duration::new(10, 0)))?;

        stream.write_u32::<LittleEndian>(command.len().try_into().unwrap())?;
        stream.write_all(command.as_bytes())?;

        let mut buffer = Vec::new();
        let read = match stream.read_to_end(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                let duration = start.elapsed();
                match e.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        // Retry on this error:
                        //
                        //   Error: Resource temporarily unavailable (os error 35)
                        eprintln!("{:?} {:?} got {:?}, retrying", msgs, duration, e);
                        continue;
                    }
                    _ => {
                        bail!("{:?} {:?} {:?}", msgs, duration, e);
                    }
                }
            }
        };
        let duration = start.elapsed();
        eprintln!("{:?} {:?}", msgs, duration);

        if read == 0 {
            return Ok("".to_string());
        }
        if buffer[0] == YABAI_FAILURE_BYTE {
            bail!("{}", String::from_utf8(buffer[1..].to_vec())?);
        }
        let s = String::from_utf8(buffer)?;
        return Ok(s);
    }
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
    if space == "" {
        eprintln!("Not moving {} to an unlabeled space", window_id);
        return Ok(());
    }
    let r = yabai_message(&["window", &window_id.to_string(), "--space", space]);
    match r {
        Err(e) => {
            if !e
                .to_string()
                .contains(&"could not locate the window to act on!")
                && !e
                    .to_string()
                    .contains(&"is not a valid option for SPACE_SEL")
            {
                return Err(e);
            }
            eprintln!("Not moving {}. It no longer exists", window_id);
        }
        Ok(_) => {}
    }
    Ok(())
}

fn focus(space: &Space) -> Result<()> {
    focus_space_arg(&space.index.to_string())?;
    Ok(())
}

fn focus_space_by_label(label_index: u32) -> Result<()> {
    focus_space_arg(&format!("s{}", label_index))?;
    Ok(())
}

fn focus_space_arg(arg: &str) -> Result<()> {
    let r = yabai_message(&["space", "--focus", arg]);
    match r {
        Err(e) => {
            if !e
                .to_string()
                .contains(&"cannot focus an already focused space.")
            {
                return Err(e);
            }
        }
        Ok(_) => {}
    }
    Ok(())
}

fn move_space_to_display(space_index: u32, display_index: u32) -> Result<()> {
    let r = yabai_message(&[
        "space",
        &space_index.to_string(),
        "--display",
        &display_index.to_string(),
    ]);

    match r {
        Err(e) => {
            if !e
                .to_string()
                .contains(&"acting space is already located on the given display.")
            {
                return Err(e);
            }
        }
        Ok(_) => {}
    }
    Ok(())
}

fn neighbor_space(states: &YabaiStates, direction: WindowArg) -> Option<&Space> {
    let focused_space = states.focused_space().expect("No focused space found");
    let label_index = focused_space.label_index().expect("Invalid space label");

    // My main window is on the right
    let next_label_index = match direction {
        WindowArg::East => {
            if label_index % 2 == 0 {
                label_index - 1
            } else {
                label_index + 1
            }
        }
        WindowArg::West => {
            if label_index % 2 == 0 {
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

fn even_spaces(states: &YabaiStates) -> Result<()> {
    // Evenly split the spaces among the monitors
    match states.num_displays() {
        1 => {}
        2 | 3 => {
            for i in 1..=NUM_SPACES {
                if i <= NUM_SPACES / 2 {
                    move_space_to_display(i + 1, 1)?
                } else {
                    move_space_to_display(i + 1, 2)?
                }
            }
            if states.num_displays() > 2 {
                move_space_to_display(NUM_SPACES + 2, 3)?
            }
        }
        _ => {
            bail!(
                "Don't know how to handle {} monitors",
                states.num_displays()
            );
        }
    }
    Ok(())
}

fn ensure_spaces(states: &YabaiStates) -> Result<YabaiStates> {
    let layout = if states.num_displays() > 1 {
        "bsp"
    } else {
        "stack"
    };

    // Cycle through all the spaces and focus each one with a short delay.
    // This gives yabai enough time to pick up the most up-to-date states.
    // This is esp. important when yabai has just been reloaded, in which
    // case the windows array in every space is empty (except for the one
    // already in focus).
    let focused_space = states.focused_space().expect("No focused space");
    let sleep = Duration::from_millis(250);
    for space in states.spaces.iter() {
        focus(space)?;
        thread::sleep(sleep);
        yabai_message(&["space", "--layout", layout])?;
    }
    focus(focused_space)?;

    let states = query()?;
    // Add one for the unused Desktop 1. See comments in ensure_labels() for
    // more details.
    //
    // Display 3 and beyond have one desktop each.
    let target = NUM_SPACES + 1 + (states.num_displays() - 2);

    // Evenly distribute the spaces among displays to handle the edge
    // case where only one space is left to destroy (and that would fail).
    even_spaces(&states)?;
    if states.num_spaces() < target {
        for _i in states.num_spaces()..NUM_SPACES + 1 {
            yabai_message(&["space", "--create"])?;
        }
    } else if states.num_spaces() > target {
        for _i in target + 1..=states.num_spaces() {
            yabai_message(&["space", &(target + 1).to_string(), "--destroy"])?;
        }
    }
    // Now evenly distribute the spaces again after the creation/destruction.
    even_spaces(&states)?;

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
        2 | 3 => {
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
                } else if i <= NUM_SPACES {
                    label_space(
                        (i + 1).try_into()?,
                        &format!("s{}", (i - NUM_SPACES / 2) * 2 - 1),
                    )?;
                } else {
                    label_space((i + 1).try_into()?, &format!("s{}", NUM_SPACES + 1))?;
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
    let states = restore_spaces_core(states)?;
    states::save_yabai(&states)?;
    Ok(())
}

fn restore_spaces_core(states: YabaiStates) -> Result<YabaiStates> {
    let states = ensure_spaces(&states)?;
    let states = ensure_labels(&states)?;
    let states = reorganize_spaces(&states)?;
    // Probably a yabai bug somehwere. When this is called by yabai on a signal
    // of the display_added event, sending a window to a different space
    // sometimes doesn't take effect. So, here we run it twice.
    let states = reorganize_spaces(&states)?;
    Ok(states)
}

fn restore_if_necessary(states: YabaiStates) -> Result<YabaiStates> {
    if states.find_unlabeled_space().is_none() {
        return Ok(states);
    }
    eprintln!("Restoring spaces");
    let states = restore_spaces_core(states)?;
    Ok(states)
}

pub fn focus_space(space: SpaceArg) -> Result<()> {
    let states = query()?;
    let states = restore_if_necessary(states)?;

    let focused_space = states.focused_space().expect("No focused space found");
    let focused_label_index = focused_space.label_index().expect("Invalid space label");
    let display_count = if states.num_displays() >= 2 { 2 } else { 1 };
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
            let index = focused_label_index + display_count;
            if index > NUM_SPACES {
                index % NUM_SPACES
            } else {
                index
            }
        }
        SpaceArg::Prev => {
            if focused_label_index <= display_count {
                let extra_monitors = if states.num_displays() > 2 {
                    states.num_displays() - 2
                } else {
                    0
                };
                states.num_spaces() - 1 /* reserved */ - extra_monitors - (display_count - focused_label_index)
            } else {
                focused_label_index - display_count
            }
        }
        SpaceArg::Extra => 11,
        SpaceArg::Space(number) => number,
    };
    eprintln!("focus_space: label_index={}", label_index);
    match states.num_displays() {
        1 => {
            focus_space_by_label(label_index)?;
        }
        2 | 3 => {
            // This is to bring both desktops to focus
            let neighbor_label_index = match label_index % 2 {
                0 => label_index - 1,
                _ => label_index + 1,
            };
            let neighbor_space = states.find_space_by_label_index(neighbor_label_index);
            match neighbor_space {
                None => {}
                Some(neighbor_space) => {
                    // Skip bringing the other screen to focus if it is already in focus or visible
                    if focused_label_index != neighbor_label_index && !neighbor_space.is_visible {
                        focus_space_by_label(neighbor_label_index)?;
                    }
                }
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
        recent: focused_label_index,
    };
    states::save_yabaictl(ctl)?;
    let states = query()?;
    states::save_yabai(&states)?;
    Ok(())
}

pub fn operate_window(op: WindowOp, direction: WindowArg) -> Result<()> {
    let states = query()?;
    let states = restore_if_necessary(states)?;

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
                "could not locate a {}ward managed window.",
                direction.as_str()
            );
            // This is the error when the space has no windows
            let expected2 = "could not locate the selected window.";
            if !e_str.contains(&expected1) && !e_str.contains(&expected2) {
                return Err(e);
            }

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
                2 | 3 => {
                    let neighbor_space = neighbor_space(&states, direction);
                    let neighbor_space = match neighbor_space {
                        None => {
                            return Err(e);
                        }
                        Some(space) => space,
                    };

                    match op {
                        WindowOp::Focus => {
                            let next_window = match direction {
                                WindowArg::East => neighbor_space.first_window,
                                WindowArg::West => neighbor_space.last_window,
                                _ => {
                                    return Err(e);
                                }
                            };
                            let next_window = if next_window == 0
                                // Sometimes yabai's first-window and
                                // last-window states get stale.  Verify that
                                // the window is still in the windows array for
                                // the space. If it is not, most likely the
                                // space is empty with a hidden window or two.
                                || neighbor_space.find_window_id(&next_window).is_none()
                            {
                                let space = states.focused_space().expect("No focused space found");
                                match direction {
                                    WindowArg::East => space.first_window,
                                    WindowArg::West => space.last_window,
                                    _ => {
                                        return Err(e);
                                    }
                                }
                            } else {
                                next_window
                            };
                            eprintln!("next_window={}", next_window);
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
