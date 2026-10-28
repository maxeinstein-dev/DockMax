//! The Lista de tarefas (spec 0017 F3): tasks with a mark, kept in the
//! widget's config (so in the database, with the others), the number still
//! open on the tile.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// Most tasks in a list, the longest text of one, and of an ID.
pub const MAX_TASKS: usize = 200;
pub const MAX_TASK_CHARS: usize = 200;
const MAX_ID_CHARS: usize = 40;

/// A task: an ID the panel makes, its text, and whether it is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TodoConfig {
    /// In the author's order.
    pub items: Vec<TodoItem>,
}

impl TodoConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        let fair = |text: &str, most: usize| {
            let count = text.chars().count();
            text.trim() == text && count > 0 && count <= most && !text.chars().any(char::is_control)
        };
        let mut ids: Vec<&str> = self.items.iter().map(|item| item.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        if self.items.len() <= MAX_TASKS
            && ids.len() == self.items.len()
            && self
                .items
                .iter()
                .all(|item| fair(&item.id, MAX_ID_CHARS) && fair(&item.text, MAX_TASK_CHARS))
        {
            Ok(())
        } else {
            Err(WidgetError::InvalidTasks)
        }
    }

    /// The tasks still open: the tile's number.
    pub fn open(&self) -> usize {
        self.items.iter().filter(|item| !item.done).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, text: &str, done: bool) -> TodoItem {
        TodoItem {
            id: id.into(),
            text: text.into(),
            done,
        }
    }

    #[test]
    fn it_counts_the_open_tasks() {
        let config = TodoConfig {
            items: vec![
                item("a", "Pão", false),
                item("b", "Leite", true),
                item("c", "Café", false),
            ],
        };
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.open(), 2);
        assert_eq!(TodoConfig::default().open(), 0);
    }

    #[test]
    fn a_task_needs_a_tidy_text_and_its_own_id() {
        let long = "x".repeat(MAX_TASK_CHARS + 1);
        for items in [
            vec![item("a", "", false)],
            vec![item("a", " Pão", false)],
            vec![item("a", "a\nb", false)],
            vec![item("a", &long, false)],
            vec![item("", "Pão", false)],
            vec![item("a", "Pão", false), item("a", "Leite", false)],
        ] {
            assert_eq!(
                TodoConfig { items }.validate(),
                Err(WidgetError::InvalidTasks)
            );
        }
        let many = (0..=MAX_TASKS)
            .map(|n| item(&n.to_string(), "x", false))
            .collect();
        assert_eq!(
            TodoConfig { items: many }.validate(),
            Err(WidgetError::InvalidTasks)
        );
    }
}
