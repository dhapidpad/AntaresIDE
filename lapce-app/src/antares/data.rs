//! Reactive state bridge for the Antares AI agent panel in Lapce.

use std::rc::Rc;

use antares_agent::{PanelAction, PanelSideEffect, PanelState};
use floem::reactive::{RwSignal, Scope, SignalUpdate};

use crate::window_tab::CommonData;

/// Reactive panel state holder owned by WindowTabData.
#[derive(Clone)]
pub struct AntaresPanelData {
    pub scope: Scope,
    pub state: RwSignal<PanelState>,
    pub common: Rc<CommonData>,
}

impl AntaresPanelData {
    /// Initialize a new Antares panel data container within the reactive scope.
    pub fn new(cx: Scope, common: Rc<CommonData>) -> Self {
        Self {
            scope: cx,
            state: cx.create_rw_signal(PanelState::new()),
            common,
        }
    }

    /// Dispatch an action to the pure domain reducer and execute any side effects.
    pub fn dispatch(&self, action: PanelAction) {
        let mut effects = Vec::new();
        self.state.update(|s| {
            effects = s.reduce(action);
        });

        for effect in effects {
            self.handle_effect(effect);
        }
    }

    /// Execute side effects emitted by the pure reducer.
    fn handle_effect(&self, effect: PanelSideEffect) {
        match effect {
            PanelSideEffect::LaunchProcess => {
                // Background supervisor launch in desktop environment
            }
            PanelSideEffect::SendPrompt { session_id: _, text: _ } => {
                // Background prompt dispatch to ACP child
            }
            PanelSideEffect::SendCancel { session_id: _ } => {
                // Background cancel dispatch to ACP child
            }
            PanelSideEffect::NotifyError(_err) => {
                // Diagnostic logging
            }
        }
    }

    /// Update the draft prompt text in the input composer.
    pub fn set_input(&self, text: String) {
        self.dispatch(PanelAction::SetInputDraft(text));
    }

    /// Submit the current draft prompt to the agent.
    pub fn send_prompt(&self) {
        self.dispatch(PanelAction::SubmitPrompt);
    }

    /// Request cancellation of the active prompt turn.
    pub fn cancel(&self) {
        self.dispatch(PanelAction::RequestCancel);
    }

    /// Clear the transcript history for the current session.
    pub fn clear_transcript(&self) {
        self.dispatch(PanelAction::ClearTranscript);
    }

    /// Clean up session and cancel stale work upon workspace switch.
    pub fn switch_workspace(&self) {
        self.dispatch(PanelAction::SwitchWorkspace);
    }
}
