use std::fs;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoSource {
    TodoWrite,
    TaskCreate,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
    pub source: TodoSource,
}

#[derive(Deserialize)]
struct TodoWriteItem {
    content: String,
    #[serde(default)]
    status: String,
}

/// Merge TodoWrite file + JSONL-parsed task items into a display list.
/// Completed items are filtered out.
pub fn load_for_session(session_id: &str, task_items: &[TodoItem]) -> Vec<TodoItem> {
    let mut items = load_todowrite(session_id);

    for item in task_items {
        if item.status != TodoStatus::Completed {
            items.push(item.clone());
        }
    }

    items.retain(|t| t.status != TodoStatus::Completed);
    items
}

fn load_todowrite(session_id: &str) -> Vec<TodoItem> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };
    let todos_dir = home.join(".claude").join("todos");

    let candidates = [
        todos_dir.join(format!("{}-agent-{}.json", session_id, session_id)),
        todos_dir.join(format!("{}.json", session_id)),
    ];

    for path in &candidates {
        if !path.exists() {
            continue;
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let trimmed = content.trim();
        if trimmed == "[]" || trimmed.is_empty() {
            continue;
        }
        let raw: Vec<TodoWriteItem> = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        return raw
            .into_iter()
            .map(|t| TodoItem {
                content: t.content,
                status: match t.status.as_str() {
                    "in_progress" => TodoStatus::InProgress,
                    "completed" => TodoStatus::Completed,
                    _ => TodoStatus::Pending,
                },
                source: TodoSource::TodoWrite,
            })
            .collect();
    }

    vec![]
}
