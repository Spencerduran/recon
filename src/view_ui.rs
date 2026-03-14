use std::collections::BTreeMap;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::app::App;
use crate::session::{Session, SessionStatus};

// Layout constants
const MIN_ROOM_WIDTH: u16 = 28;
const CHAR_WIDTH: u16 = 12;
const CHAR_ART_HEIGHT: u16 = 5;
const CHAR_LABEL_LINES: u16 = 2; // session name + branch
const CHAR_HEIGHT: u16 = CHAR_ART_HEIGHT + CHAR_LABEL_LINES;

// ── Character art (5 lines each, ≤10 chars wide, 2 variants per status) ──

const CHAR_NEW: [[&str; 5]; 2] = [
    ["  .---.  ", " / . . \\ ", "|   .   |", " \\ ___ / ", "  '---'  "],
    ["  ,---.  ", " / o o \\ ", "|   ~   |", " \\ === / ", "  '---'  "],
];

const CHAR_WORKING: [[&str; 5]; 2] = [
    ["  .---.  ", " / ^.^ \\ ", "|  ===  |", " \\_____/ ", "  d   b  "],
    ["  .---.  ", " / >.< \\ ", "|  ~~~  |", " \\_____/ ", "  d   b  "],
];

const CHAR_IDLE: [[&str; 5]; 2] = [
    [" .---. Zz", " / -.- \\  ", "|  ~~~  | ", " \\_____/  ", "          "],
    [" .---. Zz", " / =.= \\  ", "|  ~~~  | ", " \\_____/  ", "          "],
];

const CHAR_INPUT: [[&str; 5]; 2] = [
    [" .---.  !", " / T.T \\  ", "|  ___  | ", " \\_____/  ", "  /   \\   "],
    [" .---.  !", " / ;.; \\  ", "|  ___  | ", " \\_____/  ", "  /   \\   "],
];

// ── Room grouping ────────────────────────────────────────────────────

struct Room {
    name: String,
    session_indices: Vec<usize>,
    has_input: bool,
}

fn group_into_rooms(sessions: &[Session]) -> Vec<Room> {
    let mut map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (i, s) in sessions.iter().enumerate() {
        let basename = if s.cwd.is_empty() {
            "unknown".to_string()
        } else {
            std::path::Path::new(&s.cwd)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| s.cwd.clone())
        };
        map.entry(basename).or_default().push(i);
    }

    let mut rooms: Vec<Room> = map
        .into_iter()
        .map(|(name, indices)| {
            let has_input = indices
                .iter()
                .any(|&i| sessions[i].status == SessionStatus::Input);
            Room {
                name,
                session_indices: indices,
                has_input,
            }
        })
        .collect();

    // Rooms with Input agents sort first, then alphabetical
    rooms.sort_by(|a, b| {
        b.has_input
            .cmp(&a.has_input)
            .then_with(|| a.name.cmp(&b.name))
    });

    rooms
}

// ── Character art selection ──────────────────────────────────────────

fn character_variant(session_id: &str) -> usize {
    (session_id.bytes().fold(0u8, |a, b| a.wrapping_add(b)) % 2) as usize
}

fn character_art(status: &SessionStatus, variant: usize) -> &'static [&'static str; 5] {
    let v = variant % 2;
    match status {
        SessionStatus::New => &CHAR_NEW[v],
        SessionStatus::Working => &CHAR_WORKING[v],
        SessionStatus::Idle => &CHAR_IDLE[v],
        SessionStatus::Input => &CHAR_INPUT[v],
    }
}

fn status_color(status: &SessionStatus) -> Color {
    match status {
        SessionStatus::New => Color::Blue,
        SessionStatus::Working => Color::Green,
        SessionStatus::Idle => Color::DarkGray,
        SessionStatus::Input => Color::Yellow,
    }
}

// ── Public render entry point ────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    render_rooms(frame, app, chunks[0]);
    render_footer(frame, chunks[1]);
}

fn render_rooms(frame: &mut Frame, app: &App, area: Rect) {
    let rooms = group_into_rooms(&app.sessions);

    if rooms.is_empty() {
        render_empty(frame, area);
        return;
    }

    let cols = (area.width / MIN_ROOM_WIDTH).max(1) as usize;

    // Chunk rooms into grid rows
    let grid_rows: Vec<&[Room]> = rooms.chunks(cols).collect();

    // Compute height for each grid row
    let row_heights: Vec<u16> = grid_rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|room| room_height(room, area.width / cols as u16))
                .max()
                .unwrap_or(CHAR_HEIGHT + 2)
        })
        .collect();

    let total_height: u16 = row_heights.iter().sum();
    let top_pad = area.height.saturating_sub(total_height) / 2;

    // Offset area downward for vertical centering
    let centered_area = Rect {
        x: area.x,
        y: area.y + top_pad,
        width: area.width,
        height: area.height.saturating_sub(top_pad),
    };

    // Build vertical constraints: grid rows + remainder
    let mut constraints: Vec<Constraint> = row_heights
        .iter()
        .map(|&h| Constraint::Length(h))
        .collect();
    constraints.push(Constraint::Min(0)); // absorb extra space

    let v_chunks = Layout::vertical(constraints).split(centered_area);
    let row_offset = 0;

    for (row_idx, room_row) in grid_rows.iter().enumerate() {
        let h_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        let h_chunks = Layout::horizontal(h_constraints).split(v_chunks[row_offset + row_idx]);

        for (col_idx, room) in room_row.iter().enumerate() {
            render_room(frame, app, room, h_chunks[col_idx]);
        }
    }
}

fn room_height(room: &Room, room_width: u16) -> u16 {
    let inner_width = room_width.saturating_sub(2); // borders
    let chars_per_row = (inner_width / CHAR_WIDTH).max(1) as usize;
    let char_rows = (room.session_indices.len() + chars_per_row - 1) / chars_per_row;
    2 + (char_rows as u16) * CHAR_HEIGHT // 2 for top/bottom border
}

fn render_room(frame: &mut Frame, app: &App, room: &Room, area: Rect) {
    let border_color = if room.has_input {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let title = format!(" {} ({}) ", room.name, room.session_indices.len());
    let title_style = if room.has_input {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, title_style))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Lay out characters inside the room
    let chars_per_row = (inner.width / CHAR_WIDTH).max(1) as usize;
    let char_rows: Vec<&[usize]> = room.session_indices.chunks(chars_per_row).collect();

    let row_constraints: Vec<Constraint> = char_rows
        .iter()
        .map(|_| Constraint::Length(CHAR_HEIGHT))
        .collect();
    let v_chunks = Layout::vertical(row_constraints).split(inner);

    for (row_idx, indices) in char_rows.iter().enumerate() {
        if row_idx >= v_chunks.len() {
            break;
        }
        let col_constraints: Vec<Constraint> = indices
            .iter()
            .map(|_| Constraint::Length(CHAR_WIDTH))
            .collect();
        let h_chunks = Layout::horizontal(col_constraints).split(v_chunks[row_idx]);

        for (col_idx, &session_idx) in indices.iter().enumerate() {
            if col_idx >= h_chunks.len() {
                break;
            }
            render_character(frame, &app.sessions[session_idx], h_chunks[col_idx]);
        }
    }
}

fn render_character(frame: &mut Frame, session: &Session, area: Rect) {
    if area.height < 3 || area.width < 4 {
        return;
    }

    let variant = character_variant(&session.session_id);
    let art = character_art(&session.status, variant);
    let color = status_color(&session.status);

    let mut lines: Vec<Line> = Vec::new();

    // Character art (5 lines)
    for &line in art {
        let truncated = truncate_str(line, area.width as usize);
        lines.push(Line::from(Span::styled(
            truncated,
            Style::default().fg(color),
        )));
    }

    // Session name label
    let name = session
        .tmux_session
        .as_deref()
        .unwrap_or("???");
    let name_truncated = truncate_str(name, area.width as usize);
    lines.push(Line::from(Span::styled(
        name_truncated,
        Style::default().fg(Color::White),
    )));

    // Git branch label
    let branch = session.branch.as_deref().unwrap_or("");
    let branch_truncated = truncate_str(branch, area.width as usize);
    lines.push(Line::from(Span::styled(
        branch_truncated,
        Style::default().fg(Color::Green),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_empty(frame: &mut Frame, area: Rect) {
    let art = CHAR_IDLE[0];
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for &line in &art {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "No active sessions",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("v", Style::default().fg(Color::Cyan)),
        Span::raw(" table  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(" refresh  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
    ]));
    frame.render_widget(footer, area);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width > 1 {
        format!("{}\u{2026}", &s[..max_width - 1])
    } else {
        String::new()
    }
}
