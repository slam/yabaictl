use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

static YABAICTL_STATE: &str = "yabaictl";
static YABAI_STATE: &str = "yabai";

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaictlStates {
    recent: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaiStates {
    pub spaces: Vec<Space>,
    pub displays: Vec<Display>,
    pub windows: Vec<Window>,
}

impl YabaiStates {
    pub fn num_spaces(&self) -> usize {
        return self.spaces.len();
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Space {
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
pub struct Display {
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
pub struct Window {
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

fn save<T>(states: &T, filename: &str) -> Result<()>
where
    T: Serialize,
{
    let file = File::create(get_full_path(filename)?)?;
    let result = serde_json::to_writer(file, states)?;
    Ok(result)
}

fn load<T>(filename: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let output = fs::read_to_string(get_full_path(filename)?)?;
    let json: T = serde_json::from_str(&output)
        .with_context(|| format!("Failed to deserialize JSON: {}", output))?;
    Ok(json)
}

fn get_full_path(filename: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME")?;
    let path = PathBuf::from(format!("{}/.cache/{}", home, filename));
    Ok(path)
}

pub fn load_yabaictl() -> Result<YabaictlStates> {
    let states: YabaictlStates = load(YABAICTL_STATE)?;
    Ok(states)
}

pub fn load_yabai() -> Result<YabaiStates> {
    let states: YabaiStates = load(YABAI_STATE)?;
    Ok(states)
}

pub fn save_yabai(states: &YabaiStates) -> Result<()> {
    save(states, YABAI_STATE)?;
    Ok(())
}
