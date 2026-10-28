//! The Calendário's tasks (spec 0017 F2a-3): Google Tasks' lists and tasks
//! read into the ones the panel shows, today's and the late ones, not
//! done, and the addresses to read and to tick them off. Pure.

use serde::{Deserialize, Serialize};

pub const LISTS_URL: &str = "https://tasks.googleapis.com/tasks/v1/users/@me/lists";

/// A task list of an account.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTaskList {
    pub id: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTaskLists {
    #[serde(default)]
    pub items: Vec<GoogleTaskList>,
}

/// An item of `tasks.list`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTask {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    /// `2026-10-28T00:00:00.000Z`: only the day counts.
    pub due: Option<String>,
    pub status: Option<String>,
    pub web_view_link: Option<String>,
    /// Its place in the list, as the person ordered it in Google Tasks:
    /// text that sorts in that order.
    #[serde(default)]
    pub position: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTasks {
    #[serde(default)]
    pub items: Vec<GoogleTask>,
}

/// A task as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub account: String,
    pub list: String,
    pub list_name: String,
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    /// The day it is due, `2026-10-28`.
    pub due: String,
    /// Due before today.
    pub late: bool,
    /// The task in Google Tasks.
    pub link: Option<String>,
    /// Its place in its list ([`GoogleTask::position`]).
    #[serde(skip)]
    pub position: String,
}

/// The local day of `now`, at `offset` seconds from UTC.
pub fn today(now: i64, offset: i64) -> Option<jiff::civil::Date> {
    jiff::Timestamp::from_second(now + offset)
        .ok()
        .map(|at| at.to_zoned(jiff::tz::TimeZone::UTC).date())
}

/// The address of a list's tasks not done and due up to the end of
/// `today` (Google keeps a due day as midnight UTC).
pub fn tasks_url(list: &str, today: jiff::civil::Date) -> String {
    let mut url =
        url::Url::parse("https://tasks.googleapis.com/tasks/v1/lists/").expect("a valid constant");
    url.path_segments_mut()
        .expect("an https address")
        .pop_if_empty()
        .push(list)
        .push("tasks");
    let day_after = today.tomorrow().unwrap_or(today);
    url.query_pairs_mut()
        .append_pair("showCompleted", "false")
        .append_pair("showHidden", "false")
        .append_pair("maxResults", "100")
        .append_pair("dueMax", &format!("{day_after}T00:00:00.000Z"));
    url.into()
}

/// The address of one task, to tick it off.
pub fn task_url(list: &str, id: &str) -> String {
    let mut url =
        url::Url::parse("https://tasks.googleapis.com/tasks/v1/lists/").expect("a valid constant");
    url.path_segments_mut()
        .expect("an https address")
        .pop_if_empty()
        .push(list)
        .push("tasks")
        .push(id);
    url.into()
}

/// What ticks a task off.
pub const COMPLETED: &str = r#"{"status":"completed"}"#;

/// Google's tasks of a list as the panel's: due today or before, not
/// done, not deleted; the late ones first, then by title.
pub fn read_tasks(
    items: Vec<GoogleTask>,
    account: &str,
    list: &GoogleTaskList,
    today: jiff::civil::Date,
) -> Vec<Task> {
    items
        .into_iter()
        .filter(|task| !task.deleted && !task.hidden && task.status.as_deref() != Some("completed"))
        .filter_map(|task| {
            let due: jiff::civil::Date = task.due.as_deref()?.get(..10)?.parse().ok()?;
            (due <= today).then(|| Task {
                account: account.to_owned(),
                list: list.id.clone(),
                list_name: list.title.clone(),
                id: task.id,
                title: task.title.trim().to_owned(),
                notes: task.notes.filter(|notes| !notes.trim().is_empty()),
                due: due.to_string(),
                late: due < today,
                link: task.web_view_link,
                position: task.position,
            })
        })
        .collect()
}

/// All the accounts' tasks in the order the panel shows them: the late
/// ones first, by day; then, in each list, the order the person gave them
/// in Google Tasks. The API gives no time of day (Google keeps only the
/// date), so the list's order is the closest to the day's order.
pub fn sort(mut tasks: Vec<Task>) -> Vec<Task> {
    tasks.sort_by(|a, b| {
        (!a.late, &a.due, &a.list, &a.position).cmp(&(!b.late, &b.due, &b.list, &b.position))
    });
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> GoogleTaskList {
        GoogleTaskList {
            id: "L1".into(),
            title: "Minhas tarefas".into(),
        }
    }

    #[test]
    fn todays_and_late_tasks_show_the_done_and_later_ones_do_not() {
        let items: GoogleTasks = serde_json::from_str(
            r#"{"items":[
            {"id":"a","title":"Pagar boleto","due":"2026-10-27T00:00:00.000Z","status":"needsAction"},
            {"id":"b","title":"Ligar","due":"2026-10-28T00:00:00.000Z","status":"needsAction","notes":"  "},
            {"id":"c","title":"Depois","due":"2026-10-29T00:00:00.000Z","status":"needsAction"},
            {"id":"d","title":"Feita","due":"2026-10-28T00:00:00.000Z","status":"completed"},
            {"id":"e","title":"Sem data","status":"needsAction"},
            {"id":"f","title":"Apagada","due":"2026-10-28T00:00:00.000Z","deleted":true}
        ]}"#,
        )
        .unwrap();
        let today: jiff::civil::Date = "2026-10-28".parse().unwrap();
        let tasks = sort(read_tasks(items.items, "max@example.com", &list(), today));
        let shown: Vec<_> = tasks
            .iter()
            .map(|task| (task.title.as_str(), task.late))
            .collect();
        assert_eq!(shown, [("Pagar boleto", true), ("Ligar", false)]);
        assert_eq!(tasks[1].notes, None);
        assert_eq!(tasks[1].list_name, "Minhas tarefas");
    }

    #[test]
    fn todays_tasks_keep_the_order_of_their_list() {
        let items: GoogleTasks = serde_json::from_str(
            r#"{"items":[
            {"id":"a","title":"Academia (20h)","due":"2026-10-28T00:00:00.000Z","position":"00000000000000000002"},
            {"id":"b","title":"Reunião (9h)","due":"2026-10-28T00:00:00.000Z","position":"00000000000000000000"},
            {"id":"c","title":"Almoço (12h)","due":"2026-10-28T00:00:00.000Z","position":"00000000000000000001"}
        ]}"#,
        )
        .unwrap();
        let today: jiff::civil::Date = "2026-10-28".parse().unwrap();
        let tasks = sort(read_tasks(items.items, "max@example.com", &list(), today));
        let titles: Vec<_> = tasks.iter().map(|task| task.title.as_str()).collect();
        assert_eq!(titles, ["Reunião (9h)", "Almoço (12h)", "Academia (20h)"]);
    }

    #[test]
    fn the_addresses_name_the_list_the_task_and_the_day() {
        let today: jiff::civil::Date = "2026-10-28".parse().unwrap();
        let url = url::Url::parse(&tasks_url("MDE x/y", today)).unwrap();
        assert_eq!(url.path(), "/tasks/v1/lists/MDE%20x%2Fy/tasks");
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs["dueMax"], "2026-10-29T00:00:00.000Z");
        assert_eq!(pairs["showCompleted"], "false");
        assert_eq!(
            task_url("L1", "t/1"),
            "https://tasks.googleapis.com/tasks/v1/lists/L1/tasks/t%2F1"
        );
    }

    #[test]
    fn today_is_the_local_day() {
        // 2026-10-28 01:30 in Brasília is 04:30 UTC: the same day.
        assert_eq!(
            today(1_793_163_000, -3 * 3600).map(|day| day.to_string()),
            Some("2026-10-28".into())
        );
        // 23:30 in Brasília is already the next day in UTC.
        assert_eq!(
            today(1_793_241_000, -3 * 3600).map(|day| day.to_string()),
            Some("2026-10-28".into())
        );
    }
}
