//! The launcher's results for assistive technology (spec 0012 §7): a UI
//! Automation provider for the list's window, a list "Resultados" with an
//! item per result (its name, its kind, its place, whether it is selected)
//! that can be selected and invoked. The search box is a Windows text field
//! and speaks for itself.
//!
//! What the provider says comes from `ENTRIES`, which the launcher keeps up
//! to date; UI Automation may call from its own threads. A press goes back to
//! the launcher through `post`.

// The patterns below match Windows' constants, which keep their own names.
#![allow(non_upper_case_globals)]

use std::sync::{Mutex, PoisonError};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{SafeArrayCreateVector, SafeArrayPutElement};
use windows::Win32::System::Variant::{VARIANT, VT_I4, VT_UNKNOWN};
use windows::Win32::UI::Accessibility::{
    IInvokeProvider, IInvokeProvider_Impl, IRawElementProviderFragment,
    IRawElementProviderFragment_Impl, IRawElementProviderFragmentRoot,
    IRawElementProviderFragmentRoot_Impl, IRawElementProviderSimple,
    IRawElementProviderSimple_Impl, ISelectionItemProvider, ISelectionItemProvider_Impl,
    ISelectionProvider, ISelectionProvider_Impl, IValueProvider, IValueProvider_Impl,
    NavigateDirection, NavigateDirection_FirstChild, NavigateDirection_LastChild,
    NavigateDirection_NextSibling, NavigateDirection_Parent, NavigateDirection_PreviousSibling,
    NotificationKind_ActionCompleted, NotificationProcessing_MostRecent, ProviderOptions,
    ProviderOptions_ServerSideProvider, UIA_AutomationIdPropertyId, UIA_ControlTypePropertyId,
    UIA_EditControlTypeId, UIA_HelpTextPropertyId, UIA_InvokePatternId, UIA_IsEnabledPropertyId,
    UIA_IsKeyboardFocusablePropertyId, UIA_IsPasswordPropertyId, UIA_ItemTypePropertyId,
    UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_NamePropertyId, UIA_PATTERN_ID,
    UIA_PROPERTY_ID, UIA_PositionInSetPropertyId, UIA_SelectionItem_ElementSelectedEventId,
    UIA_SelectionItemPatternId, UIA_SelectionPatternId, UIA_SizeOfSetPropertyId,
    UIA_ValuePatternId, UiaAppendRuntimeId, UiaClientsAreListening, UiaHostProviderFromHwnd,
    UiaRaiseAutomationEvent, UiaRaiseNotificationEvent, UiaRect, UiaReturnRawElementProvider,
    UiaRootObjectId,
};
use windows::core::{BSTR, Error, IUnknown, IUnknownImpl, Interface, Result, implement};

/// One result as assistive technology sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The result's key: stable while the list changes.
    pub key: String,
    pub name: String,
    /// What it is ("App").
    pub kind: String,
    /// What is said after it, when there is more than its name: a
    /// conversion's sum, in words (spec 0012 §5b).
    pub help: Option<String>,
    /// Its box on the screen, in physical pixels.
    pub rect: (f64, f64, f64, f64),
    pub selected: bool,
}

/// The results shown now (there is one launcher).
static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

fn entries() -> Vec<Entry> {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// What the list shows now.
pub fn publish(found: Vec<Entry>) {
    *ENTRIES.lock().unwrap_or_else(PoisonError::into_inner) = found;
}

/// The list's name for assistive technology.
const LABEL: &str = "Resultados";

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

/// The selection moved to `key`: the screen reader reads it (the event, and
/// a notification with its name, kind and place, since the keyboard stays in
/// the search box).
pub fn selected(hwnd: isize, key: &str, said: &str) {
    if !unsafe { UiaClientsAreListening() }.as_bool() {
        return;
    }
    let item: IRawElementProviderSimple = Item {
        hwnd,
        key: key.to_owned(),
    }
    .into();
    unsafe {
        let _ = UiaRaiseAutomationEvent(&item, UIA_SelectionItem_ElementSelectedEventId);
    }
    announce(hwnd, said);
}

/// Answers `WM_GETOBJECT` for the search box: a text field named "Buscar
/// apps", whose value is what is typed. Windows' own text field gave no name
/// and no kind to UI Automation (it showed as an unnamed pane).
pub fn get_field_object(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if lparam.0 as i32 != UiaRootObjectId {
        return None;
    }
    let field: IRawElementProviderSimple = Field {
        hwnd: hwnd.0 as isize,
    }
    .into();
    Some(unsafe { UiaReturnRawElementProvider(hwnd, wparam, lparam, &field) })
}

#[implement(IRawElementProviderSimple, IValueProvider)]
struct Field {
    hwnd: isize,
}

impl IRawElementProviderSimple_Impl for Field_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        if pattern == UIA_ValuePatternId {
            self.to_interface::<IValueProvider>().cast()
        } else {
            none()
        }
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(match property {
            UIA_ControlTypePropertyId => VARIANT::from(UIA_EditControlTypeId.0),
            UIA_NamePropertyId => {
                VARIANT::from(BSTR::from(crate::i18n::messages().launcher.placeholder))
            }
            UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from("search")),
            UIA_IsKeyboardFocusablePropertyId => VARIANT::from(true),
            UIA_IsPasswordPropertyId => VARIANT::from(false),
            _ => VARIANT::default(),
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as _)) }
    }
}

impl IValueProvider_Impl for Field_Impl {
    fn SetValue(&self, value: &windows::core::PCWSTR) -> Result<()> {
        let text = unsafe { value.to_string() }.unwrap_or_default();
        super::post(move |launcher| launcher.set_query(&text));
        Ok(())
    }

    fn Value(&self) -> Result<BSTR> {
        Ok(BSTR::from(super::read_text(HWND(self.hwnd as _)).as_str()))
    }

    fn IsReadOnly(&self) -> Result<windows::core::BOOL> {
        Ok(false.into())
    }
}

/// Says `text` (how many results there are, the selected one), the newest
/// replacing what was not said yet.
pub fn announce(hwnd: isize, text: &str) {
    if !unsafe { UiaClientsAreListening() }.as_bool() {
        return;
    }
    let root: IRawElementProviderSimple = Root { hwnd }.into();
    unsafe {
        let _ = UiaRaiseNotificationEvent(
            &root,
            NotificationKind_ActionCompleted,
            NotificationProcessing_MostRecent,
            &BSTR::from(text),
            &BSTR::from("dockmax-launcher"),
        );
    }
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
    IRawElementProviderFragmentRoot,
    ISelectionProvider
)]
struct Root {
    hwnd: isize,
}

impl IRawElementProviderSimple_Impl for Root_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        if pattern == UIA_SelectionPatternId {
            self.to_interface::<ISelectionProvider>().cast()
        } else {
            none()
        }
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(match property {
            UIA_ControlTypePropertyId => VARIANT::from(UIA_ListControlTypeId.0),
            UIA_NamePropertyId => VARIANT::from(BSTR::from(LABEL)),
            UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from("results")),
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
        let entries = entries();
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
        let found = entries().into_iter().find(|entry| {
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

impl ISelectionProvider_Impl for Root_Impl {
    fn GetSelection(&self) -> Result<*mut SAFEARRAY> {
        let chosen = entries().into_iter().find(|entry| entry.selected);
        unsafe {
            let count = u32::from(chosen.is_some());
            let array = SafeArrayCreateVector(VT_UNKNOWN, 0, count);
            if array.is_null() {
                return none();
            }
            if let Some(entry) = chosen {
                let element: IRawElementProviderSimple = Item {
                    hwnd: self.hwnd,
                    key: entry.key,
                }
                .into();
                let at = 0i32;
                SafeArrayPutElement(array, &at, element.as_raw())?;
            }
            Ok(array)
        }
    }

    fn CanSelectMultiple(&self) -> Result<windows::core::BOOL> {
        Ok(false.into())
    }

    fn IsSelectionRequired(&self) -> Result<windows::core::BOOL> {
        Ok(false.into())
    }
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider,
    ISelectionItemProvider
)]
struct Item {
    hwnd: isize,
    key: String,
}

impl Item_Impl {
    fn entry(&self) -> Option<(usize, usize, Entry)> {
        let entries = entries();
        let count = entries.len();
        entries
            .into_iter()
            .enumerate()
            .find(|(_, entry)| entry.key == self.key)
            .map(|(index, entry)| (index, count, entry))
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
        match pattern {
            UIA_InvokePatternId => self.to_interface::<IInvokeProvider>().cast(),
            UIA_SelectionItemPatternId => self.to_interface::<ISelectionItemProvider>().cast(),
            _ => none(),
        }
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        let Some((index, count, entry)) = self.entry() else {
            return Ok(VARIANT::default());
        };
        Ok(match property {
            UIA_ControlTypePropertyId => VARIANT::from(UIA_ListItemControlTypeId.0),
            UIA_NamePropertyId => VARIANT::from(BSTR::from(entry.name.as_str())),
            UIA_ItemTypePropertyId => VARIANT::from(BSTR::from(entry.kind.as_str())),
            UIA_HelpTextPropertyId => match &entry.help {
                Some(help) => VARIANT::from(BSTR::from(help.as_str())),
                None => VARIANT::default(),
            },
            UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from(entry.key.as_str())),
            UIA_PositionInSetPropertyId => VARIANT::from(index as i32 + 1),
            UIA_SizeOfSetPropertyId => VARIANT::from(count as i32),
            UIA_IsEnabledPropertyId => VARIANT::from(true),
            // The keyboard stays in the search box.
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
        let entries = entries();
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
            Some((_, _, entry)) => {
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
        let key = self.key.clone();
        super::post(move |launcher| launcher.open_key(&key));
        Ok(())
    }
}

impl ISelectionItemProvider_Impl for Item_Impl {
    fn Select(&self) -> Result<()> {
        let key = self.key.clone();
        super::post(move |launcher| launcher.select_key(&key));
        Ok(())
    }

    fn AddToSelection(&self) -> Result<()> {
        self.Select()
    }

    fn RemoveFromSelection(&self) -> Result<()> {
        Ok(())
    }

    fn IsSelected(&self) -> Result<windows::core::BOOL> {
        Ok(self
            .entry()
            .is_some_and(|(_, _, entry)| entry.selected)
            .into())
    }

    fn SelectionContainer(&self) -> Result<IRawElementProviderSimple> {
        Ok(Root { hwnd: self.hwnd }.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_results_number_is_stable_and_tells_keys_apart() {
        assert_eq!(number_of("app:Paint"), number_of("app:Paint"));
        assert_ne!(number_of("app:Paint"), number_of("app:Calculadora"));
    }
}
