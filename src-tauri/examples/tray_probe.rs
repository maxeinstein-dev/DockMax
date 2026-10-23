//! Runs the tray host (`src/windows/tray_host.rs`) on its own for a few
//! seconds and lists the icons programs sent it; given a name, it then
//! clicks the icon whose tooltip has it, the way the tray panel does. For
//! looking, not for the app:
//!
//! `cargo run --example tray_probe -- [seconds] [tooltip part] [left|right]`

#[path = "../src/windows/tray_host.rs"]
mod tray_host;

// The host listens to the menus programs open (a sibling module in the app).
#[allow(dead_code, reason = "the probe uses only the hook of this module")]
#[path = "../src/windows/popup_menu.rs"]
mod popup_menu;

fn main() {
    let mut args = std::env::args().skip(1);
    let seconds: u64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(10);
    let wanted = args.next();
    let button = match args.next().as_deref() {
        Some("left") => tray_host::Button::Left,
        _ => tray_host::Button::Right,
    };
    if std::env::var_os("TRAY_TRACE").is_some() {
        tray_host::trace(true);
    }
    assert!(
        tray_host::start(|| println!("changed")),
        "the host did not start"
    );
    println!("host up; waiting {seconds} s for programs to send their icons");
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    let icons = tray_host::icons();
    println!("{} icons", icons.len());
    for icon in &icons {
        println!(
            "{:>14} uid={:<4} cb={:#x} v={} hidden={} image={} tip={:?}",
            icon.key,
            icon.uid,
            icon.callback,
            icon.version,
            icon.hidden,
            icon.png.as_ref().map_or(0, String::len),
            icon.tip
        );
    }
    if let Some(wanted) = wanted {
        let found = icons
            .iter()
            .find(|icon| icon.tip.to_lowercase().contains(&wanted.to_lowercase()));
        match found {
            Some(icon) => {
                println!("clicking {:?} with the {button:?} button", icon.tip);
                // TRAY_EVENTS="0x7b,0x205": only those messages, to see which
                // ones a program answers.
                let result = match std::env::var("TRAY_EVENTS") {
                    Ok(list) => {
                        let events: Vec<u32> = list
                            .split(',')
                            .filter_map(|n| {
                                u32::from_str_radix(n.trim().trim_start_matches("0x"), 16).ok()
                            })
                            .collect();
                        println!("sending only {events:#x?}");
                        tray_host::send_events(&icon.key, &events)
                    }
                    Err(_) => tray_host::click(&icon.key, button),
                };
                println!("{result:?}");
                // Time to look at the menu the program opened.
                std::thread::sleep(std::time::Duration::from_secs(8));
            }
            None => println!("no icon with {wanted:?} in its tooltip"),
        }
    }
    tray_host::stop();
    std::thread::sleep(std::time::Duration::from_millis(300));
}
