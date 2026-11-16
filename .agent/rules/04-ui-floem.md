# Floem UI rules

- Read current panel registration and view patterns in Lapce before adding the Antares panel.
- Keep panel domain state/reducer independent from Floem. Only view/adapter modules should use Floem types.
- Use existing Lapce theme/config keys. New theme keys use an `antares-` namespace only after confirming the current theme architecture.
- Stream text incrementally into the active assistant message; do not rebuild the whole conversation list per token.
- Every async operation exposes idle/running/cancelling/succeeded/error state and a working cancel path.
- Keep editor focus and keyboard navigation predictable. Do not steal focus on every streamed event.
- No inert buttons, simulated API results, or hidden actions. Disabled controls need a reason or tooltip.
