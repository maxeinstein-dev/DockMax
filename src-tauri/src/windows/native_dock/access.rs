//! The dock for assistive technology (spec 0011 H3): a UI Automation
//! provider, so the Narrator and the other screen readers read the dock as a
//! toolbar of buttons, each with its name, and press them.
//!
//! What the provider says comes from `ENTRIES`, which the dock keeps up to
//! date; UI Automation may call from its own threads. A press goes back to the
//! dock through `post`.

// The patterns below match Windows' constants, which keep their own names.
#![allow(non_upper_case_globals)]

use std::sync::{Mutex, PoisonError};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{SafeArrayCreateVector, SafeArrayPutElement};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    IInvokeProvider, IInvokeProvider_Impl, IRawElementProviderFragment,
    IRawElementProviderFragment_Impl, IRawElementProviderFragmentRoot,
    IRawElementProviderFragmentRoot_Impl, IRawElementProviderSimple,
    IRawElementProviderSimple_Impl, NavigateDirection, NavigateDirection_FirstChild,
    NavigateDirection_LastChild, NavigateDirection_NextSibling, NavigateDirection_Parent,
    NavigateDirection_PreviousSibling, ProviderOptions, ProviderOptions_ServerSideProvider,
    UIA_AutomationIdPropertyId, UIA_ButtonControlTypeId, UIA_ControlTypePropertyId,
    UIA_InvokePatternId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId,
    UIA_NamePropertyId, UIA_PATTERN_ID, UIA_PROPERTY_ID, UIA_ToolBarControlTypeId,
    UiaAppendRuntimeId, UiaHostProviderFromHwnd, UiaRect, UiaReturnRawElementProvider,
    UiaRootObjectId,
};
use windows::core::{BSTR, Error, IUnknown, IUnknownImpl, Interface, Result, implement};

/// One item as assistive technology sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The item's key: stable across changes of the list.
    pub key: String,
    pub name: String,
    /// Its box on the screen, in physical pixels.
    pub rect: (f64, f64, f64, f64),
}

/// Each dock's items, by its window (one dock per monitor).
static ENTRIES: Mutex<Vec<(isize, Vec<Entry>)>> = Mutex::new(Vec::new());

/// What the dock of the window `hwnd` shows now.
pub fn publish(hwnd: isize, entries: Vec<Entry>) {
    let mut all = ENTRIES.lock().unwrap_or_else(PoisonError::into_inner);
    all.retain(|(window, _)| *window != hwnd);
    all.push((hwnd, entries));
}

/// The dock of the window `hwnd` is gone.
pub fn forget(hwnd: isize) {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|(window, _)| *window != hwnd);
}

fn entries(hwnd: isize) -> Vec<Entry> {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(window, _)| *window == hwnd)
        .map(|(_, entries)| entries.clone())
        .unwrap_or_default()
}

/// The dock's name for assistive technology.
const LABEL: &str = "Dock";

/// Answers `WM_GETOBJECT` asking for UI Automation's root; `None` for any
/// other object.
pub fn get_object(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if lparam.0 as i32 != UiaRootObjectId {
        return None;
    }
    let root: IRawElementProviderSimple = Root {
        hwnd: hwnd.0 as isize,
    }
    .into();
    Some(unsafe { UiaReturnRawElementProvider(hwnd, wparam, lparam, &root) })
}

/// Nothing: a null out-pointer with success.
fn none<T>() -> Result<T> {
    Err(Error::empty())
}

fn item(hwnd: isize, key: String) -> IRawElementProviderFragment {
    Item { hwnd, key }.into()
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRawElementProviderFragmentRoot
)]
struct Root {
    hwnd: isize,
}

impl IRawElementProviderSimple_Impl for Root_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, _pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        none()
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(match property {
            UIA_ControlTypePropertyId => VARIANT::from(UIA_ToolBarControlTypeId.0),
            UIA_NamePropertyId => VARIANT::from(BSTR::from(LABEL)),
            _ => VARIANT::default(),
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as _)) }
    }
}

impl IRawElementProviderFragment_Impl for Root_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        let entries = entries(self.hwnd);
        let found = match direction {
            NavigateDirection_FirstChild => entries.first(),
            NavigateDirection_LastChild => entries.last(),
            _ => None,
        };
        match found {
            Some(entry) => Ok(item(self.hwnd, entry.key.clone())),
            None => none(),
        }
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        // The window's own: UI Automation takes it from the host.
        Ok(std::ptr::null_mut())
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        // The window's own, from the host.
        Ok(UiaRect::default())
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> Result<()> {
        Ok(())
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        Ok(self.to_interface())
    }
}

impl IRawElementProviderFragmentRoot_Impl for Root_Impl {
    fn ElementProviderFromPoint(&self, x: f64, y: f64) -> Result<IRawElementProviderFragment> {
        let found = entries(self.hwnd).into_iter().find(|entry| {
            let (left, top, width, height) = entry.rect;
            x >= left && y >= top && x < left + width && y < top + height
        });
        match found {
            Some(entry) => Ok(item(self.hwnd, entry.key)),
            None => none(),
        }
    }

    fn GetFocus(&self) -> Result<IRawElementProviderFragment> {
        none()
    }
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider
)]
struct Item {
    hwnd: isize,
    key: String,
}

impl Item_Impl {
    fn entry(&self) -> Option<(usize, Entry)> {
        entries(self.hwnd)
            .into_iter()
            .enumerate()
            .find(|(_, entry)| entry.key == self.key)
    }
}

/// A number for the key, the same each time it is asked.
fn number_of(key: &str) -> i32 {
    // FNV-1a: stable across runs, unlike the standard hasher.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in key.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash as i32
}

impl IRawElementProviderSimple_Impl for Item_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        if pattern == UIA_InvokePatternId {
            self.to_interface::<IInvokeProvider>().cast()
        } else {
            none()
        }
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        let Some((_, entry)) = self.entry() else {
            return Ok(VARIANT::default());
        };
        Ok(match property {
            UIA_ControlTypePropertyId => VARIANT::from(UIA_ButtonControlTypeId.0),
            UIA_NamePropertyId => VARIANT::from(BSTR::from(entry.name.as_str())),
            UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from(entry.key.as_str())),
            UIA_IsEnabledPropertyId => VARIANT::from(true),
            // The dock never takes the keyboard (it is never activated).
            UIA_IsKeyboardFocusablePropertyId => VARIANT::from(false),
            _ => VARIANT::default(),
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        none()
    }
}

impl IRawElementProviderFragment_Impl for Item_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        if direction == NavigateDirection_Parent {
            let root: IRawElementProviderFragment = Root { hwnd: self.hwnd }.into();
            return Ok(root);
        }
        let entries = entries(self.hwnd);
        let Some(index) = entries.iter().position(|entry| entry.key == self.key) else {
            return none();
        };
        let found = match direction {
            NavigateDirection_NextSibling => entries.get(index + 1),
            NavigateDirection_PreviousSibling => index.checked_sub(1).and_then(|i| entries.get(i)),
            _ => None,
        };
        match found {
            Some(entry) => Ok(item(self.hwnd, entry.key.clone())),
            None => none(),
        }
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        unsafe {
            let ids = SafeArrayCreateVector(VT_I4, 0, 2);
            if ids.is_null() {
                return none();
            }
            for (index, value) in [UiaAppendRuntimeId as i32, number_of(&self.key)]
                .into_iter()
                .enumerate()
            {
                let at = index as i32;
                SafeArrayPutElement(ids, &at, std::ptr::from_ref(&value).cast())?;
            }
            Ok(ids)
        }
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        Ok(match self.entry() {
            Some((_, entry)) => {
                let (left, top, width, height) = entry.rect;
                UiaRect {
                    left,
                    top,
                    width,
                    height,
                }
            }
            None => UiaRect::default(),
        })
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> Result<()> {
        Ok(())
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        Ok(Root { hwnd: self.hwnd }.into())
    }
}

impl IInvokeProvider_Impl for Item_Impl {
    fn Invoke(&self) -> Result<()> {
        // Only this dock: each monitor has the same workspace dots.
        let key = self.key.clone();
        super::post_to(self.hwnd, move |dock| dock.press(&key));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_items_number_is_stable_and_tells_keys_apart() {
        assert_eq!(number_of("about"), number_of("about"));
        assert_ne!(number_of("about"), number_of("settings"));
    }
}
