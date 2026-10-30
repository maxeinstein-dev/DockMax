//! The launcher's helper process (author's decision, 2026-10-27): reading a
//! program's icon through the shell keeps data of each Store app in the
//! process for good, about 20 MB after a few searches. The helper is this
//! executable started with `FLAG`; it lives while the launcher is open and
//! takes that memory with it when it goes. Since L4 it also reads the icons
//! of files and folders and searches them (`files`): the shell, COM and the
//! Windows Search's OLE DB stay out of the app's process.
//!
//! The protocol, on the helper's standard input and output: one request per
//! line, and one answer per request, in order, as the length of what follows
//! in four bytes (little endian, 0 when there is nothing) and the bytes.
//!
//! | Request | Answer |
//! |---|---|
//! | `<size>\t<app id>` | the program's icon, a PNG |
//! | `file\t<size>\t<path>` | the icon of a file or folder, a PNG |
//! | `files\t<limit>\t<words>` | the files and folders found (`launcher_files::encode_hits`) |

use std::io::{BufRead, Read, Write};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

use crate::domain::apps::is_valid_app_id;
use crate::domain::launcher_files::{self, FileHit, ONLY_LIMIT};

/// The first argument that makes this executable the launcher's helper.
pub const FLAG: &str = "--launcher-icons";

/// The biggest answer accepted from the helper.
const MAX_ANSWER: usize = 4 * 1024 * 1024;

/// `CREATE_NO_WINDOW`: the helper never shows anything.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// What the launcher asks of the helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// A program's icon, `size` pixels a side.
    App { id: String, size: u32 },
    /// The icon of a file or folder, kept under `key` by the launcher.
    File {
        key: String,
        path: String,
        size: u32,
    },
    /// The files and folders whose name has `words`; `generation` tells
    /// the launcher's typing it answers.
    Files {
        generation: u64,
        words: Vec<String>,
        limit: usize,
    },
}

/// What the helper answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The icon of the request's `id` (a program's) or `key` (a file's);
    /// `None` when there is none or it could not be read.
    Icon {
        key: String,
        png: Option<Vec<u8>>,
    },
    Files {
        generation: u64,
        hits: Vec<FileHit>,
    },
}

impl Request {
    /// The request's line, without its end.
    fn line(&self) -> String {
        match self {
            Request::App { id, size } => format!("{size}\t{id}"),
            Request::File { path, size, .. } => format!("file\t{size}\t{path}"),
            Request::Files { words, limit, .. } => format!("files\t{limit}\t{}", words.join(" ")),
        }
    }

    /// What the launcher gets when there is no answer.
    fn unanswered(self) -> Answer {
        match self {
            Request::App { id, .. } => Answer::Icon { key: id, png: None },
            Request::File { key, .. } => Answer::Icon { key, png: None },
            Request::Files { generation, .. } => Answer::Files {
                generation,
                hits: Vec::new(),
            },
        }
    }

    /// The answer `bytes` of the helper to this request.
    fn answered(self, bytes: Option<Vec<u8>>) -> Answer {
        match self {
            Request::App { id, .. } => Answer::Icon {
                key: id,
                png: bytes,
            },
            Request::File { key, .. } => Answer::Icon { key, png: bytes },
            Request::Files { generation, .. } => Answer::Files {
                generation,
                hits: bytes
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map(|text| launcher_files::decode_hits(&text))
                    .unwrap_or_default(),
            },
        }
    }
}

/// A request line as the helper reads it.
#[derive(Debug, PartialEq, Eq)]
enum Line<'a> {
    App(u32, &'a str),
    File(u32, &'a str),
    Files(usize, Vec<String>),
}

fn size_of_icon(size: &str) -> Option<u32> {
    let size: u32 = size.parse().ok()?;
    (16..=256).contains(&size).then_some(size)
}

/// A request line; `None` when it is not one.
fn parse(line: &str) -> Option<Line<'_>> {
    let (head, rest) = line.split_once('\t')?;
    match head {
        "file" => {
            let (size, path) = rest.split_once('\t')?;
            let size = size_of_icon(size)?;
            launcher_files::is_valid_path(path).then_some(Line::File(size, path))
        }
        "files" => {
            let (limit, words) = rest.split_once('\t')?;
            let limit: usize = limit.parse().ok()?;
            let words = launcher_files::words(words);
            ((1..=ONLY_LIMIT).contains(&limit) && !words.is_empty())
                .then_some(Line::Files(limit, words))
        }
        size => {
            let size = size_of_icon(size)?;
            is_valid_app_id(rest).then_some(Line::App(size, rest))
        }
    }
}

/// The helper's side: answers the requests on standard input until it
/// closes. Started by `main` before anything of the app.
pub fn serve() {
    super::super::apps::init_com();
    let mut searcher: Option<super::files::Searcher> = None;
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return;
        };
        let bytes = match parse(&line) {
            Some(Line::App(size, id)) => super::super::apps::icon_png(id, size),
            Some(Line::File(size, path)) => super::super::apps::path_icon_png(path, size),
            Some(Line::Files(limit, words)) => {
                let hits = searcher
                    .get_or_insert_with(super::files::Searcher::new)
                    .search(&words, limit);
                Some(launcher_files::encode_hits(&hits).into_bytes())
            }
            None => None,
        };
        let bytes = bytes
            .filter(|bytes| bytes.len() <= MAX_ANSWER)
            .unwrap_or_default();
        let length = u32::try_from(bytes.len()).unwrap_or(0).to_le_bytes();
        if out.write_all(&length).is_err() || out.write_all(&bytes).is_err() || out.flush().is_err()
        {
            return;
        }
    }
}

/// The launcher's side: a thread that starts the helper on the first
/// request and hands it the requests, one at a time. Dropping it closes the
/// helper.
pub struct Reader {
    requests: mpsc::Sender<Request>,
}

impl Reader {
    /// `answered` gets each answer, on the reader's thread.
    pub fn start(answered: impl Fn(Answer) + Send + 'static) -> Option<Reader> {
        let (requests, received) = mpsc::channel::<Request>();
        std::thread::Builder::new()
            .name("launcher-helper".into())
            .spawn(move || run(&received, &answered))
            .inspect_err(|error| log::warn!("launcher: the helper could not start: {error}"))
            .ok()?;
        Some(Reader { requests })
    }

    /// A reader whose thread could not start: what it is asked goes
    /// nowhere (the rows keep their glyph, no file is found).
    pub fn closed() -> Reader {
        let (requests, _) = mpsc::channel();
        Reader { requests }
    }

    pub fn ask(&self, request: Request) {
        let _ = self.requests.send(request);
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
        .inspect_err(|error| log::warn!("launcher: the helper could not start: {error}"))
        .ok()
}

/// One answer of the helper; `Err` when it is gone.
fn answer(out: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut length = [0u8; 4];
    out.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_ANSWER {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut bytes = vec![0u8; length];
    out.read_exact(&mut bytes)?;
    Ok((length > 0).then_some(bytes))
}

/// The requests waiting, in the order they are sent: the icons first, as
/// they came, then only the last search of files (the ones before it
/// answer an older typing, and get an empty answer at once).
fn order(waiting: Vec<Request>) -> (Vec<Request>, Vec<Request>) {
    let mut icons = Vec::new();
    let mut searches = Vec::new();
    for request in waiting {
        match request {
            Request::Files { .. } => searches.push(request),
            _ => icons.push(request),
        }
    }
    let last = searches.pop();
    icons.extend(last);
    (icons, searches)
}

fn run(received: &mpsc::Receiver<Request>, answered: &dyn Fn(Answer)) {
    let Ok(first) = received.recv() else {
        return;
    };
    let Some(mut child) = spawn() else {
        answered(first.unanswered());
        for request in received.iter() {
            answered(request.unanswered());
        }
        return;
    };
    let (Some(mut input), Some(mut output)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        return;
    };
    let mut gone = false;
    let mut next = Some(first);
    while let Some(request) = next.take().or_else(|| received.recv().ok()) {
        let mut waiting = vec![request];
        waiting.extend(received.try_iter());
        let (send, dropped) = order(waiting);
        for request in dropped {
            answered(request.unanswered());
        }
        for request in send {
            if gone {
                answered(request.unanswered());
                continue;
            }
            let sent = writeln!(input, "{}", request.line()).and_then(|()| input.flush());
            match sent.and_then(|()| answer(&mut output)) {
                Ok(bytes) => answered(request.answered(bytes)),
                Err(error) => {
                    log::warn!("launcher: the helper stopped: {error}");
                    gone = true;
                    answered(request.unanswered());
                }
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
    fn a_request_is_an_icon_or_a_search() {
        assert_eq!(
            parse("64\tMicrosoft.Paint!App"),
            Some(Line::App(64, "Microsoft.Paint!App"))
        );
        assert_eq!(
            parse(r"40	C:\Tools\a.exe"),
            Some(Line::App(40, r"C:\Tools\a.exe"))
        );
        assert_eq!(
            parse("file\t32\tC:\\Docs\\a.pdf"),
            Some(Line::File(32, r"C:\Docs\a.pdf"))
        );
        assert_eq!(
            parse("files\t8\trelatorio mensal"),
            Some(Line::Files(8, vec!["relatorio".into(), "mensal".into()]))
        );
        for bad in [
            "",
            "64",
            "x\tPaint",
            "8\tPaint",
            "999\tPaint",
            "64\t",
            "64\t Paint",
            "file\t32\ta.pdf",
            "file\t32\t\\\\server\\a.pdf",
            "file\t8\tC:\\a.pdf",
            "files\t0\tx",
            "files\t999\tx",
            "files\t8\t\"*?",
        ] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_request_line_is_what_the_helper_reads() {
        let file = Request::File {
            key: "k".into(),
            path: r"C:\a.pdf".into(),
            size: 40,
        };
        assert_eq!(parse(&file.line()), Some(Line::File(40, r"C:\a.pdf")));
        let files = Request::Files {
            generation: 3,
            words: vec!["a".into(), "b".into()],
            limit: 8,
        };
        assert_eq!(
            parse(&files.line()),
            Some(Line::Files(8, vec!["a".into(), "b".into()]))
        );
    }

    #[test]
    fn an_answer_is_its_length_and_the_bytes() {
        let mut stream: &[u8] = &[3, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0];
        assert_eq!(answer(&mut stream).unwrap(), Some(vec![1, 2, 3]));
        assert_eq!(answer(&mut stream).unwrap(), None);
        assert!(answer(&mut stream).is_err(), "the helper is gone");
        let mut too_big: &[u8] = &u32::MAX.to_le_bytes();
        assert!(answer(&mut too_big).is_err());
    }

    #[test]
    fn a_search_answer_carries_its_generation() {
        let request = Request::Files {
            generation: 7,
            words: vec!["a".into()],
            limit: 8,
        };
        let text = launcher_files::encode_hits(&[FileHit {
            path: r"C:\a.txt".into(),
            folder: false,
        }]);
        match request.answered(Some(text.into_bytes())) {
            Answer::Files { generation, hits } => {
                assert_eq!(generation, 7);
                assert_eq!(hits.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn only_the_last_search_waiting_is_sent_after_the_icons() {
        let search = |generation| Request::Files {
            generation,
            words: vec!["a".into()],
            limit: 8,
        };
        let icon = |id: &str| Request::App {
            id: id.into(),
            size: 32,
        };
        let (send, dropped) = order(vec![search(1), icon("a"), search(2), icon("b")]);
        assert_eq!(send, [icon("a"), icon("b"), search(2)]);
        assert_eq!(dropped, [search(1)]);
    }
}
