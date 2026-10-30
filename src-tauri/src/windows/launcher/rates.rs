//! The currency rates of the launcher's conversions (spec 0012 §5b): the
//! European Central Bank's, from Frankfurter, fetched at most once a day
//! and kept in a small JSON in the app's data folder
//! (`launcher_units::CACHE_FILE`).
//!
//! Everything here runs off the launcher's thread: opening the launcher
//! only asks for them (`prepare`), and the conversions read what is in
//! memory (`current`), never waiting. Without rates kept nor network, a
//! currency gives nothing and the query goes on to the other providers.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::domain::launcher_units::{self, Cache, Rates};

/// A failed fetch is tried again only after this, at the next opening.
const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
struct State {
    /// The rates in memory, and the day they were fetched.
    cache: Option<Arc<Cache>>,
    /// The file was read once.
    loaded: bool,
    /// A thread is reading or fetching them.
    busy: bool,
    /// When a fetch last failed.
    failed_at: Option<Instant>,
}

static STATE: Mutex<State> = Mutex::new(State {
    cache: None,
    loaded: false,
    busy: false,
    failed_at: None,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The rates in memory, if any (from the file or a fetch).
pub fn current() -> Option<Arc<Cache>> {
    state().cache.clone()
}

/// The day today, in UTC (`YYYY-MM-DD`): the rates are fetched once per it.
fn today() -> String {
    jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date()
        .to_string()
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join(launcher_units::CACHE_FILE))
}

/// The launcher opened: reads the rates kept, the first time, and fetches
/// them again when they are not of today, on a thread of their own. Once
/// they are in memory, `arrived` runs (on that thread).
pub fn prepare(app: &AppHandle, arrived: impl FnOnce() + Send + 'static) {
    let today = today();
    {
        let mut state = state();
        let fresh = launcher_units::is_fresh(state.cache.as_deref(), &today);
        let waiting = state.failed_at.is_some_and(|at| at.elapsed() < RETRY_AFTER);
        if state.busy || (state.loaded && (fresh || waiting)) {
            return;
        }
        state.busy = true;
    }
    let path = file(app);
    let spawned = std::thread::Builder::new()
        .name("launcher-rates".into())
        .spawn(move || {
            run(path, &today);
            arrived();
        });
    if spawned.is_err() {
        state().busy = false;
    }
}

fn run(path: Option<PathBuf>, today: &str) {
    let loaded = state().loaded;
    if !loaded {
        let kept = path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| launcher_units::read_cache(&text));
        let mut state = state();
        state.cache = kept.map(Arc::new);
        state.loaded = true;
    }
    let old = state().cache.as_deref().cloned();
    if launcher_units::is_fresh(old.as_ref(), today) {
        state().busy = false;
        return;
    }
    let fetched = fetch();
    let failed = fetched.is_none();
    let next = launcher_units::after_fetch(old, fetched, today);
    if !failed
        && let (Some(path), Some(cache)) = (&path, &next)
        && let Err(error) = std::fs::write(path, launcher_units::write_cache(cache))
    {
        log::warn!("launcher: the currency rates were not kept: {error}");
    }
    let mut state = state();
    state.cache = next.map(Arc::new);
    state.failed_at = failed.then(Instant::now);
    state.busy = false;
}

/// Frankfurter's latest rates, once.
fn fetch() -> Option<Rates> {
    match super::super::http::get(launcher_units::RATES_URL, None) {
        Ok(response) if response.status == 200 => {
            let rates = launcher_units::parse_rates(&response.body);
            if rates.is_none() {
                log::warn!("launcher: Frankfurter's answer was not the rates");
            }
            rates
        }
        Ok(response) => {
            log::warn!("launcher: Frankfurter answered {}", response.status);
            None
        }
        Err(error) => {
            log::info!("launcher: no currency rates: {error:?}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asks Frankfurter for real, once: `cargo test frankfurter_answers --
    /// --ignored --nocapture`.
    #[test]
    #[ignore = "goes to the network"]
    fn frankfurter_answers() {
        let started = Instant::now();
        let rates = fetch().expect("Frankfurter answered with the rates");
        println!(
            "Frankfurter: {} moedas, cotação de {}, em {:?}; 1 EUR = {:?} BRL, {:?} USD",
            rates.rates.len(),
            rates.date,
            started.elapsed(),
            rates.rates.get("BRL"),
            rates.rates.get("USD"),
        );
        let conversion = launcher_units::convert("100 usd em brl", Some(&rates));
        let shown = launcher_units::show(
            &conversion[0],
            ',',
            |_, _| None,
            "Cotação BCE de {d}/{m}/{y}",
        );
        println!("100 usd em brl → {} ({})", shown.title, shown.subtitle);
        assert!(rates.rates.contains_key("BRL"));
    }
}
