use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaictlStates {
    recent: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaiStates {
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

enum QueryParam {
    Windows,
    Spaces,
    Displays,
}

impl QueryParam {
    pub fn as_str(&self) -> &'static str {
        match *self {
            QueryParam::Windows => "--windows",
            QueryParam::Spaces => "--spaces",
            QueryParam::Displays => "--displays",
        }
    }
}

fn yabai_query<T>(param: QueryParam) -> Result<T>
where
    T: DeserializeOwned,
{
    let output = Command::new("yabai")
        .arg("-m")
        .arg("query")
        .arg(param.as_str())
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8(output.stderr)?;
        bail!("Failed to execute yabai query: {}", err);
    }

    let raw = String::from_utf8(output.stdout)?;
    let json: T = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to deserialize JSON: {}", raw))?;
    Ok(json)
}

pub fn fetch_from_yabai() -> Result<YabaiStates> {
    let windows: Vec<Window> =
        yabai_query(QueryParam::Windows).context("Failed to query yabai for the window states")?;
    let displays: Vec<Display> = yabai_query(QueryParam::Displays)
        .context("Failed to query yabai for the display states")?;
    let spaces: Vec<Space> =
        yabai_query(QueryParam::Spaces).context("Failed to query yabai for the space states")?;
    let states = YabaiStates {
        windows,
        displays,
        spaces,
    };
    Ok(states)
}

fn load_from_file<T>(filename: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let home = std::env::var("HOME")?;
    let path = &PathBuf::from(format!("{}/.cache/{}", home, filename));
    let output = read_to_string(path)?;
    let json: T = serde_json::from_str(&output)
        .with_context(|| format!("Failed to deserialize JSON: {}", output))?;
    Ok(json)
}

pub fn load_yabaictl() -> Result<YabaictlStates> {
    let states: YabaictlStates = load_from_file("yabaictl")?;
    Ok(states)
}

pub fn load_yabai() -> Result<YabaiStates> {
    let states: YabaiStates = load_from_file("yabai")?;
    Ok(states)
}
