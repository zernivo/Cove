//! The "Calm" preset: hide Windows UI clutter through documented settings.
//! Implementations come next; this list is the agreed scope
//! (see docs/research/03 and 04 for the clutter tables).

use super::TweakInfo;

pub fn planned() -> Vec<TweakInfo> {
    vec![
        TweakInfo {
            id: "calm.taskbar-widgets",
            title: "Hide the Widgets button",
            description: "Removes the weather/news button from the taskbar.",
            setting: "Settings > Personalization > Taskbar > Widgets",
        },
        TweakInfo {
            id: "calm.taskbar-taskview",
            title: "Hide the Task View button",
            description: "Cove's overview replaces it (Ctrl+Up).",
            setting: "Settings > Personalization > Taskbar > Task view",
        },
        TweakInfo {
            id: "calm.taskbar-search",
            title: "Hide the taskbar search box",
            description: "Cove Search (Alt+Space) takes over.",
            setting: "Settings > Personalization > Taskbar > Search",
        },
        TweakInfo {
            id: "calm.start-recommended",
            title: "Quiet Start's Recommended section",
            description: "No recent files, tips or app promotions in Start.",
            setting: "Settings > Personalization > Start",
        },
        TweakInfo {
            id: "calm.lockscreen-tips",
            title: "Turn off lock-screen tips",
            description: "No 'fun facts' or tips on the lock screen.",
            setting: "Settings > Personalization > Lock screen",
        },
        TweakInfo {
            id: "calm.suggestions",
            title: "Turn off suggestions and offers",
            description: "Fewer nags in Settings, notifications and setup.",
            setting: "Settings > Privacy & security > General; System > Notifications",
        },
        TweakInfo {
            id: "calm.taskbar-autohide",
            title: "Auto-hide the taskbar while the Cove Dock runs",
            description: "Put back automatically when Cove quits.",
            setting: "Settings > Personalization > Taskbar > Taskbar behaviors",
        },
    ]
}
