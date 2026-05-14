use std::fmt::Display;

pub fn ok(msg: impl Display) {
    eprintln!("  {} {}", green("\u{2713}"), msg);
}

pub fn info(msg: impl Display) {
    eprintln!("  {} {}", dim("\u{2022}"), msg);
}

pub fn err(msg: impl Display) {
    eprintln!("  {} {}", red("\u{2717}"), msg);
}

pub fn header(title: impl Display) -> String {
    let t = title.to_string();
    let pad = 48usize.saturating_sub(t.len() + 2);
    format!(" {} {}", bold(&t), "\u{2500}".repeat(pad))
}

pub fn footer() -> String {
    format!(" {}", "\u{2500}".repeat(50))
}

pub fn elapsed(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.2}s", secs)
    } else {
        let m = (secs / 60.0) as u64;
        let s = secs - (m as f64 * 60.0);
        format!("{}m {:05.2}s", m, s)
    }
}

pub fn green(s: impl Display) -> String {
    format!("\x1b[32m{}\x1b[0m", s)
}

pub fn dim(s: impl Display) -> String {
    format!("\x1b[2m{}\x1b[0m", s)
}

pub fn bold(s: impl Display) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

pub fn red(s: impl Display) -> String {
    format!("\x1b[31m{}\x1b[0m", s)
}

pub fn format_downloads(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
