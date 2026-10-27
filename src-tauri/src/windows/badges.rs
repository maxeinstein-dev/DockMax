//! The notification badges, read from the Windows taskbar (spec 0016).
//!
//! A program that counts what is unread (WhatsApp, Telegram, Outlook, Teams)
//! puts the number on its button of the taskbar; UI Automation shows it as a
//! text under the button. The taskbar stays shown, only see-through, while
//! the dock holds it (`taskbar.rs`), so its buttons can be read.
//!
//! No polling of the whole taskbar: a thread of its own asks UI Automation
//! for the changes of names and of the tree of Explorer's primary taskbar,
//! and reads the buttons once, a moment after the last of a burst. A slow
//! heartbeat reads them too, in case an event never comes (and finds a
//! taskbar Explorer made again). The dock gets the buttons only when they
//! changed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Listener};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    SAFEARRAY,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCacheRequest, IUIAutomationCondition,
    IUIAutomationElement, IUIAutomationPropertyChangedEventHandler,
    IUIAutomationPropertyChangedEventHandler_Impl, IUIAutomationStructureChangedEventHandler,
    IUIAutomationStructureChangedEventHandler_Impl, StructureChangeType, TreeScope_Descendants,
    TreeScope_Subtree, UIA_AutomationIdPropertyId, UIA_ClassNamePropertyId, UIA_NamePropertyId,
    UIA_PROPERTY_ID,
};
use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::{BSTR, GUID, Interface, Ref, Result, implement};

use crate::commands::settings::CHANGED_EVENT as SETTINGS_CHANGED_EVENT;
use crate::domain::badges::{Button, app_id_of, known_folder_path, parse};

/// The class UI Automation gives the program buttons of the taskbar.
const BUTTON_CLASS: &str = "Taskbar.TaskListButtonAutomationPeer";

/// How long after the last event the buttons are read: a burst is one read.
const QUIET: Duration = Duration::from_millis(250);
/// The longest a read waits for a burst to end, when events keep coming.
const AT_MOST: Duration = Duration::from_millis(1500);
/// How often the buttons are read with no event.
const HEARTBEAT: Duration = Duration::from_secs(10);
/// How deep under a button its texts are looked for.
const DEPTH: usize = 6;

/// The buttons of the taskbar, as last read.
static BUTTONS: Mutex<Vec<Button>> = Mutex::new(Vec::new());

/// Whether the thread that reads them was started.
static STARTED: AtomicBool = AtomicBool::new(false);

/// The buttons of the taskbar, as last read: empty before the first read,
/// and when the taskbar can't be read.
pub fn buttons() -> Vec<Button> {
    BUTTONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Starts reading the badges, when the setting asks for them, now or when it
/// is turned on. Called once, at startup.
pub fn start(app: &AppHandle) {
    if crate::commands::settings::current(app).dock.show_badges {
        spawn();
    }
    let handle = app.clone();
    app.listen(SETTINGS_CHANGED_EVENT, move |_| {
        if crate::commands::settings::current(&handle).dock.show_badges {
            spawn();
        }
    });
}

/// Starts the reading thread, once in a run. Turned off, the setting leaves
/// it running: the dock stops drawing the badges.
fn spawn() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("taskbar-badges".into())
        .spawn(|| {
            if let Err(error) = run() {
                log::warn!("badges: the taskbar can not be read: {error}");
            }
        });
    if let Err(error) = spawned {
        log::warn!("badges: the reader did not start: {error}");
    }
}

/// Tells the reading thread that something under the taskbar changed.
#[implement(
    IUIAutomationStructureChangedEventHandler,
    IUIAutomationPropertyChangedEventHandler
)]
struct Changes {
    wake: Sender<()>,
}

impl IUIAutomationStructureChangedEventHandler_Impl for Changes_Impl {
    fn HandleStructureChangedEvent(
        &self,
        _sender: Ref<IUIAutomationElement>,
        _change: StructureChangeType,
        _runtime_id: *const SAFEARRAY,
    ) -> Result<()> {
        let _ = self.wake.send(());
        Ok(())
    }
}

impl IUIAutomationPropertyChangedEventHandler_Impl for Changes_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        _sender: Ref<IUIAutomationElement>,
        _property: UIA_PROPERTY_ID,
        _value: &VARIANT,
    ) -> Result<()> {
        let _ = self.wake.send(());
        Ok(())
    }
}

/// What a read asks UI Automation for, made once.
struct Reader {
    automation: IUIAutomation,
    buttons: IUIAutomationCondition,
    cache: IUIAutomationCacheRequest,
    /// The known folders an AppID made from a file names, by GUID.
    folders: HashMap<String, Option<String>>,
}

impl Reader {
    fn new() -> Result<Reader> {
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }?;
        let buttons = unsafe {
            automation.CreatePropertyCondition(
                UIA_ClassNamePropertyId,
                &VARIANT::from(BSTR::from(BUTTON_CLASS)),
            )
        }?;
        // Each button with everything under it, in one call to Explorer.
        let cache = unsafe { automation.CreateCacheRequest() }?;
        unsafe {
            cache.AddProperty(UIA_NamePropertyId)?;
            cache.AddProperty(UIA_AutomationIdPropertyId)?;
            cache.SetTreeScope(TreeScope_Subtree)?;
            cache.SetTreeFilter(&automation.RawViewCondition()?)?;
        }
        Ok(Reader {
            automation,
            buttons,
            cache,
            folders: HashMap::new(),
        })
    }

    /// The buttons under `root`, the taskbar.
    fn read(&mut self, root: &IUIAutomationElement) -> Result<Vec<Button>> {
        let found =
            unsafe { root.FindAllBuildCache(TreeScope_Descendants, &self.buttons, &self.cache) }?;
        let count = unsafe { found.Length() }?;
        let mut buttons = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
        for index in 0..count {
            let element = unsafe { found.GetElement(index) }?;
            let name = unsafe { element.CachedName() }
                .map(|name| name.to_string())
                .unwrap_or_default();
            let app_id = unsafe { element.CachedAutomationId() }
                .ok()
                .and_then(|id| app_id_of(&id.to_string()));
            let mut texts = Vec::new();
            texts_under(&element, DEPTH, &mut texts);
            let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
            let path = app_id.as_deref().and_then(|id| self.path_of(id));
            buttons.push(Button {
                name,
                app_id,
                path,
                badge: parse(&texts),
            });
        }
        Ok(buttons)
    }

    /// The file an AppID made from one names (`{GUID}\folder\program.exe`),
    /// lowercased, its known folder resolved.
    fn path_of(&mut self, app_id: &str) -> Option<String> {
        let (guid, rest) = known_folder_path(app_id)?;
        let folder = self
            .folders
            .entry(guid.to_owned())
            .or_insert_with(|| known_folder(guid))
            .as_ref()?;
        Some(format!("{folder}\\{rest}").to_lowercase())
    }
}

/// The path of the known folder `guid` (`{…}`, as an AppID has it).
fn known_folder(guid: &str) -> Option<String> {
    let id = GUID::try_from(guid.trim_start_matches('{').trim_end_matches('}')).ok()?;
    let path = unsafe { SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None) }.ok()?;
    let text = unsafe { path.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(path.0 as _)) };
    text
}

/// The names of everything under `element` (cached), depth first.
fn texts_under(element: &IUIAutomationElement, depth: usize, texts: &mut Vec<String>) {
    if depth == 0 {
        return;
    }
    // No children is an error, or no array.
    let Ok(children) = (unsafe { element.GetCachedChildren() }) else {
        return;
    };
    let count = unsafe { children.Length() }.unwrap_or(0);
    for index in 0..count {
        let Ok(child) = (unsafe { children.GetElement(index) }) else {
            continue;
        };
        if let Ok(name) = unsafe { child.CachedName() } {
            let name = name.to_string();
            if !name.is_empty() {
                texts.push(name);
            }
        }
        texts_under(&child, depth - 1, texts);
    }
}

/// The taskbar being followed: its window and its element, with the
/// handlers on it.
struct Attached {
    hwnd: isize,
    root: IUIAutomationElement,
}

/// The reading thread's life.
fn run() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
    let mut reader = Reader::new()?;
    let (wake, woken) = mpsc::channel();
    let structure: IUIAutomationStructureChangedEventHandler = Changes { wake }.into();
    let property: IUIAutomationPropertyChangedEventHandler = structure.cast()?;
    let mut attached: Option<Attached> = None;
    let mut failed = false;
    loop {
        // Explorer's taskbar, never the dock's proxy: the primary first.
        let taskbar = super::taskbar::taskbar_windows()
            .first()
            .map(|hwnd| hwnd.0 as isize);
        if attached.as_ref().map(|a| a.hwnd) != taskbar {
            if attached.take().is_some() {
                let _ = unsafe { reader.automation.RemoveAllEventHandlers() };
            }
            attached = taskbar.and_then(|hwnd| {
                attach(&reader.automation, hwnd, &structure, &property)
                    .inspect_err(|error| log::warn!("badges: no events from the taskbar: {error}"))
                    .ok()
            });
        }
        let buttons = match &attached {
            Some(taskbar) => match reader.read(&taskbar.root) {
                Ok(buttons) => {
                    failed = false;
                    buttons
                }
                Err(error) => {
                    // Once per failure, not each heartbeat.
                    if !failed {
                        log::warn!("badges: the taskbar could not be read: {error}");
                    }
                    failed = true;
                    // Found again on the next turn (Explorer may be restarting).
                    attached = None;
                    let _ = unsafe { reader.automation.RemoveAllEventHandlers() };
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        publish(buttons);
        match woken.recv_timeout(HEARTBEAT) {
            Ok(()) => settle(&woken),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

/// Waits for a burst of events to end, at most `AT_MOST`.
fn settle(woken: &mpsc::Receiver<()>) {
    let started = Instant::now();
    while started.elapsed() < AT_MOST {
        match woken.recv_timeout(QUIET) {
            Ok(()) => continue,
            Err(_) => return,
        }
    }
    // Still coming: drop what is queued, the read sees it all.
    while woken.try_recv().is_ok() {}
}

/// Follows the taskbar `hwnd`: its element, and the changes under it.
fn attach(
    automation: &IUIAutomation,
    hwnd: isize,
    structure: &IUIAutomationStructureChangedEventHandler,
    property: &IUIAutomationPropertyChangedEventHandler,
) -> Result<Attached> {
    let root =
        unsafe { automation.ElementFromHandle(windows::Win32::Foundation::HWND(hwnd as _)) }?;
    unsafe {
        automation.AddStructureChangedEventHandler(&root, TreeScope_Subtree, None, structure)?;
        automation.AddPropertyChangedEventHandlerNativeArray(
            &root,
            TreeScope_Subtree,
            None,
            property,
            &[UIA_NamePropertyId],
        )?;
    }
    Ok(Attached { hwnd, root })
}

/// Keeps the buttons read and, when they changed, tells the docks.
fn publish(buttons: Vec<Button>) {
    {
        let mut known = BUTTONS.lock().unwrap_or_else(PoisonError::into_inner);
        if *known == buttons {
            return;
        }
        *known = buttons;
    }
    super::native_dock::badges_changed();
}

#[cfg(all(test, windows))]
mod probe {
    use super::*;

    /// Not a check: prints the buttons of the taskbar as read, with their
    /// badges. Reads only.
    /// `cargo test spike_taskbar_badges -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the taskbar of the machine"]
    fn spike_taskbar_badges() {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .unwrap();
        let mut reader = Reader::new().unwrap();
        for hwnd in super::super::taskbar::taskbar_windows() {
            let root = unsafe { reader.automation.ElementFromHandle(hwnd) }.unwrap();
            let started = Instant::now();
            let buttons = reader.read(&root).unwrap();
            println!(
                "{hwnd:?}: {} buttons in {:?}",
                buttons.len(),
                started.elapsed()
            );
            for button in buttons {
                println!("  {button:?}");
            }
        }
    }
}
