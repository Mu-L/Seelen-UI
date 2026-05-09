use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::state::application::FULL_STATE;
use crate::trace_lock;
use crate::virtual_desktops::SluWorkspacesManager2;
use crate::widgets::window_manager::state_v2::{
    set_rect_to_float_initial_size, TwmState, TwmStateEvent, WM_STATE,
};
use crate::windows_api::window::Window;

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum AllowedReservations {
    Left,
    Right,
    Top,
    Bottom,
    Stack,
    Float,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum NodeSiblingSide {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Sizing {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum StepWay {
    Next,
    Prev,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Axis {
    Horizontal,
    Vertical,
    Top,
    Bottom,
    Left,
    Right,
}

/// Manage the Seelen Window Manager.
#[derive(Debug, Serialize, Deserialize, clap::Args)]
#[command(alias = "wm")]
pub struct WindowManagerCli {
    #[command(subcommand)]
    pub subcommand: WmCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum WmCommand {
    /// Open Dev Tools (only works if the app is running in dev mode)
    Debug,
    /// Toggles the Seelen Window Manager.
    Toggle,
    /// Reserve space for a incoming window.
    Reserve {
        /// The position of the new window.
        side: AllowedReservations,
    },
    /// Cancels the current reservation
    CancelReservation,
    /// Increases or decreases the size of the window
    Width {
        /// What to do with the width.
        action: Sizing,
    },
    /// Increases or decreases the size of the window
    Height {
        /// What to do with the height.
        action: Sizing,
    },
    /// Resets the size of the containers in current workspace to the default size.
    ResetWorkspaceSize,
    /// Toggles the floating state of the window
    ToggleFloat,
    /// Toggles workspace layout mode to monocle (single stack)
    ToggleMonocle,
    /// Moves the window to the specified position
    Move { side: NodeSiblingSide },
    /// Cycles the foregrounf node if it is a stack
    CycleStack { way: StepWay },
    /// Focuses the window in the specified position.
    Focus {
        /// The position of the window to focus.
        side: NodeSiblingSide,
    },
}

impl WindowManagerCli {
    pub fn process(self) -> Result<()> {
        self.subcommand.process()
    }
}

impl WmCommand {
    pub fn process(self) -> Result<()> {
        let foreground = Window::get_foregrounded();

        match self {
            WmCommand::Toggle => {
                FULL_STATE.rcu(move |state| {
                    let mut state = state.cloned();
                    state.settings.by_widget.wm.enabled = !state.settings.by_widget.wm.enabled;
                    state
                });
                FULL_STATE.load().write_settings()?;
            }
            WmCommand::Debug => {
                #[cfg(debug_assertions)]
                {
                    let guard = trace_lock!(crate::app::SEELEN);
                    for instance in &guard.widgets_per_display {
                        if let Some(wm) = &instance.wm {
                            wm.window.open_devtools();
                        }
                    }
                }
            }
            WmCommand::Width { action } => {
                let percentage = match action {
                    Sizing::Increase => FULL_STATE.load().settings.by_widget.wm.resize_delta,
                    Sizing::Decrease => -FULL_STATE.load().settings.by_widget.wm.resize_delta,
                };
                WM_STATE
                    .lock()
                    .update_size(&foreground, Axis::Horizontal, percentage, false)?;
            }
            WmCommand::Height { action } => {
                let percentage = match action {
                    Sizing::Increase => FULL_STATE.load().settings.by_widget.wm.resize_delta,
                    Sizing::Decrease => -FULL_STATE.load().settings.by_widget.wm.resize_delta,
                };
                WM_STATE
                    .lock()
                    .update_size(&foreground, Axis::Vertical, percentage, false)?;
            }
            WmCommand::Reserve { .. } => {
                // self.reserve(side)?;
            }
            WmCommand::CancelReservation => {
                // self.discard_reservation()?;
            }
            WmCommand::ResetWorkspaceSize => {
                let window_id = foreground.address();
                let mut guard = WM_STATE.lock();
                if guard.state.contains(&window_id) {
                    if guard.state.is_tiled(&window_id) {
                        if let Some((_, tree)) = guard.get_tree_for_window_mut(&foreground) {
                            tree.reset_sizes();
                            TwmState::send(TwmStateEvent::Changed);
                        }
                    } else {
                        set_rect_to_float_initial_size(&foreground)?;
                    }
                }
            }
            WmCommand::ToggleFloat => {
                let mut state = WM_STATE.lock();
                let window_id = foreground.address();
                if let Some((_ws_id, tree)) = state.get_tree_for_window_mut(&foreground) {
                    if tree.is_floating(&window_id) {
                        let residual = tree.add_to_tiled(window_id);
                        for w in residual {
                            tree.add_to_floating(w);
                        }
                    } else if tree.is_tiled(&window_id) {
                        let residual = tree.remove_window(&window_id);
                        for w in residual {
                            tree.add_to_floating(w);
                        }
                        tree.add_to_floating(window_id);
                        set_rect_to_float_initial_size(&foreground)?;
                    }

                    TwmState::send(TwmStateEvent::Changed);
                }
            }
            WmCommand::ToggleMonocle => {
                let monitor_id = foreground.monitor_id();
                let workspace_id = SluWorkspacesManager2::instance()
                    .monitors
                    .get(&monitor_id, |m| m.active_workspace_id().clone())
                    .ok_or("Monitor not found")?;
                WM_STATE.lock().toggle_monocle(&workspace_id);
            }
            WmCommand::Focus { side } => {
                let mut state = WM_STATE.lock();
                let window_id = foreground.address();
                if let Some((_ws_id, tree)) = state.get_tree_for_window_mut(&foreground) {
                    let (match_h, want_before) = side_to_flags(&side);
                    let siblings = tree.siblings_at_side(&window_id, match_h, want_before);
                    match siblings.first().and_then(|&nid| tree.face_of_node(nid)) {
                        Some(target_id) => Window::from(target_id).focus()?,
                        None => log::warn!("There is no node at {side:?} to be focused"),
                    }
                }
            }
            WmCommand::Move { side } => {
                let mut state = WM_STATE.lock();
                let window_id = foreground.address();
                if let Some((_ws_id, tree)) = state.get_tree_for_window_mut(&foreground) {
                    let (match_h, want_before) = side_to_flags(&side);
                    let siblings = tree.siblings_at_side(&window_id, match_h, want_before);
                    match siblings.first().and_then(|&nid| tree.face_of_node(nid)) {
                        Some(target_id) => {
                            tree.swap_windows(window_id, target_id);
                            TwmState::send(TwmStateEvent::Changed);
                        }
                        None => log::warn!("There is no node at {side:?} to be swapped"),
                    }
                }
            }
            WmCommand::CycleStack { way } => {
                WM_STATE.lock().cycle_stack(&foreground, way)?;
            }
        };

        Ok(())
    }
}

fn side_to_flags(side: &NodeSiblingSide) -> (bool, bool) {
    match side {
        NodeSiblingSide::Left => (true, true),
        NodeSiblingSide::Right => (true, false),
        NodeSiblingSide::Up => (false, true),
        NodeSiblingSide::Down => (false, false),
    }
}
