//! The "Calm" preset: hide Windows UI clutter through per-user settings
//! that the Settings app itself writes. See docs/research/03 and 04.
//!
//! NOTE: value names verified against Windows 11 behaviour as commonly
//! documented; re-check each on 26H2 before release. Some (Widgets) can be
//! write-protected by Windows; Cove then offers the Settings page instead.

use super::RegTweak;

const ADV: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";
const CDM: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager";

macro_rules! t {
    ($base:expr, $name:literal) => {
        concat!($base, "\\", $name)
    };
}

// concat! needs literals, so the bases are repeated as literals here.
macro_rules! adv {
    ($name:literal) => {
        t!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced", $name)
    };
}
macro_rules! cdm {
    ($name:literal) => {
        t!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager", $name)
    };
}

pub static TWEAKS: &[RegTweak] = &[
    RegTweak {
        id: "calm.taskbar-taskview",
        title: "Hide the Task View button",
        description: "Cove's overview replaces it.",
        settings_uri: "ms-settings:taskbar",
        writes: &[(adv!("ShowTaskViewButton"), 0)],
    },
    RegTweak {
        id: "calm.taskbar-search",
        title: "Hide the taskbar search box",
        description: "Cove Search takes over (Alt+Space).",
        settings_uri: "ms-settings:taskbar",
        writes: &[(
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Search\\SearchboxTaskbarMode",
            0,
        )],
    },
    RegTweak {
        id: "calm.taskbar-widgets",
        title: "Hide the Widgets button",
        description: "Removes the weather and news button.",
        settings_uri: "ms-settings:taskbar",
        writes: &[(adv!("TaskbarDa"), 0)],
    },
    RegTweak {
        id: "calm.taskbar-copilot",
        title: "Hide the Copilot button",
        description: "Copilot stays installed; only the button goes.",
        settings_uri: "ms-settings:taskbar",
        writes: &[(adv!("ShowCopilotButton"), 0)],
    },
    RegTweak {
        id: "calm.start-recommended",
        title: "Quiet Start's Recommended section",
        description: "No recent files or app promotions in Start.",
        settings_uri: "ms-settings:personalization-start",
        writes: &[(adv!("Start_TrackDocs"), 0), (adv!("Start_IrisRecommendations"), 0)],
    },
    RegTweak {
        id: "calm.lockscreen-tips",
        title: "Turn off lock-screen tips",
        description: "No fun facts or tips on the lock screen.",
        settings_uri: "ms-settings:lockscreen",
        writes: &[
            (cdm!("RotatingLockScreenOverlayEnabled"), 0),
            (cdm!("SubscribedContent-338387Enabled"), 0),
        ],
    },
    RegTweak {
        id: "calm.suggestions",
        title: "Turn off tips and suggestions",
        description: "Fewer nags in Start, Settings and notifications.",
        settings_uri: "ms-settings:privacy",
        writes: &[
            (cdm!("SystemPaneSuggestionsEnabled"), 0),
            (cdm!("SubscribedContent-338388Enabled"), 0),
            (cdm!("SubscribedContent-338389Enabled"), 0),
            (cdm!("SubscribedContent-310093Enabled"), 0),
        ],
    },
];

pub fn find(id: &str) -> Option<&'static RegTweak> {
    TWEAKS.iter().find(|t| t.id == id)
}

#[allow(dead_code)]
const _BASES: (&str, &str) = (ADV, CDM);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_targets_are_hkcu() {
        let mut ids: Vec<_> = TWEAKS.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), TWEAKS.len());
        for t in TWEAKS {
            for (target, _) in t.writes {
                assert!(target.starts_with("HKCU\\"), "{target}");
            }
        }
    }
}
