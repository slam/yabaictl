use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::process::Command;
use std::time::Instant;
use structopt::clap::arg_enum;

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
