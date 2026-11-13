use std::path::PathBuf;

use super::store::{load_from, save_to};
use super::{SavedPane, SavedState, SavedTab, SavedWorkspace, VERSION};
use crate::shared::pane::{PaneKind, PaneTree, SplitAxis};

fn sample() -> SavedState {
    let agent = SavedPane {
        kind: PaneKind::Agent,
        cwd: PathBuf::from("/tmp/project"),
        path: None,
        session: Some(PathBuf::from("/tmp/sessions/a.jsonl")),
    };
    let terminal = SavedPane {
        kind: PaneKind::Terminal,
        cwd: PathBuf::from("/tmp/project"),
        path: None,
        session: None,
    };
    let file = SavedPane {
        kind: PaneKind::File,
        cwd: PathBuf::from("/tmp/project"),
        path: Some(PathBuf::from("/tmp/project/README.md")),
        session: None,
    };
    SavedState {
        version: VERSION,
        current: 1,
        workspaces: vec![
            SavedWorkspace {
                name: "Default".into(),
                root: PathBuf::from("/tmp"),
                tabs: vec![SavedTab {
                    layout: PaneTree::Leaf(agent.clone()),
                    focused: 0,
                }],
                active_tab: 0,
                last_active: 10,
            },
            SavedWorkspace {
                name: "project".into(),
                root: PathBuf::from("/tmp/project"),
                tabs: vec![SavedTab {
                    layout: PaneTree::Split {
                        axis: SplitAxis::Horizontal,
                        ratio: 0.4,
                        first: Box::new(PaneTree::Leaf(agent)),
                        second: Box::new(PaneTree::Split {
                            axis: SplitAxis::Vertical,
                            ratio: 0.5,
                            first: Box::new(PaneTree::Leaf(terminal)),
                            second: Box::new(PaneTree::Leaf(file)),
                        }),
                    },
                    focused: 2,
                }],
                active_tab: 0,
                last_active: 20,
            },
        ],
        sidebar: false,
    }
}

#[test]
fn json_round_trip() {
    let state = sample();
    let json = serde_json::to_string(&state).unwrap();
    let back: SavedState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, state);
    // Optional fields stay out of the file and default back in.
    assert!(!json.contains("\"path\":null"));
    let minimal: SavedState =
        serde_json::from_str(r#"{"workspaces":[{"name":"x","root":"/x"}]}"#).unwrap();
    assert_eq!(minimal.workspaces[0].tabs.len(), 0);
    assert_eq!(minimal.current, 0);
    // A file from before the sidebar was saved keeps it showing.
    assert!(minimal.sidebar);
    assert_eq!(SavedState::default().sidebar, minimal.sidebar);
}

#[test]
fn file_round_trip() {
    let dir = std::env::temp_dir().join(format!("gentle-ws-test-{}", std::process::id()));
    let path = dir.join("nested").join("workspaces.json");
    let state = sample();
    save_to(&path, &state).unwrap();
    assert_eq!(load_from(&path), Some(state));
    std::fs::write(&path, "not json").unwrap();
    assert_eq!(load_from(&path), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// A newer build saved a pane kind this one doesn't know (as the History
/// pane was for older builds): the file still loads with every workspace,
/// and only that pane goes.
#[test]
fn unknown_pane_kinds_drop_only_those_panes() {
    let dir = std::env::temp_dir().join(format!("gentle-ws-unknown-{}", std::process::id()));
    let path = dir.join("workspaces.json");
    std::fs::create_dir_all(&dir).unwrap();
    let mut json = serde_json::to_value(sample()).unwrap();
    // Workspace 1: the file pane (second of the inner split) becomes unknown.
    json["workspaces"][1]["tabs"][0]["layout"]["split"]["second"]["split"]["second"]["leaf"]["kind"] =
        "from_a_newer_build".into();
    // Workspace 0: its only tab is unknown and goes; a second tab stays.
    json["workspaces"][0]["tabs"][0]["layout"]["leaf"]["kind"] = "from_a_newer_build".into();
    let kept = serde_json::to_value(&sample().workspaces[0].tabs[0]).unwrap();
    json["workspaces"][0]["tabs"]
        .as_array_mut()
        .unwrap()
        .push(kept);
    json["workspaces"][0]["active_tab"] = 1.into();
    std::fs::write(&path, json.to_string()).unwrap();

    let state = load_from(&path).expect("loads despite the unknown panes");
    assert_eq!(state.workspaces.len(), 2);
    assert_eq!(state.workspaces[0].tabs.len(), 1);
    assert_eq!(state.workspaces[0].active_tab, 0);
    let tab = &state.workspaces[1].tabs[0];
    let kinds: Vec<PaneKind> = tab.layout.leaves().iter().map(|p| p.kind).collect();
    assert_eq!(kinds, [PaneKind::Agent, PaneKind::Terminal]);
    // The focused pane was the dropped one: focus falls back to the first.
    assert_eq!(tab.focused, 0);
    assert!(!dir.join("workspaces.json.unreadable").exists());
    let _ = std::fs::remove_dir_all(dir);
}

/// A file that doesn't read at all is kept aside before the next save
/// replaces it.
#[test]
fn an_unreadable_file_is_copied_aside() {
    let dir = std::env::temp_dir().join(format!("gentle-ws-unreadable-{}", std::process::id()));
    let path = dir.join("workspaces.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    assert_eq!(load_from(&path), None);
    assert_eq!(
        std::fs::read_to_string(dir.join("workspaces.json.unreadable")).unwrap(),
        "{ not json"
    );
    let _ = std::fs::remove_dir_all(dir);
}
