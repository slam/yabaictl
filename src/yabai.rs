use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
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
    let args = vec!["query", param.as_str()];

    let raw = yabai_message(&args)?;
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

fn ensure_spaces(states: YabaiStates) -> Result<YabaiStates> {
    for _i in states.num_spaces()..=NUM_SPACES {
        yabai_message(&["space", "--create"])?;
    }
    Ok(query()?)
}

pub fn restore_spaces() -> Result<()> {
    let states = query()?;
    let states = ensure_spaces(states)?;
    states::save_yabai(&states)?;

    println!("load_yabaictl returned {:?}", states::load_yabaictl()?,);
    println!("load_yabai returned {:?}", states::load_yabai()?,);

    println!("yabai query returned {:?}", states);
    Ok(())
}
