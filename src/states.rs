use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

static YABAICTL_STATE: &str = "yabaictl";
static YABAI_STATE: &str = "yabai";

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaictlStates {
    pub recent: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YabaiStates {
    pub spaces: Vec<Space>,
    pub displays: Vec<Display>,
    pub windows: Vec<Window>,
}

impl YabaiStates {
    pub fn num_spaces(&self) -> u32 {
        return self.spaces.len().try_into().unwrap();
    }

    pub fn num_displays(&self) -> u32 {
        return self.displays.len().try_into().unwrap();
    }

    pub fn focused_space(&self) -> Option<&Space> {
        self.spaces.iter().find(|space| space.has_focus)
    }

    pub fn find_space_by_label(&self, label: &str) -> Option<&Space> {
        self.spaces.iter().find(|&space| space.label == label)
    }

    pub fn find_unlabeled_space(&self) -> Option<&Space> {
        self.spaces
            .iter()
            // An app going fullscreen gets its own space. MacOS is... weird.
            .find(|&space| space.label == "" && space.is_native_fullscreen)
    }

    pub fn find_space_by_label_index(&self, label_index: u32) -> Option<&Space> {
        let label = format!("s{}", label_index);
        self.spaces.iter().find(|&space| space.label == label)
    }

    pub fn find_window_id_in_space(&self, space_label: &str, window_id: &u32) -> Option<&u32> {
        let space = self.find_space_by_label(space_label);
        match space {
            None => return None,
            Some(space) => return space.find_window_id(window_id),
        };
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Space {
    id: u32,
    uuid: String,
    pub index: u32,
    pub label: String,
    r#type: String,
    display: u32,
    pub windows: Vec<u32>,
    #[serde(rename = "first-window")]
    pub first_window: u32,
    #[serde(rename = "last-window")]
    pub last_window: u32,
    #[serde(rename = "has-focus")]
    pub has_focus: bool,
    #[serde(rename = "is-visible")]
    pub is_visible: bool,
    #[serde(rename = "is-native-fullscreen")]
    is_native_fullscreen: bool,
}

impl Space {
    pub fn find_window_id(&self, window_id: &u32) -> Option<&u32> {
        self.windows.iter().find(|&id| id == window_id)
    }

    pub fn label_index(&self) -> Option<u32> {
        if !self.label.starts_with("s") {
            return None;
        }
        let index = u32::from_str_radix(&self.label[1..], 10);
        match index {
            Ok(index) => Some(index),
            Err(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Display {
    id: u32,
    uuid: String,
    index: u32,
    frame: Frame,
    spaces: Vec<u32>,
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
    role: String,
    subrole: String,
    display: u32,
    space: u32,
    level: u32,
    opacity: f32,
    #[serde(rename = "split-type")]
    split_type: String,
    #[serde(rename = "stack-index")]
    stack_index: u32,

    #[serde(rename = "can-move")]
    can_move: bool,
    #[serde(rename = "can-resize")]
    can_resize: bool,
    #[serde(rename = "has-focus")]
    has_focus: bool,
    #[serde(rename = "has-shadow")]
    has_shadow: bool,
    #[serde(rename = "has-border")]
    has_border: bool,
    #[serde(rename = "has-parent-zoom")]
    has_parent_zoom: bool,
    #[serde(rename = "has-fullscreen-zoom")]
    has_fullscreen_zoom: bool,
    #[serde(rename = "is-native-fullscreen")]
    is_native_fullscreen: bool,
    #[serde(rename = "is-visible")]
    is_visible: bool,
    #[serde(rename = "is-minimized")]
    is_minimized: bool,
    #[serde(rename = "is-hidden")]
    is_hidden: bool,
    #[serde(rename = "is-floating")]
    is_floating: bool,
    #[serde(rename = "is-sticky")]
    is_sticky: bool,
    #[serde(rename = "is-topmost")]
    is_topmost: bool,
    #[serde(rename = "is-grabbed")]
    is_grabbed: bool,
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

pub fn save_yabaictl(states: &YabaictlStates) -> Result<()> {
    save(states, YABAICTL_STATE)?;
    Ok(())
}
