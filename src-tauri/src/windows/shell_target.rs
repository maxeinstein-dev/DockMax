//! What a tile of the dock was let go on outside the dock (spec 0018): a
//! folder or a file in a File Explorer window or on the desktop, or the
//! folder the window shows.

use std::path::PathBuf;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, IDispatch, IServiceProvider};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, UIA_ListItemControlTypeId,
};
use windows::Win32::UI::Shell::{
    IFolderView2, IPersistFolder2, IShellBrowser, IShellItem, IShellWindows, IWebBrowserApp,
    SHCreateItemFromIDList, SID_STopLevelBrowser, SIGDN_FILESYSPATH, SIGDN_NORMALDISPLAY,
    SVGIO_ALLVIEW, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetClassNameW, IsChild, IsWindowVisible, WindowFromPoint,
};
use windows::core::Interface;

fn class_of(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) } as usize;
    String::from_utf16_lossy(&buffer[..length])
}

fn path_of(item: &IShellItem) -> Option<PathBuf> {
    let text = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    let path = unsafe { text.to_string() }.ok();
    unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(text.0.cast())) };
    path.map(PathBuf::from)
}

fn name_of(item: &IShellItem) -> Option<String> {
    let text = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }.ok()?;
    let name = unsafe { text.to_string() }.ok();
    unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(text.0.cast())) };
    name
}

/// The view of a window of the shell (`browser`), with its window: what a
/// File Explorer window shows, in the tab in front, or the desktop's.
fn view_of(browser: &IDispatch) -> Option<(IFolderView2, HWND)> {
    let provider: IServiceProvider = browser.cast().ok()?;
    let shell: IShellBrowser = unsafe { provider.QueryService(&SID_STopLevelBrowser) }.ok()?;
    let view = unsafe { shell.QueryActiveShellView() }.ok()?;
    let window = unsafe { view.GetWindow() }.ok()?;
    Some((view.cast().ok()?, window))
}

/// The folder a view shows.
fn folder_of(view: &IFolderView2) -> Option<PathBuf> {
    let folder: IPersistFolder2 = unsafe { view.GetFolder() }.ok()?;
    let list = unsafe { folder.GetCurFolder() }.ok()?;
    let item: Option<IShellItem> = unsafe { SHCreateItemFromIDList(list) }.ok();
    unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(list.cast_const().cast())) };
    path_of(&item?)
}

/// The item of `view` named `name`, as the view shows it.
fn item_named(view: &IFolderView2, name: &str) -> Option<PathBuf> {
    let count = unsafe { view.ItemCount(SVGIO_ALLVIEW) }.ok()?;
    (0..count).find_map(|index| {
        let item: IShellItem = unsafe { view.GetItem(index) }.ok()?;
        (name_of(&item).as_deref() == Some(name))
            .then(|| path_of(&item))
            .flatten()
    })
}

/// The name of the item of a list under `point`, if a list item is there.
fn list_item_at(point: POINT) -> Option<String> {
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) }.ok()?;
    let walker = unsafe { automation.ControlViewWalker() }.ok()?;
    let mut element: IUIAutomationElement = unsafe { automation.ElementFromPoint(point) }.ok()?;
    // From the cell under the pointer up to its row, a few levels at most.
    for _ in 0..6 {
        if unsafe { element.CurrentControlType() }.ok() == Some(UIA_ListItemControlTypeId) {
            let name = unsafe { element.CurrentName() }.ok()?.to_string();
            return (!name.is_empty()).then_some(name);
        }
        element = unsafe { walker.GetParentElement(&element) }.ok()?;
    }
    None
}

/// The path at `point` in `view` (shown in `window`): the item under the
/// pointer, or the folder the view shows.
fn path_in(view: &IFolderView2, window: HWND, under: HWND, point: POINT) -> Option<PathBuf> {
    let on_view = under == window || unsafe { IsChild(window, under) }.as_bool();
    if on_view && let Some(path) = list_item_at(point).and_then(|name| item_named(view, &name)) {
        return Some(path);
    }
    folder_of(view)
}

/// The folder or file at the screen point `(x, y)`, in physical pixels:
/// in a File Explorer window, the item under it or the folder the window
/// shows; on the desktop, the item under it or the desktop folder. `None`
/// anywhere else.
pub fn at(x: i32, y: i32) -> Option<PathBuf> {
    super::apps::init_com();
    let point = POINT { x, y };
    let under = unsafe { WindowFromPoint(point) };
    let root = unsafe { GetAncestor(under, GA_ROOT) };
    let windows: IShellWindows =
        unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_ALL) }.ok()?;
    match class_of(root).as_str() {
        "CabinetWClass" => {
            let count = unsafe { windows.Count() }.ok()?;
            (0..count).find_map(|index| {
                let browser = unsafe { windows.Item(&VARIANT::from(index)) }.ok()?;
                let app: IWebBrowserApp = browser.cast().ok()?;
                if unsafe { app.HWND() }.ok()?.0 != root.0 as isize {
                    return None;
                }
                // The tabs share the window: the one in front shows.
                let (view, window) = view_of(&browser)?;
                if !unsafe { IsWindowVisible(window) }.as_bool() {
                    return None;
                }
                path_in(&view, window, under, point)
            })
        }
        "Progman" | "WorkerW" => {
            let mut hwnd = 0;
            let desktop = unsafe {
                windows.FindWindowSW(
                    &VARIANT::from(0i32),
                    &VARIANT::default(),
                    SWC_DESKTOP,
                    &mut hwnd,
                    SWFO_NEEDDISPATCH,
                )
            }
            .ok()?;
            let (view, window) = view_of(&desktop)?;
            path_in(&view, window, under, point)
        }
        _ => None,
    }
}
