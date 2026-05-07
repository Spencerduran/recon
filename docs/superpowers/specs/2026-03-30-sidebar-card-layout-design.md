# Recon Sidebar Card Layout

## Goal

Recon currently renders a wide table (~118 col minimum) that squishes unreadably in a tmux sidebar pane. This spec adds a width-responsive card layout that activates automatically when the terminal is narrow.

## Behavior

When `frame.size().width < 100`, render sessions as stacked 3-line cards instead of the table. Above 100 cols, the existing table renders unchanged.

No flags, no new subcommands. Resize the pane and the layout adapts.

## Card Layout (per session)

```
  ● session-name
    Status  ·  68k / 200k  ·  < 1m
    ~/path/to/dir  ·  Sonnet 4.6
```

Three lines plus a blank separator between sessions. The selected session highlights all three lines.

## Status Colors

| Status  | Icon | Color  | Meaning                          |
|---------|------|--------|----------------------------------|
| Working | ●    | cyan   | Claude is running                |
| Input   | ⧗    | yellow | Waiting — permission prompt      |
| Idle    | ○    | yellow | Waiting — done, ready for input  |
| New     | ·    | dim    | No tokens yet                    |

Input and Idle share yellow because both are `prefix+u` / `recon done` targets. The icon distinguishes why Claude stopped.

## Implementation

Changes are isolated to `ui.rs`:

1. Add `render_cards(frame, app, area)` function — builds a ratatui `List` where each `ListItem` contains a 3-line `Text` block using `Span`s for color.
2. In the existing render entry point, check `area.width`. Call `render_cards` or the existing table renderer accordingly.
3. Selection highlight spans all 3 lines of the focused card.

No changes to `session.rs`, `app.rs`, key handling, or any other module. The `SessionStatus` enum values (`Working`, `Input`, `Idle`, `New`) map directly to the card colors.

## Width Threshold

`< 100` cols triggers card mode. This matches the point where the table starts visibly squishing (the `Constraint::Min(20)` Project column collapses below readable width around 90-95 cols).
