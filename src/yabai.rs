use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::process::Command;
use std::time::Instant;
use structopt::clap::arg_enum;

use crate::states;
use crate::states::{Display, Space, Window, YabaiStates};

const NUM_SPACES: usize = 10;

arg_enum! {
    #[derive(Debug)]
    pub enum Direction {
        North,
        East,
        South,
        West,
    }
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

pub fn yabai_message<T: AsRef<OsStr>>(msgs: &[T]) -> Result<String> {
    let mut command = Command::new("yabai");
    command.arg("-m");
    for msg in msgs.into_iter() {
        command.arg(msg);
    }

    let start = Instant::now();
    let output = command.output()?;
    let duration = start.elapsed();
    eprintln!("{:?} {:?}", command, duration);

    if !output.status.success() {
        let err = String::from_utf8(output.stderr)?;
        bail!("Failed to execute yabai: {}", err);
    }
    let s = String::from_utf8(output.stdout)?;
    Ok(s)
}

pub fn yabai_query<T>(param: QueryDomain) -> Result<T>
where
    T: DeserializeOwned,
{
    let raw = yabai_message(&["query", param.as_str()])?;
    let json: T = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to deserialize JSON: {}", raw))?;
    Ok(json)
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

fn ensure_spaces(states: YabaiStates) -> Result<YabaiStates> {
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

fn ensure_labels(states: YabaiStates) -> Result<YabaiStates> {
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

pub fn restore_spaces() -> Result<()> {
    let states = query()?;
    let states = ensure_spaces(states)?;
    let states = ensure_labels(states)?;
    states::save_yabai(&states)?;

    println!("load_yabaictl returned {:?}", states::load_yabaictl()?,);
    println!("load_yabai returned {:?}", states::load_yabai()?,);

    println!("yabai query returned {:?}", states);
    Ok(())
}
