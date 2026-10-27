//! The icons of the launcher's results, read by Windows in a helper process
//! (author's decision, 2026-10-27): reading a program's icon through the
//! shell keeps data of each Store app in the process for good, about 20 MB
//! after a few searches. The helper is this executable started with `FLAG`;
//! it lives while the launcher is open and takes that memory with it when it
//! goes.
//!
//! The protocol, on the helper's standard input and output: one request per
//! line, `<size>\t<id>`; one answer per request, in order, the length of the
//! PNG as four bytes (little endian, 0 when there is no icon) and the PNG.

use std::io::{BufRead, Read, Write};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

use crate::domain::apps::is_valid_app_id;

/// The first argument that makes this executable the icon reader.
pub const FLAG: &str = "--launcher-icons";

/// The biggest icon accepted from the helper.
const MAX_PNG: usize = 4 * 1024 * 1024;

/// `CREATE_NO_WINDOW`: the helper never shows anything.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The helper's side: answers the requests on standard input until it
/// closes. Started by `main` before anything of the app.
pub fn serve() {
    super::super::apps::init_com();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return;
        };
        let png = parse(&line).and_then(|(size, id)| super::super::apps::icon_png(id, size));
        let png = png.filter(|png| png.len() <= MAX_PNG).unwrap_or_default();
        let length = u32::try_from(png.len()).unwrap_or(0).to_le_bytes();
        if out.write_all(&length).is_err() || out.write_all(&png).is_err() || out.flush().is_err() {
            return;
        }
    }
}

/// A request line: the size and a program of the Apps folder.
fn parse(line: &str) -> Option<(u32, &str)> {
    let (size, id) = line.split_once('\t')?;
    let size: u32 = size.parse().ok()?;
    ((16..=256).contains(&size) && is_valid_app_id(id)).then_some((size, id))
}

/// The launcher's side: a thread that starts the helper on the first
/// request and hands it the requests, one at a time. Dropping it closes the
/// helper.
pub struct Reader {
    requests: mpsc::Sender<(String, u32)>,
}

impl Reader {
    /// `answered` gets each program's icon (`None` when there is none or it
    /// could not be read), on the reader's thread.
    pub fn start(answered: impl Fn(String, Option<Vec<u8>>) + Send + 'static) -> Option<Reader> {
        let (requests, received) = mpsc::channel::<(String, u32)>();
        std::thread::Builder::new()
            .name("launcher-icons".into())
            .spawn(move || run(&received, &answered))
            .inspect_err(|error| log::warn!("launcher: the icon reader could not start: {error}"))
            .ok()?;
        Some(Reader { requests })
    }

    /// Asks for the icon of `id`, `size` pixels a side.
    pub fn ask(&self, id: String, size: u32) {
        let _ = self.requests.send((id, size));
    }
}

fn spawn() -> Option<Child> {
    let exe = std::env::current_exe().ok()?;
    Command::new(exe)
        .arg(FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .inspect_err(|error| log::warn!("launcher: the icon reader could not start: {error}"))
        .ok()
}

/// One answer of the helper; `Err` when it is gone.
fn answer(out: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut length = [0u8; 4];
    out.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_PNG {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut png = vec![0u8; length];
    out.read_exact(&mut png)?;
    Ok((length > 0).then_some(png))
}

fn run(received: &mpsc::Receiver<(String, u32)>, answered: &dyn Fn(String, Option<Vec<u8>>)) {
    let Ok(first) = received.recv() else {
        return;
    };
    let Some(mut child) = spawn() else {
        answered(first.0, None);
        for (id, _) in received.iter() {
            answered(id, None);
        }
        return;
    };
    let (Some(mut input), Some(mut output)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        return;
    };
    let mut gone = false;
    for (id, size) in std::iter::once(first).chain(received.iter()) {
        if gone {
            answered(id, None);
            continue;
        }
        let sent = writeln!(input, "{size}\t{id}").and_then(|()| input.flush());
        match sent.and_then(|()| answer(&mut output)) {
            Ok(png) => answered(id, png),
            Err(error) => {
                log::warn!("launcher: the icon reader stopped: {error}");
                gone = true;
                answered(id, None);
            }
        }
    }
    // The launcher closed: the helper sees its input end and leaves.
    drop(input);
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_a_size_and_an_app() {
        assert_eq!(
            parse("64\tMicrosoft.Paint!App"),
            Some((64, "Microsoft.Paint!App"))
        );
        assert_eq!(parse(r"40	C:\Tools\a.exe"), Some((40, r"C:\Tools\a.exe")));
        for bad in [
            "",
            "64",
            "x\tPaint",
            "8\tPaint",
            "999\tPaint",
            "64\t",
            "64\t Paint",
        ] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn an_answer_is_its_length_and_the_png() {
        let mut stream: &[u8] = &[3, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0];
        assert_eq!(answer(&mut stream).unwrap(), Some(vec![1, 2, 3]));
        assert_eq!(answer(&mut stream).unwrap(), None);
        assert!(answer(&mut stream).is_err(), "the helper is gone");
        let mut too_big: &[u8] = &u32::MAX.to_le_bytes();
        assert!(answer(&mut too_big).is_err());
    }
}
