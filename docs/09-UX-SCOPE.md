# Antares IDE UX scope

## Shell layout

Keep the native Lapce workspace structure. Add an Antares AI panel as a dockable panel; do not redesign the whole editor before its agent flow is stable.

### Core zones

- **Activity / navigation:** existing explorer, search, source control, plugins and debug/navigation surfaces.
- **Editor canvas:** existing Lapce editor, tabs, splits, diagnostics and inline actions.
- **Right-side Antares panel:** conversation, session selector, model/role selector, context summary, activity stream and composer.
- **Bottom panel:** existing terminal/problems/output; later add Antares background task view where Lapce patterns support it.
- **Status:** agent connection, active session, model/role, usage/cost when provided, and policy mode.

## AI panel states

- Empty: explain how to start a session and attach active context.
- Starting: show sidecar/protocol startup state.
- Ready: composer enabled, current session and context visible.
- Streaming: append partial assistant output and update activity without rebuilding transcript.
- Waiting for user: display pending patch/command proposal with clear actions.
- Cancelling: visibly show cancellation progress.
- Authentication required: show provider-specific setup guidance without exposing secrets.
- Disconnected/error: show process/protocol error and a safe reconnect/resume action.

## First release features

1. Start/new session, resume session and cancel turn.
2. Stream assistant text and tool activity.
3. Ask about active file or selection.
4. Attach a small set of selected workspace files.
5. Show context summary and what was excluded/redacted.
6. Receive and preview patch proposals.
7. Accept/reject proposals by hunk with undo support.
8. Request a terminal command; run only after user action.
9. Show policy denials, provider state and usage when supported by Octomind.

## Visual quality principles

- Use Lapce theme system, typography and spacing tokens; no arbitrary hard-coded colors.
- Match existing editor density. Avoid oversize chat bubbles that steal code area.
- Keep hierarchy clear: user request, assistant response, tool activity, pending proposal and policy status are visually distinct.
- Use concise labels, informative empty states and visible focus/keyboard states.
- Avoid heavy animations. Prefer brief transitions that do not block text rendering or input.
- Ensure panel layout works with narrow windows, high-DPI scaling and accessibility settings.
