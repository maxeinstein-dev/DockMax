//! Putting a text in the clipboard (the launcher's copies: the sum, the
//! conversion, "Copiar caminho" and Ctrl+C), as a rule over Windows' calls,
//! so their order is tested without Windows.
//!
//! The clipboard is one for the whole session, and a program holds it while
//! it reads or writes: a moment after something is copied, the clipboard
//! history and other listeners open it to read it. `OpenClipboard` then
//! fails (`ERROR_ACCESS_DENIED`), and Windows' advice is to try again a few
//! times, a moment apart. It is emptied only once it is open, right before
//! the text is set; the text's memory is made before it is opened, so
//! nothing that can fail runs between emptying it and setting the text but
//! `SetClipboardData` itself.

/// How many times opening the clipboard is tried, and the pause between
/// two tries, in milliseconds: about 100 ms in all, on the launcher's
/// thread.
pub const OPEN_TRIES: u32 = 5;
pub const OPEN_PAUSE_MS: u64 = 20;

/// Windows' calls, as `write` makes them.
pub trait Clipboard {
    /// `OpenClipboard`: whether it opened.
    fn open(&mut self) -> bool;
    /// `EmptyClipboard`: whether it emptied (taking it as its owner).
    fn empty(&mut self) -> bool;
    /// `SetClipboardData` with the text made before: whether it took it.
    fn set(&mut self) -> bool;
    /// `CloseClipboard`.
    fn close(&mut self);
    /// Waits `ms` milliseconds before trying to open it again.
    fn pause(&mut self, ms: u64);
}

/// How a copy went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    /// The text is in the clipboard.
    Done,
    /// The text's memory could not be had: the clipboard was not touched.
    NoMemory,
    /// Another program held the clipboard through every try: it was not
    /// touched (what was copied before is still there).
    Busy,
    /// It opened but did not empty: what was copied before is still there.
    NotEmptied,
    /// It emptied but did not take the text.
    NotSet,
}

impl Written {
    /// Whether the launcher closes, as after opening something; otherwise
    /// it stays, with the row saying the copy failed.
    pub fn closes_launcher(self) -> bool {
        self == Written::Done
    }
}

/// Puts the text in `clipboard`: opens it (trying `OPEN_TRIES` times,
/// `OPEN_PAUSE_MS` apart), empties it, sets the text and closes it. It is
/// closed whenever it was opened, and never emptied without the text ready
/// to be set.
pub fn write(clipboard: &mut impl Clipboard) -> Written {
    let mut opened = false;
    for attempt in 0..OPEN_TRIES {
        if attempt > 0 {
            clipboard.pause(OPEN_PAUSE_MS);
        }
        if clipboard.open() {
            opened = true;
            break;
        }
    }
    if !opened {
        return Written::Busy;
    }
    let written = if !clipboard.empty() {
        Written::NotEmptied
    } else if !clipboard.set() {
        Written::NotSet
    } else {
        Written::Done
    };
    clipboard.close();
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clipboard that opens after `busy` failed tries and records the
    /// calls.
    struct Fake {
        busy: u32,
        empties: bool,
        sets: bool,
        calls: Vec<String>,
    }

    impl Fake {
        fn new(busy: u32) -> Fake {
            Fake {
                busy,
                empties: true,
                sets: true,
                calls: Vec::new(),
            }
        }
    }

    impl Clipboard for Fake {
        fn open(&mut self) -> bool {
            if self.busy > 0 {
                self.busy -= 1;
                self.calls.push("open: busy".into());
                false
            } else {
                self.calls.push("open".into());
                true
            }
        }
        fn empty(&mut self) -> bool {
            self.calls.push("empty".into());
            self.empties
        }
        fn set(&mut self) -> bool {
            self.calls.push("set".into());
            self.sets
        }
        fn close(&mut self) {
            self.calls.push("close".into());
        }
        fn pause(&mut self, ms: u64) {
            self.calls.push(format!("pause {ms}"));
        }
    }

    #[test]
    fn a_free_clipboard_takes_the_text_at_once() {
        let mut clipboard = Fake::new(0);
        assert_eq!(write(&mut clipboard), Written::Done);
        assert_eq!(clipboard.calls, ["open", "empty", "set", "close"]);
    }

    #[test]
    fn a_held_clipboard_is_tried_again_a_moment_apart() {
        // The clipboard history reading what the script had just copied.
        let mut clipboard = Fake::new(2);
        assert_eq!(write(&mut clipboard), Written::Done);
        assert_eq!(
            clipboard.calls,
            [
                "open: busy",
                "pause 20",
                "open: busy",
                "pause 20",
                "open",
                "empty",
                "set",
                "close"
            ]
        );
    }

    #[test]
    fn a_clipboard_held_through_every_try_is_left_as_it_was() {
        let mut clipboard = Fake::new(OPEN_TRIES);
        assert_eq!(write(&mut clipboard), Written::Busy);
        let opens = clipboard
            .calls
            .iter()
            .filter(|call| call.starts_with("open"))
            .count();
        assert_eq!(opens, OPEN_TRIES as usize);
        // Never emptied, nor closed (it was never open).
        assert!(!clipboard.calls.iter().any(|call| call == "empty"));
        assert!(!clipboard.calls.iter().any(|call| call == "close"));
        // About 100 ms in all.
        assert_eq!(u64::from(OPEN_TRIES - 1) * OPEN_PAUSE_MS, 80);
    }

    #[test]
    fn a_failed_empty_or_set_still_closes() {
        let mut clipboard = Fake::new(0);
        clipboard.empties = false;
        assert_eq!(write(&mut clipboard), Written::NotEmptied);
        assert_eq!(clipboard.calls, ["open", "empty", "close"]);

        let mut clipboard = Fake::new(0);
        clipboard.sets = false;
        assert_eq!(write(&mut clipboard), Written::NotSet);
        assert_eq!(clipboard.calls, ["open", "empty", "set", "close"]);
    }

    #[test]
    fn only_a_done_copy_closes_the_launcher() {
        assert!(Written::Done.closes_launcher());
        for failed in [
            Written::NoMemory,
            Written::Busy,
            Written::NotEmptied,
            Written::NotSet,
        ] {
            assert!(!failed.closes_launcher(), "{failed:?}");
        }
    }
}
