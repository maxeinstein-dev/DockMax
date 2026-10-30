//! A menu for assistive technology (spec 0014 R3), as Windows' own menus
//! show: a menu with its items (name, shortcut, whether it is checked,
//! whether it can be chosen) and separators; the item under the keyboard has
//! the focus, and an item can be invoked.
//!
//! What the provider says comes from `LINES`, which the menu keeps; UI
//! Automation may call from its own threads. An invoke or a focus goes back
//! to the menu's window as a message.

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
    IRawElementProviderSimple_Impl, IToggleProvider, IToggleProvider_Impl, NavigateDirection,
    NavigateDirection_FirstChild, NavigateDirection_LastChild, NavigateDirection_NextSibling,
    NavigateDirection_Parent, NavigateDirection_PreviousSibling, ProviderOptions,
    ProviderOptions_ServerSideProvider, ToggleState, ToggleState_Off, ToggleState_On,
    UIA_AcceleratorKeyPropertyId, UIA_AutomationFocusChangedEventId, UIA_AutomationIdPropertyId,
    UIA_ControlTypePropertyId, UIA_HasKeyboardFocusPropertyId, UIA_InvokePatternId,
    UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_MenuClosedEventId,
    UIA_MenuControlTypeId, UIA_MenuItemControlTypeId, UIA_MenuOpenedEventId, UIA_NamePropertyId,
    UIA_PATTERN_ID, UIA_PROPERTY_ID, UIA_PositionInSetPropertyId, UIA_SeparatorControlTypeId,
    UIA_SizeOfSetPropertyId, UIA_TextControlTypeId, UIA_TogglePatternId, UiaAppendRuntimeId,
    UiaClientsAreListening, UiaHostProviderFromHwnd, UiaRaiseAutomationEvent, UiaRect,
    UiaReturnRawElementProvider, UiaRootObjectId,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{BSTR, Error, IUnknown, IUnknownImpl, Interface, Result, implement};

use super::{WM_MENU_FOCUS, WM_MENU_INVOKE};
use crate::domain::menu::Line;

/// One line as assistive technology sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub line: Line,
    /// Its box on the screen, in physical pixels.
    pub rect: (f64, f64, f64, f64),
}

/// The menu shown now (one at a time), and the line with the focus.
static LINES: Mutex<(Vec<Entry>, Option<usize>)> = Mutex::new((Vec::new(), None));

fn lines() -> (Vec<Entry>, Option<usize>) {
    LINES.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// The menu's name for assistive technology.
const LABEL: &str = "Dockyard";

/// The menu opened with these lines: the screen reader hears it open.
pub fn opened(hwnd: HWND, found: Vec<Entry>) {
    *LINES.lock().unwrap_or_else(PoisonError::into_inner) = (found, None);
    raise(
        &Root {
            hwnd: hwnd.0 as isize,
        }
        .into(),
        UIA_MenuOpenedEventId,
    );
}

/// The menu closed.
pub fn closed(hwnd: HWND) {
    raise(
        &Root {
            hwnd: hwnd.0 as isize,
        }
        .into(),
        UIA_MenuClosedEventId,
    );
    *LINES.lock().unwrap_or_else(PoisonError::into_inner) = (Vec::new(), None);
}

/// The keyboard (or the pointer) is on `index` now.
pub fn focused(hwnd: HWND, index: Option<usize>) {
    LINES.lock().unwrap_or_else(PoisonError::into_inner).1 = index;
    if let Some(index) = index {
        let item: IRawElementProviderSimple = Item {
            hwnd: hwnd.0 as isize,
            index,
        }
        .into();
        raise(&item, UIA_AutomationFocusChangedEventId);
    }
}

fn raise(
    element: &IRawElementProviderSimple,
    event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
) {
    if unsafe { UiaClientsAreListening() }.as_bool() {
        let _ = unsafe { UiaRaiseAutomationEvent(element, event) };
    }
}

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

fn fragment(hwnd: isize, index: usize) -> IRawElementProviderFragment {
    Item { hwnd, index }.into()
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
            UIA_ControlTypePropertyId => VARIANT::from(UIA_MenuControlTypeId.0),
            UIA_NamePropertyId => VARIANT::from(BSTR::from(LABEL)),
            UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from("dockmax-menu")),
            UIA_IsKeyboardFocusablePropertyId => VARIANT::from(false),
            _ => VARIANT::default(),
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as _)) }
    }
}

impl IRawElementProviderFragment_Impl for Root_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        let count = lines().0.len();
        match direction {
            NavigateDirection_FirstChild if count > 0 => Ok(fragment(self.hwnd, 0)),
            NavigateDirection_LastChild if count > 0 => Ok(fragment(self.hwnd, count - 1)),
            _ => none(),
        }
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        // The window's own: UI Automation takes it from the host.
        Ok(std::ptr::null_mut())
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
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
        let found = lines().0.iter().position(|entry| {
            let (left, top, width, height) = entry.rect;
            x >= left && y >= top && x < left + width && y < top + height
        });
        match found {
            Some(index) => Ok(fragment(self.hwnd, index)),
            None => none(),
        }
    }

    fn GetFocus(&self) -> Result<IRawElementProviderFragment> {
        match lines().1 {
            Some(index) => Ok(fragment(self.hwnd, index)),
            None => none(),
        }
    }
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider,
    IToggleProvider
)]
struct Item {
    hwnd: isize,
    index: usize,
}

impl Item_Impl {
    fn entry(&self) -> Option<Entry> {
        lines().0.get(self.index).cloned()
    }

    fn post(&self, message: u32) {
        let _ = unsafe {
            PostMessageW(
                Some(HWND(self.hwnd as _)),
                message,
                WPARAM(self.index),
                LPARAM(0),
            )
        };
    }
}

impl IRawElementProviderSimple_Impl for Item_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        let Some(Entry {
            line: Line::Item(item),
            ..
        }) = self.entry()
        else {
            return none();
        };
        match pattern {
            UIA_InvokePatternId => self.to_interface::<IInvokeProvider>().cast(),
            // Only a line that turns something on and off is a toggle.
            UIA_TogglePatternId if item.toggle => self.to_interface::<IToggleProvider>().cast(),
            _ => none(),
        }
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        let (all, focus) = lines();
        let Some(entry) = all.get(self.index) else {
            return Ok(VARIANT::default());
        };
        let items: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.line, Line::Item(_)))
            .map(|(i, _)| i)
            .collect();
        Ok(match (&entry.line, property) {
            (Line::Item(_), UIA_ControlTypePropertyId) => {
                VARIANT::from(UIA_MenuItemControlTypeId.0)
            }
            (Line::Separator, UIA_ControlTypePropertyId) => {
                VARIANT::from(UIA_SeparatorControlTypeId.0)
            }
            (Line::Title(_), UIA_ControlTypePropertyId) => VARIANT::from(UIA_TextControlTypeId.0),
            (Line::Item(item), UIA_NamePropertyId) => {
                VARIANT::from(BSTR::from(item.label.as_str()))
            }
            (Line::Title(title), UIA_NamePropertyId) => VARIANT::from(BSTR::from(title.as_str())),
            (Line::Item(item), UIA_AcceleratorKeyPropertyId) => match &item.shortcut {
                Some(keys) => VARIANT::from(BSTR::from(keys.as_str())),
                None => VARIANT::default(),
            },
            (Line::Item(item), UIA_IsEnabledPropertyId) => VARIANT::from(item.enabled),
            (Line::Item(_), UIA_IsKeyboardFocusablePropertyId) => VARIANT::from(true),
            (Line::Item(_), UIA_HasKeyboardFocusPropertyId) => {
                VARIANT::from(focus == Some(self.index))
            }
            (Line::Item(_), UIA_PositionInSetPropertyId) => {
                let at = items.iter().position(|&i| i == self.index).unwrap_or(0);
                VARIANT::from(at as i32 + 1)
            }
            (Line::Item(_), UIA_SizeOfSetPropertyId) => VARIANT::from(items.len() as i32),
            (_, UIA_AutomationIdPropertyId) => {
                VARIANT::from(BSTR::from(format!("line-{}", self.index).as_str()))
            }
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
        let count = lines().0.len();
        let found = match direction {
            NavigateDirection_NextSibling => Some(self.index + 1).filter(|&i| i < count),
            NavigateDirection_PreviousSibling => self.index.checked_sub(1),
            _ => None,
        };
        match found {
            Some(index) => Ok(fragment(self.hwnd, index)),
            None => none(),
        }
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        unsafe {
            let ids = SafeArrayCreateVector(VT_I4, 0, 2);
            if ids.is_null() {
                return none();
            }
            for (at, value) in [UiaAppendRuntimeId as i32, self.index as i32 + 1]
                .into_iter()
                .enumerate()
            {
                let at = at as i32;
                SafeArrayPutElement(ids, &at, std::ptr::from_ref(&value).cast())?;
            }
            Ok(ids)
        }
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        Ok(match self.entry() {
            Some(entry) => {
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
        self.post(WM_MENU_FOCUS);
        Ok(())
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        Ok(Root { hwnd: self.hwnd }.into())
    }
}

impl IInvokeProvider_Impl for Item_Impl {
    fn Invoke(&self) -> Result<()> {
        self.post(WM_MENU_INVOKE);
        Ok(())
    }
}

impl IToggleProvider_Impl for Item_Impl {
    fn Toggle(&self) -> Result<()> {
        // Choosing a checked line is what changes it, as in Windows' menus.
        self.post(WM_MENU_INVOKE);
        Ok(())
    }

    fn ToggleState(&self) -> Result<ToggleState> {
        Ok(match self.entry().map(|entry| entry.line) {
            Some(Line::Item(item)) if item.checked => ToggleState_On,
            _ => ToggleState_Off,
        })
    }
}
