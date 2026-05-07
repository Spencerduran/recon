# Sidebar Card Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a width-responsive card layout to recon's TUI that activates automatically when the terminal is narrower than 100 columns, making it readable as a tmux sidebar pane.

**Architecture:** All changes live in `src/ui.rs`. The `render()` entry point checks `frame.area().width` and dispatches to either the existing `render_table()` (≥ 100 cols) or a new `render_cards()` function (< 100 cols). No changes to session logic, app state, or key handling.

**Tech Stack:** Rust, ratatui (already in use — `Line`, `Span`, `Style`, `Paragraph`, `Block` all already imported)

---

### Task 1: Add `render_cards` function to `ui.rs`

**Files:**
- Modify: `src/ui.rs`

Each session renders as 3 lines + a blank separator. Status colors: Working = cyan, Input = yellow, Idle = yellow (both are `prefix+u` targets), New = dark gray. Input rows and the selected row get a background highlight, matching the table's existing behavior.

- [ ] **Step 1: Add `render_cards` after `render_table` in `src/ui.rs`**

Insert this function after the closing `}` of `render_table` (around line 145):

```rust
fn render_cards(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, session) in app.sessions.iter().enumerate() {
        let (icon, status_label, status_color) = match session.status {
            SessionStatus::New     => ("·", "New",     Color::DarkGray),
            SessionStatus::Working => ("●", "Working", Color::Cyan),
            SessionStatus::Idle    => ("○", "Idle",    Color::Yellow),
            SessionStatus::Input   => ("⧗", "Input",   Color::Yellow),
        };

        let line_style = if session.status == SessionStatus::Input {
            Style::default().bg(Color::Rgb(50, 40, 0))
        } else if i == app.selected {
            Style::default().bg(Color::Rgb(40, 40, 60))
        } else {
            Style::default()
        };

        let activity = session
            .last_activity
            .as_deref()
            .map(format_timestamp)
            .unwrap_or_else(|| "—".to_string());

        let cwd = shorten_home(&session.cwd);
        let model = session.model_display();
        let tokens = session.token_display();

        // Line 1: status icon + session name
        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(
                    session.project_name.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ])
            .style(line_style),
        );

        // Line 2: status label · token usage · age
        lines.push(
            Line::from(vec![
                Span::raw("    "),
                Span::styled(status_label, Style::default().fg(status_color)),
                Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
                Span::raw(tokens),
                Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
                Span::raw(activity),
            ])
            .style(line_style),
        );

        // Line 3: directory · model
        lines.push(
            Line::from(vec![
                Span::raw("    "),
                Span::styled(cwd, Style::default().fg(Color::DarkGray)),
                Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
                Span::styled(model, Style::default().fg(Color::DarkGray)),
            ])
            .style(line_style),
        );

        lines.push(Line::from(""));
    }

    if app.sessions.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  no sessions", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" recon — Claude Code Sessions ");

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
```

- [ ] **Step 2: Build to confirm no compile errors**

```bash
cd ~/repos/recon && cargo build 2>&1
```

Expected: `Finished` with no errors. If you see a lifetime error on `Line` or `Span`, check that all `String` values are moved (not borrowed) into spans — `session.project_name.clone()`, and local vars `tokens`, `activity`, `cwd`, `model` are all owned Strings consumed by `Span::raw`/`Span::styled`.

---

### Task 2: Wire width-responsive dispatch in `render()`

**Files:**
- Modify: `src/ui.rs:12-21`

- [ ] **Step 1: Update `render()` to check terminal width**

Replace the current `render` function (lines 12–21):

```rust
// BEFORE
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_table(frame, app, chunks[0]);
    render_footer(frame, chunks[1]);
}
```

```rust
// AFTER
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    if frame.area().width < 100 {
        render_cards(frame, app, chunks[0]);
    } else {
        render_table(frame, app, chunks[0]);
    }
    render_footer(frame, chunks[1]);
}
```

- [ ] **Step 2: Build and install**

```bash
cd ~/repos/recon && cargo install --path . 2>&1 | tail -5
```

Expected: `Installed package 'recon'` — updates `~/.cargo/bin/recon`.

- [ ] **Step 3: Verify card mode at narrow width**

Kill and retoggle the sidebar (which runs at 80 cols, well under the 100-col threshold):

```bash
fish -c cc-sidebar-toggle  # off
fish -c cc-sidebar-toggle  # on
```

Then capture the pane to confirm card layout:

```bash
tmux list-panes -s -F '#{pane_id} #{pane_title}' | grep cc-sidebar
# note the pane ID, e.g. %44

tmux capture-pane -t %44 -p | head -15
```

Expected output shape:
```
┌ recon — Claude Code Sessions ────────────────────┐
│                                                  │
│  ● session-name                                  │
│    Working  ·  68k / 200k  ·  < 1m              │
│    ~/repos/recon  ·  Sonnet 4.6                  │
│                                                  │
│  ○ other-session                                 │
│    Idle  ·  27k / 200k  ·  5m ago               │
│    ~/vaults/mind_forge  ·  Sonnet 4.6            │
│                                                  │
```

- [ ] **Step 4: Verify table mode still works at wide width**

Run recon in a full-width terminal (or resize the sidebar to ≥ 100 cols):

```bash
~/.cargo/bin/recon
```

Expected: the existing 8-column table renders unchanged.

- [ ] **Step 5: Commit**

```bash
cd ~/repos/recon
git add src/ui.rs
git commit -m "feat: width-responsive card layout for sidebar mode"
```
