//! Takes the files dropped on the dock (spec 0006 B2, drop to pin).
//!
//! Tauri hands a drop to the page's own windows (WebView2 keeps the page in
//! a few child windows) by registering a drop target on the ones that exist
//! when the webview is built; the browser creates some of them a moment
//! later, so a drop over the bar could land on a window with no target and
//! be refused. Once the page has loaded this puts the dock's own target on
//! every child window, so the drop is heard wherever it lands. The paths come
//! from the drop itself, not from the page, so only what the user really
//! dropped can be pinned.

use std::cell::Cell;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use tauri::{AppHandle, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, LPARAM, POINTL};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl,
    RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;
use windows::core::{BOOL, Ref, implement};

use crate::commands;

/// What was dragged: the paths in the data, if it carries files.
fn paths_of(data: &IDataObject) -> Vec<PathBuf> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let Ok(mut medium) = (unsafe { data.GetData(&format) }) else {
        return Vec::new();
    };
    let drop = HDROP(unsafe { medium.u.hGlobal.0 } as _);
    let count = unsafe { DragQueryFileW(drop, 0xFFFF_FFFF, None) };
    let mut paths = Vec::new();
    for index in 0..count {
        let length = unsafe { DragQueryFileW(drop, index, None) } as usize;
        let mut buffer = vec![0u16; length + 1];
        unsafe { DragQueryFileW(drop, index, Some(&mut buffer)) };
        paths.push(OsString::from_wide(&buffer[..length]).into());
    }
    unsafe { ReleaseStgMedium(&mut medium) };
    paths
}

fn effect_for(accepts: bool) -> DROPEFFECT {
    if accepts {
        DROPEFFECT_COPY
    } else {
        DROPEFFECT_NONE
    }
}

#[implement(IDropTarget)]
struct DockDropTarget {
    app: AppHandle,
    /// What is over the dock has something that can be pinned.
    accepts: Cell<bool>,
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DockDropTarget_Impl {
    fn DragEnter(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let accepts = pDataObj
            .as_ref()
            .is_some_and(|data| !commands::apps::droppable(&paths_of(data)).is_empty());
        self.accepts.set(accepts);
        unsafe { *pdwEffect = effect_for(accepts) };
        Ok(())
    }

    fn DragOver(
        &self,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe { *pdwEffect = effect_for(self.accepts.get()) };
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        self.accepts.set(false);
        Ok(())
    }

    fn Drop(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let accepts = self.accepts.replace(false);
        if let (true, Some(data)) = (accepts, pDataObj.as_ref()) {
            commands::apps::pin_dropped(&self.app, paths_of(data));
        }
        unsafe { *pdwEffect = effect_for(accepts) };
        Ok(())
    }
}

/// Puts the target on every child window of `window` (the page's own
/// windows), in place of whatever was registered there. Call it once the page
/// has loaded; the page may reload, so it can be called again.
pub fn install(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        log::warn!("dock: no window to take drops on");
        return;
    };
    let top = hwnd.0 as isize;
    let app = window.app_handle().clone();
    // The windows belong to the thread of the event loop.
    let queued = window.run_on_main_thread(move || {
        let found = register_all(HWND(top as _), &app);
        if found == 0 {
            log::warn!("dock: no child window took drops");
        }
    });
    if let Err(error) = queued {
        log::warn!("dock: the drop target could not be installed: {error}");
    }
}

fn register_all(top: HWND, app: &AppHandle) -> usize {
    let mut children: Vec<HWND> = Vec::new();
    unsafe extern "system" fn collect(child: HWND, state: LPARAM) -> BOOL {
        let children = unsafe { &mut *(state.0 as *mut Vec<HWND>) };
        children.push(child);
        true.into()
    }
    let _ = unsafe {
        EnumChildWindows(
            Some(top),
            Some(collect),
            LPARAM(&mut children as *mut Vec<HWND> as isize),
        )
    };
    let mut registered = 0;
    for child in children {
        let target: IDropTarget = DockDropTarget {
            app: app.clone(),
            accepts: Cell::new(false),
        }
        .into();
        // Whatever was there goes (the revoke fails if nothing was).
        let _ = unsafe { RevokeDragDrop(child) };
        if unsafe { RegisterDragDrop(child, &target) }.is_ok() {
            registered += 1;
        }
    }
    registered
}
