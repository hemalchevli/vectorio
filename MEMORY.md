# Session Memory

## Project Decisions Log
| Date | Decision | Rationale | Alternatives Rejected & Why |
|------|----------|-----------|-----------------------------|
| 2026-11-17 | Workspace initialization | Project setup for Windows port analysis | N/A |
| 2026-11-17 | Option 1: Native Window Frame for Windows | Uses native Windows decorations/titlebar, adapts support_dir to `%APPDATA%`, removes 78px macOS traffic-light padding conditionally. Minimal churn and zero new dependencies. | Option 2 (custom frameless controls) rejected due to UI complexity/window snapping issues; Option 3 (muda HWND menu) rejected due to Win32 menubar styling contrast with egui. |

