//! Takes the files dropped on the dock (spec 0006 B2, drop to pin).
//!
//! The dock's window (spec 0011 H3: native) takes the drop itself. The paths
//! come from the drop, so only what the user really dropped can be pinned.

use std::cell::Cell;
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

/// Puts the target on the dock's window. On the main thread, where the
/// window lives.
pub fn install(app: &AppHandle, hwnd: HWND) {
    let target: IDropTarget = DockDropTarget {
        app: app.clone(),
        accepts: Cell::new(false),
    }
    .into();
    if let Err(error) = unsafe { RegisterDragDrop(hwnd, &target) } {
        log::warn!("dock: the drop target could not be installed: {error}");
    }
}
