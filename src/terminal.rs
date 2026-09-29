//! A VS Code–style integrated terminal session.
//!
//! This is a *command* terminal, not a full PTY emulator: every line you
//! enter is run through the system shell (`sh -c` / `cmd /C`) in the
//! session's working directory, and its stdout/stderr stream into the
//! scrollback as they arrive. `cd`, `clear`/`cls` are handled here so the
//! working directory persists between commands. While a command runs, lines
//! you type are forwarded to its stdin (Ctrl+D closes stdin, Ctrl+C kills
//! it). Full-screen/interactive programs that need a real TTY (vim, top,
//! ssh prompts, ...) won't work.

use crate::theme::Theme;
use egui::{Key, RichText};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Scrollback cap, so a chatty command can't grow memory (and per-frame
/// layout cost) without bound.
const MAX_LINES: usize = 2000;

/// How long after the process exits we keep waiting for its output pipes to
/// close (a background grandchild can hold them open indefinitely).
const EXIT_GRACE: Duration = Duration::from_millis(400);

enum Line {
    Cmd { prompt: String, cmd: String },
    Out(String),
    Err(String),
    Info(String),
}

enum Msg {
    Out(String),
    Err(String),
    /// One of the output-reading threads hit end-of-file.
    Eof,
    /// The process exited (exit code, if it had one).
    Exited(Option<i32>),
}

pub struct Terminal {
    pub name: String,
    id: egui::Id,
    cwd: PathBuf,
    lines: Vec<Line>,
    input: String,
    history: Vec<String>,
    hist_pos: Option<usize>,

    rx: Option<mpsc::Receiver<Msg>>,
    child: Option<Arc<Mutex<Child>>>,
    stdin: Option<ChildStdin>,
    readers_open: u8,
    exited: Option<(Option<i32>, Instant)>,
}

impl Terminal {
    pub fn shell_name() -> &'static str {
        if cfg!(windows) {
            "cmd"
        } else {
            "sh"
        }
    }

    pub fn new(name: String, cwd: PathBuf, serial: usize) -> Self {
        Self {
            name,
            id: egui::Id::new(("terminal-input", serial)),
            cwd,
            lines: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            hist_pos: None,
            rx: None,
            child: None,
            stdin: None,
            readers_open: 0,
            exited: None,
        }
    }

    /// Id of this session's input field, so callers can tell whether the
    /// terminal currently has keyboard focus.
    pub fn input_id(&self) -> egui::Id {
        self.id
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }

    fn display_cwd(&self) -> String {
        if let Some(home) = Self::home_dir() {
            if let Ok(rest) = self.cwd.strip_prefix(&home) {
                if rest.as_os_str().is_empty() {
                    return "~".to_string();
                }
                return format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display());
            }
        }
        self.cwd.display().to_string()
    }

    fn prompt(&self) -> String {
        if cfg!(windows) {
            format!("{}> ", self.display_cwd())
        } else {
            format!("{} $ ", self.display_cwd())
        }
    }

    /// Drains output from the running command (if any). Call every frame,
    /// even while the panel is hidden, so background output isn't lost.
    pub fn poll(&mut self, lang: crate::i18n::Lang) {
        let mut finished = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(Msg::Out(t)) => self.lines.push(Line::Out(t)),
                    Ok(Msg::Err(t)) => self.lines.push(Line::Err(t)),
                    Ok(Msg::Eof) => self.readers_open = self.readers_open.saturating_sub(1),
                    Ok(Msg::Exited(code)) => self.exited = Some((code, Instant::now())),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }
        if let Some((code, at)) = self.exited {
            if self.readers_open == 0 || at.elapsed() > EXIT_GRACE {
                finished = true;
                if let Some(c) = code {
                    if c != 0 {
                        self.lines.push(Line::Info(crate::i18n::terminal_exit_code(lang, c)));
                    }
                }
            }
        }
        if finished {
            self.rx = None;
            self.child = None;
            self.stdin = None;
            self.exited = None;
            self.readers_open = 0;
        }
        if self.lines.len() > MAX_LINES {
            let extra = self.lines.len() - MAX_LINES;
            self.lines.drain(..extra);
        }
    }

    pub fn kill(&mut self) {
        if let Some(c) = &self.child {
            if let Ok(mut g) = c.lock() {
                let _ = g.kill();
            }
        }
        self.stdin = None;
    }

    /// Closes the running command's stdin (the terminal's Ctrl+D).
    fn send_eof(&mut self) {
        self.stdin = None;
    }

    fn history_step(&mut self, dir: i32) {
        if self.history.is_empty() {
            return;
        }
        let last = self.history.len() - 1;
        let pos = match (self.hist_pos, dir) {
            (None, -1) => Some(last),
            (Some(p), -1) => Some(p.saturating_sub(1)),
            (Some(p), 1) if p < last => Some(p + 1),
            (Some(_), 1) => None,
            (p, _) => p,
        };
        self.hist_pos = pos;
        self.input = pos.map(|p| self.history[p].clone()).unwrap_or_default();
    }

    fn submit(&mut self) {
        let line = std::mem::take(&mut self.input);
        self.hist_pos = None;

        // A command is already running: the line is input for it.
        if self.is_running() {
            self.lines.push(Line::Out(line.clone()));
            if let Some(si) = &mut self.stdin {
                let _ = writeln!(si, "{line}");
                let _ = si.flush();
            }
            return;
        }

        let line = line.trim().to_string();
        if line.is_empty() {
            self.lines.push(Line::Cmd { prompt: self.prompt(), cmd: String::new() });
            return;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.lines.push(Line::Cmd { prompt: self.prompt(), cmd: line.clone() });

        let mut parts = line.splitn(2, char::is_whitespace);
        let first = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("").trim();
        match first {
            "clear" | "cls" => {
                self.lines.clear();
            }
            "cd" | "chdir" => self.change_dir(rest),
            _ => self.spawn(&line),
        }
    }

    fn change_dir(&mut self, arg: &str) {
        let mut arg = arg.trim();
        if cfg!(windows) {
            // `cd /d D:\path`
            if let Some(r) = arg.strip_prefix("/d").or_else(|| arg.strip_prefix("/D")) {
                arg = r.trim();
            }
        }
        let arg = arg.trim_matches('"');

        if arg.is_empty() {
            if cfg!(windows) {
                self.lines.push(Line::Out(self.cwd.display().to_string()));
                return;
            }
            match Self::home_dir() {
                Some(h) => self.cwd = h,
                None => self.lines.push(Line::Err("cd: HOME not set".to_string())),
            }
            return;
        }

        let target: PathBuf = if arg == "~" {
            Self::home_dir().unwrap_or_else(|| self.cwd.clone())
        } else if let Some(r) = arg.strip_prefix("~/").or_else(|| arg.strip_prefix("~\\")) {
            Self::home_dir().unwrap_or_else(|| self.cwd.clone()).join(r)
        } else {
            let p = PathBuf::from(arg);
            if p.is_absolute() {
                p
            } else {
                self.cwd.join(p)
            }
        };

        match target.canonicalize() {
            Ok(p) if p.is_dir() => self.cwd = strip_unc(p),
            _ => self.lines.push(Line::Err(format!("cd: {arg}: No such file or directory"))),
        }
    }

    fn spawn(&mut self, line: &str) {
        #[cfg(windows)]
        let mut cmd = {
            use std::os::windows::process::CommandExt;
            let mut c = Command::new("cmd");
            c.arg("/C").raw_arg(line);
            // Don't flash a console window for every command.
            c.creation_flags(0x0800_0000);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(line);
            c
        };

        cmd.current_dir(&self.cwd)
            .env("TERM", "dumb")
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.lines.push(Line::Err(e.to_string()));
                return;
            }
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        self.stdin = child.stdin.take();

        let child = Arc::new(Mutex::new(child));
        let (tx, rx) = mpsc::channel();

        let mut readers = 0u8;
        if let Some(o) = stdout {
            spawn_reader(o, tx.clone(), false);
            readers += 1;
        }
        if let Some(e) = stderr {
            spawn_reader(e, tx.clone(), true);
            readers += 1;
        }

        let waiter = child.clone();
        std::thread::spawn(move || {
            let code = loop {
                let status = waiter.lock().map(|mut c| c.try_wait());
                match status {
                    Ok(Ok(Some(st))) => break st.code(),
                    Ok(Ok(None)) => std::thread::sleep(Duration::from_millis(30)),
                    _ => break None,
                }
            };
            let _ = tx.send(Msg::Exited(code));
        });

        self.readers_open = readers;
        self.exited = None;
        self.child = Some(child);
        self.rx = Some(rx);
    }

    /// Draws the scrollback and the input line. `focus` grabs keyboard
    /// focus for the input (used when the panel is opened).
    pub fn ui(&mut self, ui: &mut egui::Ui, theme: &Theme, font_size: f32, focus: bool) {
        let font = egui::FontId::monospace(font_size);
        let input_id = self.id;
        let prompt = self.prompt();
        let running = self.is_running();

        let mut do_submit = false;
        let mut do_kill = false;
        let mut do_eof = false;
        let mut do_clear = false;
        let mut hist = 0i32;

        let area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for line in &self.lines {
                    match line {
                        Line::Cmd { prompt, cmd } => {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                prompt,
                                0.0,
                                egui::text::TextFormat { font_id: font.clone(), color: theme.fg_dim, ..Default::default() },
                            );
                            job.append(
                                cmd,
                                0.0,
                                egui::text::TextFormat { font_id: font.clone(), color: theme.fg, ..Default::default() },
                            );
                            ui.label(job);
                        }
                        Line::Out(t) => {
                            ui.label(RichText::new(t.as_str()).font(font.clone()).color(theme.fg));
                        }
                        Line::Err(t) => {
                            ui.label(RichText::new(t.as_str()).font(font.clone()).color(theme.error));
                        }
                        Line::Info(t) => {
                            ui.label(RichText::new(t.as_str()).font(font.clone()).color(theme.fg_dim));
                        }
                    }
                }

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    if !running {
                        ui.label(RichText::new(prompt.as_str()).font(font.clone()).color(theme.fg_dim));
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .id(input_id)
                            .font(font.clone())
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .lock_focus(true),
                    );
                    if focus {
                        resp.request_focus();
                    }
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        do_submit = true;
                        resp.request_focus();
                    }
                    if resp.has_focus() {
                        ui.input(|i| {
                            if i.key_pressed(Key::ArrowUp) {
                                hist = -1;
                            }
                            if i.key_pressed(Key::ArrowDown) {
                                hist = 1;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::C) && running {
                                do_kill = true;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::D) && running {
                                do_eof = true;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::L) {
                                do_clear = true;
                            }
                        });
                    }
                });
            });

        // Clicking empty space in the panel puts the caret back in the input.
        let clicked_inside =
            ui.input(|i| i.pointer.primary_clicked()) && ui.rect_contains_pointer(area.inner_rect);
        if clicked_inside {
            ui.memory_mut(|m| m.request_focus(input_id));
        }

        if do_kill {
            self.kill();
            self.lines.push(Line::Info("^C".to_string()));
        }
        if do_eof {
            self.send_eof();
        }
        if do_clear {
            self.lines.clear();
        }
        if hist != 0 {
            self.history_step(hist);
            let n = self.input.chars().count();
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), input_id).unwrap_or_default();
            state.set_ccursor_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(n),
                egui::text::CCursor::new(n),
            )));
            egui::text_edit::TextEditState::store(state, ui.ctx(), input_id);
        }
        if do_submit {
            self.submit();
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.kill();
    }
}

fn spawn_reader<R: Read + Send + 'static>(r: R, tx: mpsc::Sender<Msg>, is_err: bool) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(r);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = clean_line(&String::from_utf8_lossy(&buf));
                    let msg = if is_err { Msg::Err(text) } else { Msg::Out(text) };
                    if tx.send(msg).is_err() {
                        return;
                    }
                }
            }
        }
        let _ = tx.send(Msg::Eof);
    });
}

/// One raw output line -> display text: no trailing newline, only the last
/// carriage-return segment (progress bars redraw a line with `\r`), no ANSI
/// escape sequences, tabs expanded.
fn clean_line(raw: &str) -> String {
    let raw = raw.trim_end_matches(['\n', '\r']);
    let raw = raw.rsplit('\r').next().unwrap_or(raw);
    strip_ansi(raw).replace('\t', "    ")
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            // CSI: ESC [ ... final byte in @..~
            Some('[') => {
                chars.next();
                for n in chars.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            // OSC: ESC ] ... BEL or ESC \
            Some(']') => {
                chars.next();
                while let Some(n) = chars.next() {
                    if n == '\u{7}' {
                        break;
                    }
                    if n == '\u{1b}' {
                        chars.next();
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    out
}

/// `canonicalize` on Windows yields `\\?\C:\...`, which `cmd.exe` rejects as
/// a working directory.
fn strip_unc(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => p,
    }
}

/// Shortens a path for a terminal tab title.
#[allow(dead_code)]
pub fn dir_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}
