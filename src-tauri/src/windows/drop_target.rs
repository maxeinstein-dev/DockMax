//! Takes the files dropped on the dock: on a program's tile they open with
//! it (spec 0018); anywhere else, a program among them is pinned (spec 0006
//! B2, drop to pin).
//!
//! The dock's window (spec 0011 H3: native) takes the drop itself. The paths
//! come from the drop, so only what the user really dropped can be pinned
//! or opened.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use tauri::AppHandle;
use windows::Win32::Foundation::{HWND, POINTL};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl,
    RegisterDragDrop, ReleaseStgMedium,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::core::{Ref, implement};

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
    /// The dock's window: what is dropped goes to the workspace its monitor
    /// shows.
    hwnd: isize,
    /// What is over the dock has something that can be pinned.
    accepts: Cell<bool>,
    /// The paths dragged over the dock.
    paths: RefCell<Vec<PathBuf>>,
}

impl DockDropTarget_Impl {
    /// What a drop at `at` does: open with the program under it, pin, or
    /// nothing.
    fn effect_at(&self, at: &POINTL) -> (Option<String>, DROPEFFECT) {
        let program = if self.paths.borrow().is_empty() {
            None
        } else {
            super::native_dock::drop_over(self.hwnd, Some((at.x, at.y)))
        };
        let effect = if program.is_some() {
            DROPEFFECT_COPY
        } else {
            effect_for(self.accepts.get())
        };
        (program, effect)
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DockDropTarget_Impl {
    fn DragEnter(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let paths = pDataObj.as_ref().map(paths_of).unwrap_or_default();
        self.accepts
            .set(!commands::apps::droppable(&paths).is_empty());
        *self.paths.borrow_mut() = paths;
        let (_, effect) = self.effect_at(pt);
        unsafe { *pdwEffect = effect };
        Ok(())
    }

    fn DragOver(
        &self,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let (_, effect) = self.effect_at(pt);
        unsafe { *pdwEffect = effect };
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        self.accepts.set(false);
        self.paths.borrow_mut().clear();
        super::native_dock::drop_over(self.hwnd, None);
        Ok(())
    }

    fn Drop(
        &self,
        pDataObj: Ref<'_, IDataObject>,
        _grfKeyState: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdwEffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let (program, effect) = self.effect_at(pt);
        super::native_dock::drop_over(self.hwnd, None);
        let paths = std::mem::take(&mut *self.paths.borrow_mut());
        let accepts = self.accepts.replace(false);
        if let Some(program) = program {
            // On a program's tile: it opens them.
            super::open_with::open_off_thread(program, paths);
            unsafe { *pdwEffect = effect };
            return Ok(());
        }
        if let (true, Some(data)) = (accepts, pDataObj.as_ref()) {
            commands::apps::pin_dropped(
                &self.app,
                paths_of(data),
                super::native_dock::monitor_of(self.hwnd),
            );
        }
        unsafe { *pdwEffect = effect_for(accepts) };
        Ok(())
    }
}

/// Puts the target on the dock's window. On the main thread, where the
/// window lives.
pub fn install(app: &AppHandle, hwnd: HWND) {
    let target: IDropTarget = DockDropTarget {
        app: app.clone(),
        hwnd: hwnd.0 as isize,
        accepts: Cell::new(false),
        paths: RefCell::new(Vec::new()),
    }
    .into();
    if let Err(error) = unsafe { RegisterDragDrop(hwnd, &target) } {
        log::warn!("dock: the drop target could not be installed: {error}");
    }
}
