use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Tabs, Wrap,
};
use std::io::{self, Write};

use crate::config::{Config, TranslationMode};
use crate::error::Result;
use crate::language_detector::LanguageDetector;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

// ── Public API ──────────────────────────────────────────

pub async fn run(config: Config) -> Result<()> {
    let mut app = App::new(config);

    let mut terminal = ratatui::init();
    // Enable bracketed paste for copy/paste support in text fields
    let _ = write!(io::stdout(), "\x1b[?2004h");
    let _ = io::stdout().flush();

    while !app.should_quit {
        terminal.draw(|frame| app.render(frame))?;
        app.tick = app.tick.wrapping_add(1);

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
                Event::Paste(text) => app.handle_paste(&text),
                _ => {}
            }
        }

        app.check_async_results();
    }

    let _ = write!(io::stdout(), "\x1b[?2004l");
    let _ = io::stdout().flush();
    ratatui::restore();
    Ok(())
}

// ── Screens ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Models,
    Languages,
    Config,
    TranslateText,
    TranslateFile,
    Detect,
}

const SCREENS: &[(Screen, &str)] = &[
    (Screen::Models, "Models"),
    (Screen::Languages, "Languages"),
    (Screen::Config, "Config"),
    (Screen::TranslateText, "Translate"),
    (Screen::TranslateFile, "File"),
    (Screen::Detect, "Detect"),
];

// ── Async types ─────────────────────────────────────────

enum AsyncTask {
    Translate {
        text: String,
        source: String,
        target: String,
    },
    TranslateFile {
        path: String,
        source: String,
        target: String,
    },
    Detect {
        text: String,
    },
}

struct ModelEntry {
    id: String,
    display: String,
}

enum AsyncOutcome {
    TextResult(String),
    TranslationResult(
        String,
        Option<std::sync::Arc<crate::translation_service::TranslationService>>,
    ),
    ModelList(Vec<ModelEntry>),
    DownloadDone(Option<String>),
}

// ── Focus ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Menu,
    Content,
    Search,
}

// ── Sub-states ──────────────────────────────────────────

struct InputState {
    text: String,
    source: String,
    target: String,
    file: String,
    active_field: usize,
    cursor_pos: usize,
    path_completions: Vec<String>,
    path_completion_idx: usize,
}

struct LangPickerState {
    cursor: usize,
    search: String,
    cache: Vec<crate::languages::LanguageInfo>,
}

struct ModelsState {
    installed: Vec<String>,
    scroll: usize,
    browsing: bool,
    results: Vec<ModelEntry>,
    cursor: usize,
    search_query: String,
}

struct AsyncState {
    is_loading: bool,
    loading_msg: String,
    result_text: Option<String>,
    result_is_error: bool,
    rx: Option<tokio::sync::oneshot::Receiver<AsyncOutcome>>,
    handle: Option<tokio::task::JoinHandle<()>>,
    service: Option<SvcArc>,
}

// ── App state ───────────────────────────────────────────

struct App {
    config: Config,
    focus: Focus,
    menu_index: usize,
    screen: Screen,
    should_quit: bool,
    tick: u64,

    search_query: String,
    search_active: bool,

    language_scroll: usize,
    lang_filter_cache: Vec<crate::languages::LanguageInfo>,

    quitting: bool,
    quit_choice: usize,

    service_picker: bool,
    service_picker_cursor: usize,

    input: InputState,
    lang_picker: LangPickerState,
    models: ModelsState,
    async_st: AsyncState,
}

impl App {
    fn new(config: Config) -> Self {
        let src = config.translation.default_source_lang.clone();
        let tgt = config
            .translation
            .default_target_langs
            .first()
            .cloned()
            .unwrap_or_else(|| "fr".to_string());
        let mut app = Self {
            config,
            focus: Focus::Menu,
            menu_index: 0,
            screen: Screen::Models,
            should_quit: false,
            tick: 0,
            search_query: String::new(),
            search_active: false,
            language_scroll: 0,
            lang_filter_cache: crate::languages::LANGUAGES.to_vec(),
            quitting: false,
            quit_choice: 0,
            service_picker: false,
            service_picker_cursor: 0,
            input: InputState {
                text: String::new(),
                source: src,
                target: tgt,
                file: String::new(),
                active_field: 0,
                cursor_pos: 0,
                path_completions: Vec::new(),
                path_completion_idx: 0,
            },
            lang_picker: LangPickerState {
                cursor: 0,
                search: String::new(),
                cache: crate::languages::LANGUAGES.to_vec(),
            },
            models: ModelsState {
                installed: Vec::new(),
                scroll: 0,
                browsing: false,
                results: Vec::new(),
                cursor: 0,
                search_query: String::new(),
            },
            async_st: AsyncState {
                is_loading: false,
                loading_msg: String::new(),
                result_text: None,
                result_is_error: false,
                rx: None,
                handle: None,
                service: None,
            },
        };
        app.refresh_installed();
        app
    }

    fn refresh_installed(&mut self) {
        #[cfg(feature = "local")]
        {
            self.models.installed = crate::manager::list_installed_models();
        }
    }

    fn start_fetch_models(&mut self, query: &str) {
        let (tx, rx) = tokio::sync::oneshot::channel::<AsyncOutcome>();
        self.async_st.is_loading = true;
        self.async_st.loading_msg = "Fetching models…".to_string();
        self.async_st.rx = Some(rx);
        let q = query.to_string();
        self.async_st.handle = Some(tokio::spawn(async move {
            let models = crate::manager::fetch_all_models(&q).await;
            match models {
                Ok(list) => {
                    let items: Vec<ModelEntry> = list
                        .iter()
                        .map(|m| {
                            let display = format!(
                                "{:<55} {:>8}  {:>5}",
                                m.id,
                                crate::output::format_downloads(m.downloads.unwrap_or(0)),
                                m.likes.unwrap_or(0),
                            );
                            ModelEntry {
                                id: m.id.clone(),
                                display,
                            }
                        })
                        .collect();
                    let _ = tx.send(AsyncOutcome::ModelList(items));
                }
                Err(e) => {
                    let _ = tx.send(AsyncOutcome::TextResult(format!("✗ {}", e)));
                }
            }
        }));
    }

    fn start_download(&mut self, repo: &str) {
        let (tx, rx) = tokio::sync::oneshot::channel::<AsyncOutcome>();
        self.async_st.is_loading = true;
        self.async_st.loading_msg =
            format!("Downloading {}…", repo.split('/').nth(1).unwrap_or(repo));
        self.async_st.rx = Some(rx);
        let r = repo.to_string();
        self.async_st.handle = Some(tokio::spawn(async move {
            #[cfg(feature = "local")]
            {
                let err = crate::manager::download_model(&r)
                    .await
                    .err()
                    .map(|e| e.to_string());
                let _ = tx.send(AsyncOutcome::DownloadDone(err));
            }
            #[cfg(not(feature = "local"))]
            {
                let _ = r;
                let _ = tx.send(AsyncOutcome::DownloadDone(Some(
                    "Feature 'local' not compiled. Rebuild with --features local".to_string(),
                )));
            }
        }));
    }

    fn check_async_results(&mut self) {
        let Some(rx) = self.async_st.rx.as_mut() else {
            return;
        };
        match rx.try_recv() {
            Ok(outcome) => {
                self.async_st.is_loading = false;
                self.async_st.rx = None;
                self.async_st.handle = None;
                match outcome {
                    AsyncOutcome::TextResult(text) => {
                        self.async_st.result_is_error = text.starts_with('✗');
                        self.async_st.result_text = Some(text);
                    }
                    AsyncOutcome::TranslationResult(text, maybe_svc) => {
                        if let Some(svc) = maybe_svc {
                            self.async_st.service = Some(svc);
                        }
                        self.async_st.result_is_error = text.starts_with('✗');
                        self.async_st.result_text = Some(text);
                    }
                    AsyncOutcome::ModelList(list) => {
                        self.models.results = list;
                        self.models.cursor = 0;
                        self.models.browsing = true;
                        self.focus = Focus::Content;
                    }
                    AsyncOutcome::DownloadDone(err) => match err {
                        None => {
                            self.refresh_installed();
                            self.async_st.result_text = Some("✓ Download complete".to_string());
                            self.async_st.result_is_error = false;
                        }
                        Some(msg) => {
                            self.async_st.result_text = Some(format!("✗ {}", msg));
                            self.async_st.result_is_error = true;
                        }
                    },
                }
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                self.async_st.is_loading = false;
                self.async_st.rx = None;
                self.async_st.handle = None;
            }
            Err(_) => {}
        }
    }

    // ── Render ──────────────────────────────────────────

    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);
        self.render_header(frame, chunks[0]);
        self.render_tabs(frame, chunks[1]);
        self.render_content(frame, chunks[2]);
        self.render_footer(frame, chunks[3]);
        if self.quitting {
            self.render_quit_dialog(frame, area);
        }
        if self.service_picker {
            self.render_service_picker(frame, area);
        }
    }

    fn render_header(&self, frame: &mut ratatui::Frame, area: Rect) {
        let spinner = SPINNER[(self.tick / 2) as usize % SPINNER.len()];
        let left_spans = if self.async_st.is_loading {
            vec![
                Span::styled(
                    " ◆ zellig",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{} {}", spinner, self.async_st.loading_msg),
                    Style::default().fg(Color::Yellow),
                ),
            ]
        } else {
            vec![Span::styled(
                " ◆ zellig",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]
        };

        let right_text = if self.search_active {
            let q = if self.screen == Screen::Models {
                &self.models.search_query
            } else {
                &self.search_query
            };
            format!("🔍 {}  ", q)
        } else {
            let name = SCREENS
                .iter()
                .find(|(s, _)| *s == self.screen)
                .map(|(_, n)| *n)
                .unwrap_or("");
            format!("{}  ", name)
        };
        let right_style = if self.search_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let hchunks = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(right_text.chars().count() as u16 + 2),
        ])
        .split(Rect::new(area.x, area.y, area.width, 1));
        frame.render_widget(Paragraph::new(Line::from(left_spans)), hchunks[0]);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&right_text, right_style)))
                .alignment(Alignment::Right),
            hchunks[1],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            ))),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }

    fn render_tabs(&self, frame: &mut ratatui::Frame, area: Rect) {
        let tab_area = Rect::new(area.x, area.y, area.width, 1);
        let sep_area = Rect::new(area.x, area.y + 1, area.width, 1);
        let titles: Vec<&str> = SCREENS.iter().map(|(_, label)| *label).collect();
        let tabs = Tabs::new(titles)
            .select(self.menu_index)
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider(Span::styled("│", Style::default().fg(Color::DarkGray)));
        frame.render_widget(tabs, tab_area);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            ))),
            sep_area,
        );
    }

    fn render_content(&self, frame: &mut ratatui::Frame, area: Rect) {
        match self.screen {
            Screen::Models => self.render_models(frame, area),
            Screen::Languages => self.render_languages(frame, area),
            Screen::Config => self.render_config(frame, area),
            Screen::TranslateText => self.render_translate(frame, area, false),
            Screen::TranslateFile => self.render_translate(frame, area, true),
            Screen::Detect => self.render_detect(frame, area),
        }
    }

    fn render_models(&self, frame: &mut ratatui::Frame, area: Rect) {
        if self.models.browsing {
            self.render_models_browse(frame, area);
        } else {
            self.render_models_installed(frame, area);
        }
        if let Some(ref r) = self.async_st.result_text {
            let w = 70.min(area.width.saturating_sub(4));
            let (fg, icon) = if self.async_st.result_is_error {
                (Color::Red, "✗")
            } else {
                (Color::Green, "✓")
            };
            let text = format!(" {} {}", icon, r.trim_start_matches(['✗', '✓', ' ']));
            let inner_w = w.saturating_sub(2) as usize;
            let text_lines = text.len().div_ceil(inner_w);
            let h = (text_lines as u16 + 2).clamp(3, 8);
            let x = area.x + (area.width.saturating_sub(w)) / 2;
            let y = area.y + area.height.saturating_sub(h + 1);
            let popup = Rect::new(x, y, w, h);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(fg))
                    .block(styled_block("").border_style(Style::default().fg(fg))),
                popup,
            );
        }
    }

    fn render_models_installed(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);

        // Hint bar
        let hint = Line::from(vec![
            Span::styled(" ", Style::default()),
            hint_key("Enter"),
            Span::styled("=select  ", Style::default().fg(Color::DarkGray)),
            hint_key("I"),
            Span::styled("=browse  ", Style::default().fg(Color::DarkGray)),
            hint_key("D"),
            Span::styled("=uninstall  ", Style::default().fg(Color::DarkGray)),
            hint_key("R"),
            Span::styled("=refresh  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("  {} installed", self.models.installed.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(hint), chunks[0]);

        let list_area = chunks[1];
        if self.models.installed.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  No installed models — press I to browse HuggingFace",
                    Style::default().fg(Color::DarkGray),
                )))
                .block(styled_block(" Installed ")),
                list_area,
            );
            return;
        }

        let items: Vec<ListItem> = self
            .models
            .installed
            .iter()
            .map(|m| {
                let is_cfg = m == &self.config.local.model_repo;
                let prefix = if is_cfg { "★ " } else { "  " };
                let style = if is_cfg {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{}{}", prefix, m)).style(style)
            })
            .collect();

        let visible = list_area.height.saturating_sub(2) as usize;
        let needs_bar = items.len() > visible;
        let inner = if needs_bar {
            Rect::new(
                list_area.x,
                list_area.y,
                list_area.width.saturating_sub(1),
                list_area.height,
            )
        } else {
            list_area
        };
        let mut state = ListState::default().with_selected(Some(self.models.scroll));
        frame.render_stateful_widget(
            List::new(items)
                .block(styled_block(" Installed "))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green)),
            inner,
            &mut state,
        );
        if needs_bar {
            let bar_area = Rect::new(inner.right(), list_area.y, 1, list_area.height);
            let mut bar_state = ScrollbarState::new(self.models.installed.len())
                .position(self.models.scroll)
                .viewport_content_length(visible);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                bar_area,
                &mut bar_state,
            );
        }
    }

    fn render_models_browse(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1), // search bar
            Constraint::Length(1), // column header
            Constraint::Min(1),    // list
        ])
        .split(area);

        // Search bar
        let search_line = if self.async_st.is_loading {
            Line::from(vec![Span::styled(
                format!(
                    " {} {}",
                    SPINNER[(self.tick / 2) as usize % SPINNER.len()],
                    self.async_st.loading_msg
                ),
                Style::default().fg(Color::Yellow),
            )])
        } else if self.search_active {
            Line::from(vec![
                Span::styled(" 🔍 ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    &self.models.search_query,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("█", Style::default().fg(Color::Yellow)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    " / to search  │  current: ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    if self.models.search_query.is_empty() {
                        "all ct2 models".to_string()
                    } else {
                        self.models.search_query.clone()
                    },
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        };
        frame.render_widget(Paragraph::new(search_line), chunks[0]);

        // Fixed column header
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                format!("  {:<55} {:>8}  {:>5}", "Model", "Downloads", "Likes"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )])),
            chunks[1],
        );

        if self.async_st.is_loading && self.models.results.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  Fetching…",
                    Style::default().fg(Color::DarkGray),
                ))),
                chunks[2],
            );
            return;
        }

        if self.models.results.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  Type a search term and press Enter (shows all CTranslate2 translation models by default)",
                    Style::default().fg(Color::DarkGray),
                ))),
                chunks[2],
            );
            return;
        }

        let installed_set: std::collections::HashSet<&str> =
            self.models.installed.iter().map(|s| s.as_str()).collect();
        let items: Vec<ListItem> = self
            .models
            .results
            .iter()
            .map(|entry| {
                let marker = if installed_set.contains(entry.id.as_str()) {
                    "✓ "
                } else {
                    "  "
                };
                ListItem::new(format!("{}{}", marker, entry.display))
            })
            .collect();

        let list_h = chunks[2].height;
        let visible = list_h.saturating_sub(2) as usize;
        let needs_bar = self.models.results.len() > visible;
        let list_area = if needs_bar {
            Rect::new(
                chunks[2].x,
                chunks[2].y,
                chunks[2].width.saturating_sub(1),
                list_h,
            )
        } else {
            chunks[2]
        };
        let mut state = ListState::default().with_selected(Some(self.models.cursor));
        frame.render_stateful_widget(
            List::new(items)
                .block(styled_block(" Available "))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green)),
            list_area,
            &mut state,
        );
        if needs_bar {
            let bar_area = Rect::new(chunks[2].right().saturating_sub(1), chunks[2].y, 1, list_h);
            let mut bar_state = ScrollbarState::new(self.models.results.len())
                .position(self.models.cursor)
                .viewport_content_length(visible);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                bar_area,
                &mut bar_state,
            );
        }
    }

    fn render_languages(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);

        // Fixed header row
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    "  {:<6} {:<26} {:<30} {}",
                    "ISO", "NLLB Code", "English Name", "Native Name"
                ),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            ))),
            chunks[0],
        );

        let items: Vec<ListItem> = self
            .filtered_languages()
            .iter()
            .map(|(iso, nllb, eng, native)| {
                let iso_display = if iso.is_empty() { "—" } else { iso };
                ListItem::new(format!(
                    "  {:<6} {:<26} {:<30} {}",
                    iso_display, nllb, eng, native
                ))
            })
            .collect();

        let title = format!(
            " Languages  {}/{} ",
            items.len(),
            crate::languages::LANGUAGES.len()
        );
        let total = items.len();
        let visible = chunks[1].height.saturating_sub(2) as usize;
        let needs_bar = total > visible;
        let list_area = if needs_bar {
            Rect::new(
                chunks[1].x,
                chunks[1].y,
                chunks[1].width.saturating_sub(1),
                chunks[1].height,
            )
        } else {
            chunks[1]
        };
        let mut state = ListState::default().with_offset(self.language_scroll);
        frame.render_stateful_widget(
            List::new(items)
                .block(styled_block(title.as_str()))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green)),
            list_area,
            &mut state,
        );
        if needs_bar {
            let bar_area = Rect::new(
                chunks[1].right().saturating_sub(1),
                chunks[1].y,
                1,
                chunks[1].height,
            );
            let mut bar_state = ScrollbarState::new(total)
                .position(self.language_scroll)
                .viewport_content_length(visible);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                bar_area,
                &mut bar_state,
            );
        }
    }

    fn render_config(&self, frame: &mut ratatui::Frame, area: Rect) {
        let mode_style = match &self.config.mode {
            TranslationMode::Local => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            TranslationMode::Ai => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            TranslationMode::Google | TranslationMode::Lingva => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            TranslationMode::LibreTranslate | TranslationMode::Bergamot => Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            TranslationMode::DeepL | TranslationMode::Yandex | TranslationMode::Azure => {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            }
            TranslationMode::Baidu | TranslationMode::Youdao | TranslationMode::Qq => {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            }
        };

        let mode_str = self.config.mode.display_name().to_string();
        let dg = Style::default().fg(Color::DarkGray);

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                " ─── Translation ─────────────────────────",
                dg,
            )),
            Line::from(vec![
                Span::styled("   Service         ", dg),
                Span::styled(mode_str, mode_style),
                Span::styled("   (M to select)", dg),
            ]),
        ];

        if let Some(limit) = self.config.mode.char_limit() {
            lines.push(Line::from(vec![
                Span::styled("   Char limit      ", dg),
                Span::styled(format!("{}", limit), dg),
            ]));
        }

        lines.push(Line::from(""));

        // Service-specific config section
        match &self.config.mode {
            TranslationMode::Ai => {
                lines.push(Line::from(Span::styled(
                    " ─── AI Settings ─────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   Provider        ", dg),
                    Span::styled(self.config.ai.provider.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Model           ", dg),
                    Span::styled(self.config.ai.model.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   API Key         ", dg),
                    Span::styled(
                        if self.config.ai.api_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.ai.api_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
            }
            TranslationMode::Local => {
                lines.push(Line::from(Span::styled(
                    " ─── Local Model ─────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   Repo            ", dg),
                    Span::styled(self.config.local.model_repo.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Format          ", dg),
                    Span::styled(self.config.local.model_format.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Compute         ", dg),
                    Span::styled(self.config.local.compute_type.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Beam size       ", dg),
                    Span::styled(self.config.local.beam_size.to_string(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Threads         ", dg),
                    Span::styled(
                        if self.config.local.num_threads == 0 {
                            "auto".to_string()
                        } else {
                            self.config.local.num_threads.to_string()
                        },
                        Style::default(),
                    ),
                ]));
            }
            TranslationMode::Google => {
                lines.push(Line::from(Span::styled(
                    " ─── Google Translate ────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   API key         ", dg),
                    Span::styled(
                        "not required (unofficial)",
                        Style::default().fg(Color::Green),
                    ),
                ]));
            }
            TranslationMode::DeepL => {
                lines.push(Line::from(Span::styled(
                    " ─── DeepL ───────────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   API Key         ", dg),
                    Span::styled(
                        if self.config.deepl.api_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.deepl.api_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Endpoint        ", dg),
                    Span::styled(
                        if self.config.deepl.pro { "Pro" } else { "Free" },
                        Style::default(),
                    ),
                ]));
            }
            TranslationMode::Yandex => {
                lines.push(Line::from(Span::styled(
                    " ─── Yandex Translate ────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   API Key         ", dg),
                    Span::styled(
                        if self.config.yandex.api_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.yandex.api_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                if let Some(ref fid) = self.config.yandex.folder_id {
                    lines.push(Line::from(vec![
                        Span::styled("   Folder ID       ", dg),
                        Span::styled(fid.clone(), Style::default()),
                    ]));
                }
            }
            TranslationMode::LibreTranslate => {
                lines.push(Line::from(Span::styled(
                    " ─── LibreTranslate ──────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   URL             ", dg),
                    Span::styled(self.config.libre_translate.url.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   API Key         ", dg),
                    Span::styled(
                        if self.config.libre_translate.api_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(optional)".to_string()
                        },
                        if self.config.libre_translate.api_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
            }
            TranslationMode::Azure => {
                lines.push(Line::from(Span::styled(
                    " ─── Azure Translator ────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   API Key         ", dg),
                    Span::styled(
                        if self.config.azure.api_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.azure.api_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Region          ", dg),
                    Span::styled(self.config.azure.region.clone(), Style::default()),
                ]));
            }
            TranslationMode::Bergamot => {
                lines.push(Line::from(Span::styled(
                    " ─── Bergamot ────────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   Server URL      ", dg),
                    Span::styled(self.config.bergamot.url.clone(), Style::default()),
                ]));
            }
            TranslationMode::Baidu => {
                lines.push(Line::from(Span::styled(
                    " ─── Baidu Fanyi ─────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   App ID          ", dg),
                    Span::styled(
                        if self.config.baidu.app_id.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.baidu.app_id.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Secret Key      ", dg),
                    Span::styled(
                        if self.config.baidu.secret_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.baidu.secret_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
            }
            TranslationMode::Youdao => {
                lines.push(Line::from(Span::styled(
                    " ─── Youdao Fanyi ────────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   App Key         ", dg),
                    Span::styled(
                        if self.config.youdao.app_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.youdao.app_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   App Secret      ", dg),
                    Span::styled(
                        if self.config.youdao.app_secret.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.youdao.app_secret.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
            }
            TranslationMode::Qq => {
                lines.push(Line::from(Span::styled(
                    " ─── QQ Fanyi (Tencent) ──────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   Secret ID       ", dg),
                    Span::styled(
                        if self.config.qq.secret_id.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.qq.secret_id.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Secret Key      ", dg),
                    Span::styled(
                        if self.config.qq.secret_key.is_some() {
                            "●●●●●●●●".to_string()
                        } else {
                            "(not set)".to_string()
                        },
                        if self.config.qq.secret_key.is_some() {
                            Style::default().fg(Color::Green)
                        } else {
                            dg
                        },
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Region          ", dg),
                    Span::styled(self.config.qq.region.clone(), Style::default()),
                ]));
            }
            TranslationMode::Lingva => {
                lines.push(Line::from(Span::styled(
                    " ─── Lingva Translate ────────────────────",
                    dg,
                )));
                lines.push(Line::from(vec![
                    Span::styled("   Instance URL    ", dg),
                    Span::styled(self.config.lingva.instance_url.clone(), Style::default()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   API key         ", dg),
                    Span::styled("not required", Style::default().fg(Color::Green)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " ─── Translation Defaults ────────────────",
            dg,
        )));
        lines.push(Line::from(vec![
            Span::styled("   Source          ", dg),
            Span::styled(
                self.config.translation.default_source_lang.clone(),
                Style::default(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Targets         ", dg),
            Span::styled(
                self.config.translation.default_target_langs.join(", "),
                Style::default(),
            ),
        ]));

        if let Some(ref ctx) = self.config.translation.context {
            lines.push(Line::from(vec![
                Span::styled("   Context         ", dg),
                Span::styled(ctx.clone(), Style::default().fg(Color::Yellow)),
            ]));
        }

        frame.render_widget(
            Paragraph::new(lines).block(styled_block(" Configuration ")),
            area,
        );
    }

    fn render_service_picker(&self, frame: &mut ratatui::Frame, area: Rect) {
        let all = TranslationMode::all();
        let n = all.len() as u16;
        let w = 44u16;
        let h = (n + 2).min(area.height.saturating_sub(4));
        let dialog = Rect::new(
            area.x + area.width.saturating_sub(w) / 2,
            area.y + area.height.saturating_sub(h) / 2,
            w.min(area.width),
            h,
        );
        frame.render_widget(Clear, dialog);

        let items: Vec<ListItem> = all
            .iter()
            .map(|mode| {
                let is_active = *mode == self.config.mode;
                let marker = if is_active { "●" } else { "○" };
                let name = mode.display_name();
                let key_tag = if mode.needs_key() { "[key]" } else { "     " };
                let pad = " ".repeat(20usize.saturating_sub(name.len()));
                ListItem::new(format!("  {} {}{}{}", marker, name, pad, key_tag))
            })
            .collect();

        let mut state = ListState::default().with_selected(Some(self.service_picker_cursor));
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
                .block(
                    Block::default()
                        .title(" Select Service ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Cyan)),
                ),
            dialog,
            &mut state,
        );
    }

    fn render_translate(&self, frame: &mut ratatui::Frame, area: Rect, is_file: bool) {
        let picking = self.input.active_field == 1 || self.input.active_field == 2;
        let input_h = if is_file { 3u16 } else { 5u16 };
        // When picking: hide result area and give all remaining rows to the lang picker
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(if picking { 3 } else { input_h }),
            if picking {
                Constraint::Min(5)
            } else {
                Constraint::Length(0)
            },
            if picking {
                Constraint::Length(0)
            } else {
                Constraint::Min(3)
            },
        ])
        .split(area);

        // Lang row
        let lang_chunks =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[0]);
        let src_display = Self::lang_display_name(&self.input.source);
        let tgt_display = Self::lang_display_name(&self.input.target);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(src_display, Style::default()),
            ]))
            .block(styled_block(" Source ↔ ").border_style(field_style(
                self.input.active_field == 1,
                self.focus != Focus::Menu,
            ))),
            lang_chunks[0],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(tgt_display, Style::default()),
            ]))
            .block(styled_block(" Target ").border_style(field_style(
                self.input.active_field == 2,
                self.focus != Focus::Menu,
            ))),
            lang_chunks[1],
        );

        // Text / file input
        let inp = if is_file {
            &self.input.file
        } else {
            &self.input.text
        };
        let inp_title = if is_file { " File path " } else { " Text " };
        frame.render_widget(
            Paragraph::new(inp.as_str())
                .wrap(Wrap { trim: false })
                .block(styled_block(inp_title).border_style(field_style(
                    self.input.active_field == 0,
                    self.focus != Focus::Menu,
                ))),
            chunks[1],
        );
        if self.focus != Focus::Menu && self.input.active_field == 0 {
            let field_w = chunks[1].width.saturating_sub(2) as usize;
            let display_x = self.input.cursor_pos.min(field_w.saturating_sub(1));
            let cx = chunks[1].x + 1 + display_x as u16;
            frame.set_cursor_position(Position::new(cx, chunks[1].y + 1));
        }

        // Path completion dropdown overlay
        if is_file && self.input.active_field == 0 && !self.input.path_completions.is_empty() {
            let comp_y = chunks[1].y + chunks[1].height;
            let avail_h = (area.y + area.height).saturating_sub(comp_y);
            if avail_h > 2 {
                let h = (self.input.path_completions.len().min(8) as u16 + 2).min(avail_h);
                let comp_area = Rect::new(chunks[1].x, comp_y, chunks[1].width, h);
                frame.render_widget(Clear, comp_area);
                let items: Vec<ListItem> = self
                    .input
                    .path_completions
                    .iter()
                    .map(|p| {
                        let name = std::path::Path::new(p.trim_end_matches('/'))
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(p);
                        let display = if p.ends_with('/') {
                            format!(" {}/", name)
                        } else {
                            format!(" {}", name)
                        };
                        ListItem::new(display)
                    })
                    .collect();
                let idx = self
                    .input
                    .path_completion_idx
                    .min(self.input.path_completions.len().saturating_sub(1));
                let mut state = ListState::default().with_selected(Some(idx));
                frame.render_stateful_widget(
                    List::new(items)
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
                        .block(
                            styled_block(" completions ")
                                .border_style(Style::default().fg(Color::Cyan)),
                        ),
                    comp_area,
                    &mut state,
                );
            }
        }

        // Language picker
        if picking && chunks[2].height >= 2 {
            let picker_area = chunks[2];
            let label = if self.input.active_field == 1 {
                "Source"
            } else {
                "Target"
            };
            let langs = self.lang_picker_filtered();

            // Compact 1-row filter bar
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!(" {} › ", label),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        &self.lang_picker.search,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("█", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!(
                            "  ({} match{})",
                            langs.len(),
                            if langs.len() == 1 { "" } else { "es" }
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                Rect::new(picker_area.x, picker_area.y, picker_area.width, 1),
            );

            let list_area = Rect::new(
                picker_area.x,
                picker_area.y + 1,
                picker_area.width,
                picker_area.height.saturating_sub(1),
            );
            if list_area.height < 1 {
            } else if langs.is_empty() {
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "  No matching languages — try ISO code, NLLB code, or English name",
                        Style::default().fg(Color::DarkGray),
                    ))),
                    list_area,
                );
            } else {
                let idx = self.lang_picker.cursor.min(langs.len().saturating_sub(1));
                let list_items: Vec<ListItem> = langs
                    .iter()
                    .map(|(iso, nllb, eng, native)| {
                        // Show ISO when available, NLLB code otherwise so all languages are identifiable
                        let code = Self::lang_code_display(iso, nllb);
                        ListItem::new(format!("  {:<10} {:<30} {}", code, eng, native))
                    })
                    .collect();
                let visible = list_area.height as usize;
                let needs_bar = langs.len() > visible;
                let inner = if needs_bar {
                    Rect::new(
                        list_area.x,
                        list_area.y,
                        list_area.width.saturating_sub(1),
                        list_area.height,
                    )
                } else {
                    list_area
                };
                let mut state = ListState::default().with_selected(Some(idx));
                frame.render_stateful_widget(
                    List::new(list_items)
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green)),
                    inner,
                    &mut state,
                );
                if needs_bar {
                    let bar_area = Rect::new(
                        list_area.right().saturating_sub(1),
                        list_area.y,
                        1,
                        list_area.height,
                    );
                    let mut bar_state = ScrollbarState::new(langs.len())
                        .position(idx)
                        .viewport_content_length(visible);
                    frame.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight),
                        bar_area,
                        &mut bar_state,
                    );
                }
            }
        }

        // Result
        let ra = chunks[3];
        if ra.height < 2 {
            return;
        }
        if self.async_st.is_loading {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    format!(
                        " {} {}",
                        SPINNER[(self.tick / 2) as usize % SPINNER.len()],
                        self.async_st.loading_msg
                    ),
                    Style::default().fg(Color::Yellow),
                )]))
                .block(styled_block(" Result ")),
                ra,
            );
        } else if let Some(ref r) = self.async_st.result_text {
            let (fg, prefix) = if self.async_st.result_is_error {
                (Color::Red, " ✗ ")
            } else {
                (Color::LightGreen, " ")
            };
            let display = format!("{}{}", prefix, r.trim_start_matches(['✗', '✓', ' ']));
            frame.render_widget(
                Paragraph::new(display.as_str())
                    .style(Style::default().fg(fg))
                    .wrap(Wrap { trim: false })
                    .block(styled_block(" Result ").border_style(Style::default().fg(fg))),
                ra,
            );
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Enter text above and press Enter to translate",
                    Style::default().fg(Color::DarkGray),
                )))
                .block(styled_block(" Result ")),
                ra,
            );
        }
    }

    fn render_detect(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
        frame.render_widget(
            Paragraph::new(self.input.text.as_str())
                .wrap(Wrap { trim: false })
                .block(
                    styled_block(" Text ")
                        .border_style(field_style(true, self.focus != Focus::Menu)),
                ),
            chunks[0],
        );
        if self.focus != Focus::Menu {
            let field_w = chunks[0].width.saturating_sub(2) as usize;
            let display_x = self.input.cursor_pos.min(field_w.saturating_sub(1));
            let cx = chunks[0].x + 1 + display_x as u16;
            frame.set_cursor_position(Position::new(cx, chunks[0].y + 1));
        }

        let ra = chunks[1];
        if self.async_st.is_loading {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    format!(
                        " {} {}",
                        SPINNER[(self.tick / 2) as usize % SPINNER.len()],
                        self.async_st.loading_msg
                    ),
                    Style::default().fg(Color::Yellow),
                )]))
                .block(styled_block(" Result ")),
                ra,
            );
        } else if let Some(ref r) = self.async_st.result_text {
            let (fg, prefix) = if self.async_st.result_is_error {
                (Color::Red, " ✗ ")
            } else {
                (Color::LightGreen, " ")
            };
            let display = format!("{}{}", prefix, r.trim_start_matches(['✗', '✓', ' ']));
            frame.render_widget(
                Paragraph::new(display.as_str())
                    .style(Style::default().fg(fg))
                    .block(styled_block(" Result ").border_style(Style::default().fg(fg))),
                ra,
            );
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Type text above and press Enter to detect language",
                    Style::default().fg(Color::DarkGray),
                )))
                .block(styled_block(" Result ")),
                ra,
            );
        }
    }

    fn render_footer(&self, frame: &mut ratatui::Frame, area: Rect) {
        let hints: Vec<Span> = match self.focus {
            Focus::Menu => vec![
                hint_key("←→"),
                hint_sep(" navigate  "),
                hint_key("Enter"),
                hint_sep(" select  "),
                hint_key("q"),
                hint_sep(" quit"),
            ],
            Focus::Content => {
                if self.screen == Screen::Models && self.models.browsing {
                    vec![
                        hint_key("↑↓"),
                        hint_sep(" scroll  "),
                        hint_key("Enter"),
                        hint_sep("=download  "),
                        hint_key("/"),
                        hint_sep("=search  "),
                        hint_key("I"),
                        hint_sep("=installed  "),
                        hint_key("Esc"),
                        hint_sep("=back"),
                    ]
                } else if self.screen == Screen::Models {
                    vec![
                        hint_key("↑↓"),
                        hint_sep(" scroll  "),
                        hint_key("Enter"),
                        hint_sep("=select  "),
                        hint_key("I"),
                        hint_sep("=browse  "),
                        hint_key("D"),
                        hint_sep("=uninstall  "),
                        hint_key("R"),
                        hint_sep("=refresh  "),
                        hint_key("Esc"),
                        hint_sep("=menu"),
                    ]
                } else if matches!(self.screen, Screen::TranslateText | Screen::TranslateFile) {
                    if self.input.active_field == 0 {
                        if self.screen == Screen::TranslateFile {
                            if !self.input.path_completions.is_empty() {
                                vec![
                                    hint_key("↑↓"),
                                    hint_sep("=navigate  "),
                                    hint_key("Tab/→"),
                                    hint_sep("=select  "),
                                    hint_key("Esc"),
                                    hint_sep("=dismiss"),
                                ]
                            } else {
                                vec![
                                    hint_sep("Type path  "),
                                    hint_key("→"),
                                    hint_sep("=complete  "),
                                    hint_key("Tab"),
                                    hint_sep("=lang  "),
                                    hint_key("Enter"),
                                    hint_sep("=translate  "),
                                    hint_key("Esc"),
                                    hint_sep("=menu"),
                                ]
                            }
                        } else {
                            vec![
                                hint_sep("Type  "),
                                hint_key("Tab"),
                                hint_sep("=lang  "),
                                hint_key("Enter"),
                                hint_sep("=translate  "),
                                hint_key("Esc"),
                                hint_sep("=menu"),
                            ]
                        }
                    } else {
                        vec![
                            hint_sep("Type to filter  "),
                            hint_key("↑↓"),
                            hint_sep("=navigate  "),
                            hint_key("Enter"),
                            hint_sep("=select  "),
                            hint_key("Tab"),
                            hint_sep("=next  "),
                            hint_key("Esc"),
                            hint_sep("=menu"),
                        ]
                    }
                } else if self.screen == Screen::Config {
                    vec![
                        hint_key("M"),
                        hint_sep("=select service  "),
                        hint_key("Esc"),
                        hint_sep("=menu"),
                    ]
                } else if self.screen == Screen::Detect {
                    vec![
                        hint_sep("Type  "),
                        hint_key("Enter"),
                        hint_sep("=detect  "),
                        hint_key("Esc"),
                        hint_sep("=menu"),
                    ]
                } else {
                    vec![
                        hint_key("↑↓"),
                        hint_sep(" scroll  "),
                        hint_key("/"),
                        hint_sep(" search  "),
                        hint_key("Esc"),
                        hint_sep(" menu"),
                    ]
                }
            }
            Focus::Search => vec![
                hint_sep("Type to filter  "),
                hint_key("Enter"),
                hint_sep("=confirm  "),
                hint_key("Esc"),
                hint_sep("=cancel"),
            ],
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            ))),
            Rect::new(area.x, area.y, area.width, 1),
        );
        frame.render_widget(
            Paragraph::new(Line::from(hints)).alignment(Alignment::Center),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }

    fn render_quit_dialog(&self, frame: &mut ratatui::Frame, area: Rect) {
        let w = 32u16.min(area.width.saturating_sub(4));
        let h = 6u16;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let dialog = Rect::new(x, y, w, h);
        frame.render_widget(Clear, dialog);

        let yes_style = if self.quit_choice == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let no_style = if self.quit_choice == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Quit zellig?",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [Y] Yes ", yes_style),
                    Span::raw("   "),
                    Span::styled(" [N] No ", no_style),
                ]),
            ])
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            dialog,
        );
    }

    // ── Event handling ──────────────────────────────────

    fn handle_key(&mut self, key: event::KeyEvent) {
        if self.quitting {
            self.handle_quit_key(key);
            return;
        }
        if self.service_picker {
            self.handle_service_picker_key(key);
            return;
        }
        if let KeyCode::Char('q') = key.code {
            self.quitting = true;
            self.quit_choice = 0;
            return;
        }
        match self.focus {
            Focus::Menu => self.handle_menu_key(key),
            Focus::Content => self.handle_content_key(key),
            Focus::Search => self.handle_search_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('k') => {
                self.menu_index = self.menu_index.saturating_sub(1)
            }
            KeyCode::Right | KeyCode::Char('j') => {
                self.menu_index = (self.menu_index + 1).min(SCREENS.len() - 1)
            }
            KeyCode::Tab => self.menu_index = (self.menu_index + 1).min(SCREENS.len() - 1),
            KeyCode::Enter => {
                self.screen = SCREENS[self.menu_index].0;
                self.focus = Focus::Content;
                self.input.cursor_pos = 0;
                self.async_st.result_text = None;
                self.search_query.clear();
                self.search_active = false;
                self.lang_filter_cache = crate::languages::LANGUAGES.to_vec();
                self.language_scroll = 0;
            }
            KeyCode::Esc => {
                self.quitting = true;
                self.quit_choice = 0;
            }
            _ => {}
        }
    }

    fn handle_content_key(&mut self, key: event::KeyEvent) {
        if key.code == KeyCode::Esc {
            if self.screen == Screen::Models && self.models.browsing {
                self.models.browsing = false;
                self.models.results.clear();
            }
            if let Some(h) = self.async_st.handle.take() {
                h.abort();
            }
            self.async_st.rx = None;
            self.focus = Focus::Menu;
            self.input.cursor_pos = 0;
            self.async_st.result_text = None;
            self.async_st.is_loading = false;
            return;
        }

        if key.code == KeyCode::Char('/') && self.screen_supports_search() {
            if self.screen == Screen::Models {
                self.models.browsing = true;
            }
            self.search_active = true;
            self.search_query.clear();
            self.focus = Focus::Search;
            return;
        }

        match self.screen {
            Screen::Models => self.handle_models_key(key),
            Screen::Languages => self.handle_languages_key(key),
            Screen::Config => self.handle_config_key(key),
            Screen::TranslateText | Screen::TranslateFile => self.handle_translate_key(key),
            Screen::Detect => self.handle_detect_key(key),
        }
    }

    fn handle_search_key(&mut self, key: event::KeyEvent) {
        let is_models = self.screen == Screen::Models;
        match key.code {
            KeyCode::Char(c) => {
                if is_models {
                    self.models.search_query.push(c);
                } else {
                    self.search_query.push(c);
                    self.lang_filter_cache = Self::filter_languages(&self.search_query);
                }
            }
            KeyCode::Backspace => {
                if is_models {
                    self.models.search_query.pop();
                } else {
                    self.search_query.pop();
                    self.lang_filter_cache = Self::filter_languages(&self.search_query);
                }
            }
            KeyCode::Esc => {
                self.search_active = false;
                self.focus = Focus::Content;
                if is_models {
                    if self.models.results.is_empty() {
                        self.models.browsing = false;
                    }
                } else {
                    self.search_query.clear();
                    self.lang_filter_cache = crate::languages::LANGUAGES.to_vec();
                }
            }
            KeyCode::Enter => {
                self.search_active = false;
                self.focus = Focus::Content;
                if is_models {
                    let q = self.models.search_query.clone();
                    self.start_fetch_models(&q);
                }
            }
            _ => {}
        }
    }

    fn handle_models_key(&mut self, key: event::KeyEvent) {
        if !self.models.browsing {
            match key.code {
                KeyCode::Up => {
                    if self.models.installed.is_empty() {
                        return;
                    }
                    self.models.scroll = if self.models.scroll == 0 {
                        self.models.installed.len() - 1
                    } else {
                        self.models.scroll - 1
                    };
                }
                KeyCode::Down => {
                    if self.models.installed.is_empty() {
                        return;
                    }
                    self.models.scroll = if self.models.scroll + 1 >= self.models.installed.len() {
                        0
                    } else {
                        self.models.scroll + 1
                    };
                }
                KeyCode::Enter => {
                    if !self.models.installed.is_empty() {
                        let repo = &self.models.installed
                            [self.models.scroll.min(self.models.installed.len() - 1)];
                        let config_path = crate::manager::resolve_config_path(None);
                        let _ = crate::manager::set_model_in_config(&config_path, repo);
                        self.config.local.model_repo = repo.clone();
                        self.async_st.result_text = Some(format!("✓ Configured: {}", repo));
                        self.async_st.result_is_error = false;
                    }
                }
                KeyCode::Char('i' | 'I') => {
                    self.models.browsing = true;
                    self.async_st.result_text = None;
                    if self.models.results.is_empty() {
                        self.start_fetch_models("ct2");
                    }
                }
                KeyCode::Char('d' | 'D') => {
                    if self.models.installed.is_empty() {
                        return;
                    }
                    let idx = self.models.scroll.min(self.models.installed.len() - 1);
                    let repo = self.models.installed[idx].clone();
                    #[cfg(feature = "local")]
                    {
                        let _ = crate::manager::uninstall_model_from_cache(&repo);
                        self.refresh_installed();
                        self.models.scroll = self
                            .models
                            .scroll
                            .min(self.models.installed.len().saturating_sub(1));
                    }
                    self.async_st.result_text = Some(format!("✓ Uninstalled: {}", repo));
                    self.async_st.result_is_error = false;
                }
                KeyCode::Char('r' | 'R') => self.refresh_installed(),
                _ => {}
            }
            return;
        }

        // Browsing mode
        match key.code {
            KeyCode::Up => self.models.cursor = self.models.cursor.saturating_sub(1),
            KeyCode::Down => {
                if self.models.cursor + 1 < self.models.results.len() {
                    self.models.cursor += 1;
                }
            }
            KeyCode::Enter => {
                if !self.models.results.is_empty() && !self.async_st.is_loading {
                    let repo = self.models.results[self.models.cursor].id.clone();
                    let config_path = crate::manager::resolve_config_path(None);
                    let _ = crate::manager::set_model_in_config(&config_path, &repo);
                    self.start_download(&repo);
                }
            }
            KeyCode::Char('i' | 'I') => {
                self.models.browsing = false;
                self.async_st.result_text = None;
            }
            _ => {}
        }
    }

    fn handle_languages_key(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Up => self.language_scroll = self.language_scroll.saturating_sub(1),
            KeyCode::Down => self.language_scroll += 1,
            _ => {}
        }
    }

    fn handle_config_key(&mut self, key: event::KeyEvent) {
        if key.code == KeyCode::Char('m') || key.code == KeyCode::Char('M') {
            let all = TranslationMode::all();
            self.service_picker_cursor =
                all.iter().position(|m| m == &self.config.mode).unwrap_or(0);
            self.service_picker = true;
        }
    }

    fn handle_service_picker_key(&mut self, key: event::KeyEvent) {
        let count = TranslationMode::all().len();
        match key.code {
            KeyCode::Up => {
                self.service_picker_cursor = self.service_picker_cursor.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.service_picker_cursor + 1 < count {
                    self.service_picker_cursor += 1;
                }
            }
            KeyCode::Enter => {
                self.config.mode = TranslationMode::all()
                    .into_iter()
                    .nth(self.service_picker_cursor)
                    .unwrap_or(TranslationMode::Ai);
                self.service_picker = false;
                self.async_st.service = None;
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.service_picker = false;
            }
            _ => {}
        }
    }

    fn handle_translate_key(&mut self, key: event::KeyEvent) {
        let picking = self.input.active_field == 1 || self.input.active_field == 2;

        // Completion dropdown navigation takes priority over normal key handling
        if self.screen == Screen::TranslateFile
            && self.input.active_field == 0
            && !self.input.path_completions.is_empty()
        {
            match key.code {
                KeyCode::Up => {
                    self.input.path_completion_idx =
                        self.input.path_completion_idx.saturating_sub(1);
                    return;
                }
                KeyCode::Down => {
                    let len = self.input.path_completions.len();
                    if self.input.path_completion_idx + 1 < len {
                        self.input.path_completion_idx += 1;
                    }
                    return;
                }
                KeyCode::Tab | KeyCode::Right => {
                    let idx = self
                        .input
                        .path_completion_idx
                        .min(self.input.path_completions.len() - 1);
                    self.input.file = self.input.path_completions[idx].clone();
                    self.input.cursor_pos = self.input.file.chars().count();
                    self.input.path_completions.clear();
                    return;
                }
                KeyCode::Esc => {
                    self.input.path_completions.clear();
                    return;
                }
                _ => {
                    self.input.path_completions.clear();
                    // fall through to normal handling
                }
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.input.active_field = (self.input.active_field + 1) % 3;
                self.input.cursor_pos = 0;
                self.lang_picker.search.clear();
                self.lang_picker.cache = crate::languages::LANGUAGES.to_vec();
                self.lang_picker.cursor = 0;
                self.input.path_completions.clear();
            }
            KeyCode::Up if picking => {
                self.lang_picker.cursor = self.lang_picker.cursor.saturating_sub(1);
            }
            KeyCode::Down if picking => {
                let len = self.lang_picker.cache.len();
                if len > 0 && self.lang_picker.cursor + 1 < len {
                    self.lang_picker.cursor += 1;
                }
            }
            KeyCode::Left if !picking => {
                self.input.cursor_pos = self.input.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right if !picking => {
                let pos = self.input.cursor_pos;
                let len = self.active_text_buf().chars().count();
                if pos < len {
                    self.input.cursor_pos = pos + 1;
                } else if self.screen == Screen::TranslateFile && self.input.active_field == 0 {
                    self.tab_complete_file_path();
                }
            }
            KeyCode::Home if !picking => {
                self.input.cursor_pos = 0;
            }
            KeyCode::End if !picking => {
                self.input.cursor_pos = self.active_text_buf().chars().count();
            }
            KeyCode::Delete if !picking => {
                let pos = self.input.cursor_pos;
                let buf = self.active_text_buf();
                let count = buf.chars().count();
                if pos < count {
                    let byte_pos = buf
                        .char_indices()
                        .nth(pos)
                        .map(|(i, _)| i)
                        .unwrap_or(buf.len());
                    buf.remove(byte_pos);
                }
            }
            KeyCode::Char(c) if !picking => {
                let pos = self.input.cursor_pos;
                let buf = self.active_text_buf();
                let byte_pos = buf
                    .char_indices()
                    .nth(pos)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.insert(byte_pos, c);
                self.input.cursor_pos = pos + 1;
            }
            KeyCode::Char(c) => {
                self.lang_picker.search.push(c);
                self.lang_picker.cache = Self::filter_languages(&self.lang_picker.search);
                self.lang_picker.cursor = 0;
            }
            KeyCode::Backspace if !picking => {
                let pos = self.input.cursor_pos;
                if pos > 0 {
                    let buf = self.active_text_buf();
                    let byte_pos = buf.char_indices().nth(pos - 1).map(|(i, _)| i).unwrap_or(0);
                    buf.remove(byte_pos);
                    self.input.cursor_pos = pos - 1;
                }
            }
            KeyCode::Backspace => {
                self.lang_picker.search.pop();
                self.lang_picker.cache = Self::filter_languages(&self.lang_picker.search);
                self.lang_picker.cursor = 0;
            }
            KeyCode::Enter => {
                if self.async_st.is_loading {
                    return;
                }
                if picking {
                    let len = self.lang_picker.cache.len();
                    if len > 0 {
                        let idx = self.lang_picker.cursor.min(len.saturating_sub(1));
                        let (iso, nllb, _, _) = self.lang_picker.cache[idx];
                        // Use NLLB code when ISO is absent — many NLLB-200 languages have no ISO code
                        let code = if iso.is_empty() { nllb } else { iso };
                        if self.input.active_field == 1 {
                            self.input.source = code.to_string();
                        } else {
                            self.input.target = code.to_string();
                        }
                        self.lang_picker.search.clear();
                        self.lang_picker.cache = crate::languages::LANGUAGES.to_vec();
                        self.lang_picker.cursor = 0;
                        self.input.active_field = (self.input.active_field + 1) % 3;
                        self.input.cursor_pos = 0;
                    }
                } else if self.screen == Screen::TranslateFile {
                    let path = self.input.file.trim().to_string();
                    if path.is_empty() {
                        return;
                    }
                    let (_, src, tgt) = self.collect_translate_inputs();
                    self.start_async(AsyncTask::TranslateFile {
                        path: expand_home(&path).to_string_lossy().into_owned(),
                        source: src,
                        target: tgt,
                    });
                } else {
                    let (text, src, tgt) = self.collect_translate_inputs();
                    if text.is_empty() {
                        return;
                    }
                    self.start_async(AsyncTask::Translate {
                        text,
                        source: src,
                        target: tgt,
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_detect_key(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Left => self.input.cursor_pos = self.input.cursor_pos.saturating_sub(1),
            KeyCode::Right => {
                let pos = self.input.cursor_pos;
                if pos < self.input.text.chars().count() {
                    self.input.cursor_pos = pos + 1;
                }
            }
            KeyCode::Home => self.input.cursor_pos = 0,
            KeyCode::End => self.input.cursor_pos = self.input.text.chars().count(),
            KeyCode::Delete => {
                let pos = self.input.cursor_pos;
                let count = self.input.text.chars().count();
                if pos < count {
                    let byte_pos = self
                        .input
                        .text
                        .char_indices()
                        .nth(pos)
                        .map(|(i, _)| i)
                        .unwrap_or(self.input.text.len());
                    self.input.text.remove(byte_pos);
                }
            }
            KeyCode::Char(c) => {
                let pos = self.input.cursor_pos;
                let byte_pos = self
                    .input
                    .text
                    .char_indices()
                    .nth(pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.input.text.len());
                self.input.text.insert(byte_pos, c);
                self.input.cursor_pos = pos + 1;
            }
            KeyCode::Backspace => {
                let pos = self.input.cursor_pos;
                if pos > 0 {
                    let byte_pos = self
                        .input
                        .text
                        .char_indices()
                        .nth(pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input.text.remove(byte_pos);
                    self.input.cursor_pos = pos - 1;
                }
            }
            KeyCode::Enter => {
                if self.async_st.is_loading || self.input.text.is_empty() {
                    return;
                }
                self.start_async(AsyncTask::Detect {
                    text: self.input.text.clone(),
                });
            }
            _ => {}
        }
    }

    // ── Helpers ─────────────────────────────────────────

    fn screen_supports_search(&self) -> bool {
        matches!(self.screen, Screen::Languages | Screen::Models)
    }

    fn active_text_buf(&mut self) -> &mut String {
        match self.screen {
            Screen::TranslateText | Screen::Detect => &mut self.input.text,
            Screen::TranslateFile => &mut self.input.file,
            _ => &mut self.input.text,
        }
    }

    fn collect_translate_inputs(&self) -> (String, String, String) {
        let text = match self.screen {
            Screen::TranslateText => self.input.text.clone(),
            Screen::TranslateFile => self.input.file.clone(),
            _ => self.input.text.clone(),
        };
        (
            text,
            if self.input.source.is_empty() {
                self.config.translation.default_source_lang.clone()
            } else {
                self.input.source.clone()
            },
            if self.input.target.is_empty() {
                self.config
                    .translation
                    .default_target_langs
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "fr".to_string())
            } else {
                self.input.target.clone()
            },
        )
    }

    fn filtered_languages(&self) -> &[crate::languages::LanguageInfo] {
        &self.lang_filter_cache
    }

    fn lang_picker_filtered(&self) -> &[crate::languages::LanguageInfo] {
        &self.lang_picker.cache
    }

    fn filter_languages(query: &str) -> Vec<crate::languages::LanguageInfo> {
        if query.is_empty() {
            return crate::languages::LANGUAGES.to_vec();
        }
        let q = query.to_lowercase();
        crate::languages::LANGUAGES
            .iter()
            .filter(|(iso, nllb, eng, native)| {
                iso.to_lowercase().contains(&q)
                    || nllb.to_lowercase().contains(&q)
                    || eng.to_lowercase().contains(&q)
                    || native.to_lowercase().contains(&q)
            })
            .copied()
            .collect()
    }

    fn lang_display_name(code: &str) -> String {
        if code.is_empty() {
            return "(auto)".to_string();
        }
        for &(iso, nllb, eng, _) in crate::languages::LANGUAGES {
            if iso == code || nllb == code {
                return format!("{} · {}", code, eng);
            }
        }
        // Unknown code — display as-is
        code.to_string()
    }

    fn lang_code_display<'a>(iso: &'a str, nllb: &'a str) -> &'a str {
        if !iso.is_empty() {
            iso
        } else {
            nllb
        }
    }

    fn start_async(&mut self, task: AsyncTask) {
        let (tx, rx) = tokio::sync::oneshot::channel::<AsyncOutcome>();
        self.async_st.is_loading = true;
        self.async_st.loading_msg = match &task {
            AsyncTask::Translate { .. } => "Translating…".to_string(),
            AsyncTask::TranslateFile { .. } => "Translating file…".to_string(),
            AsyncTask::Detect { .. } => "Detecting…".to_string(),
        };
        self.async_st.rx = Some(rx);
        let config = self.config.clone();
        let service = self.async_st.service.clone();

        self.async_st.handle = Some(tokio::spawn(async move {
            let outcome = match task {
                AsyncTask::Translate {
                    text,
                    source,
                    target,
                } => {
                    let (result, svc) =
                        run_translate_cached(service, config, &text, &source, &target).await;
                    AsyncOutcome::TranslationResult(result, svc)
                }
                AsyncTask::TranslateFile {
                    path,
                    source,
                    target,
                } => {
                    let (result, svc) =
                        run_translate_file_cached(service, config, &path, &source, &target).await;
                    AsyncOutcome::TranslationResult(result, svc)
                }
                AsyncTask::Detect { text } => {
                    let r = match config.mode {
                        TranslationMode::Local => run_detect_local(&config, &text).await,
                        _ => run_detect_ai(&config, &text).await,
                    };
                    AsyncOutcome::TextResult(r)
                }
            };
            let _ = tx.send(outcome);
        }));
    }

    fn handle_paste(&mut self, text: &str) {
        if self.focus == Focus::Menu || self.quitting {
            return;
        }
        match self.screen {
            Screen::TranslateText | Screen::TranslateFile if self.input.active_field == 0 => {
                let pos = self.input.cursor_pos;
                let buf = self.active_text_buf();
                let byte_pos = buf
                    .char_indices()
                    .nth(pos)
                    .map(|(i, _)| i)
                    .unwrap_or(buf.len());
                buf.insert_str(byte_pos, text);
                self.input.cursor_pos = pos + text.chars().count();
            }
            Screen::Detect => {
                let pos = self.input.cursor_pos;
                let byte_pos = self
                    .input
                    .text
                    .char_indices()
                    .nth(pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.input.text.len());
                self.input.text.insert_str(byte_pos, text);
                self.input.cursor_pos = pos + text.chars().count();
            }
            _ => {}
        }
    }

    fn tab_complete_file_path(&mut self) {
        let input = self.input.file.clone();
        let (dir_prefix, file_prefix) = if input.ends_with('/') {
            (input.as_str(), "")
        } else {
            match input.rfind('/') {
                Some(pos) => (&input[..=pos], &input[pos + 1..]),
                None => ("", input.as_str()),
            }
        };
        let read_dir = expand_home(if dir_prefix.is_empty() {
            "."
        } else {
            dir_prefix
        });
        let mut matches: Vec<String> = match std::fs::read_dir(&read_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().into_string().ok()?;
                    if !name.starts_with(file_prefix) {
                        return None;
                    }
                    let is_dir = e.path().is_dir();
                    let mut completed = format!("{}{}", dir_prefix, name);
                    if is_dir {
                        completed.push('/');
                    }
                    Some(completed)
                })
                .collect(),
            Err(_) => return,
        };
        matches.sort();
        match matches.len() {
            0 => {}
            1 => {
                self.input.file = matches.into_iter().next().unwrap();
                self.input.cursor_pos = self.input.file.chars().count();
                self.input.path_completions.clear();
            }
            _ => {
                self.input.path_completions = matches;
                self.input.path_completion_idx = 0;
            }
        }
    }

    fn handle_quit_key(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.should_quit = true,
            KeyCode::Char('n' | 'N') | KeyCode::Esc | KeyCode::Char('q') => self.quitting = false,
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                self.quit_choice = (self.quit_choice + 1) % 2
            }
            KeyCode::Enter => {
                if self.quit_choice == 0 {
                    self.should_quit = true;
                } else {
                    self.quitting = false;
                }
            }
            _ => {}
        }
    }
}

// ── Async runners ───────────────────────────────────────

type SvcArc = std::sync::Arc<crate::translation_service::TranslationService>;

async fn run_translate_cached(
    service: Option<SvcArc>,
    config: Config,
    text: &str,
    source: &str,
    target: &str,
) -> (String, Option<SvcArc>) {
    let svc = match service {
        Some(s) => s,
        None => match crate::translation_service::TranslationService::new(config.clone()).await {
            Ok(s) => std::sync::Arc::new(s),
            Err(e) => return (format!("✗ {}", e), None),
        },
    };
    let ctx = config.translation.context.clone();
    let result = match svc
        .translate_text(text, source, target, ctx.as_deref())
        .await
    {
        Ok(r) => r,
        Err(e) => format!("✗ {}", e),
    };
    (result, Some(svc))
}

async fn run_translate_file_cached(
    service: Option<SvcArc>,
    config: Config,
    path: &str,
    source: &str,
    target: &str,
) -> (String, Option<SvcArc>) {
    let svc = match service {
        Some(s) => s,
        None => match crate::translation_service::TranslationService::new(config.clone()).await {
            Ok(s) => std::sync::Arc::new(s),
            Err(e) => return (format!("✗ {}", e), None),
        },
    };
    let ctx = config.translation.context.clone();
    let result = match crate::cli::translate_file_single(
        &*svc.translator,
        path,
        source,
        target,
        ctx.as_deref(),
    )
    .await
    {
        Ok((0, _)) => "No translatable strings found".to_string(),
        Ok((count, out_path)) => format!("✓ {} strings → {}", count, out_path),
        Err(e) => format!("✗ {}", e),
    };
    (result, Some(svc))
}

async fn run_detect_local(_config: &Config, text: &str) -> String {
    #[cfg(feature = "local")]
    {
        let detector = crate::language_detector::WhatLangDetector::new();
        match detector.detect_language(text).await {
            Ok(info) => info.to_string(),
            Err(e) => format!("✗ {}", e),
        }
    }
    #[cfg(not(feature = "local"))]
    {
        let _ = (_config, text);
        "✗ Local mode requires --features local".to_string()
    }
}

async fn run_detect_ai(config: &Config, text: &str) -> String {
    let detector = crate::language_detector::AiLanguageDetector::new(&config.ai.model);
    match detector.detect_language(text).await {
        Ok(info) => info.to_string(),
        Err(e) => format!("✗ {}", e),
    }
}

// ── Standalone select/confirm (used by manager.rs) ──────

pub fn select(title: &str, items: &[String], header: Option<&str>) -> io::Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }
    let mut terminal = ratatui::init();
    let mut list_state = ListState::default().with_selected(Some(0));
    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let hh = if header.is_some() { 2u16 } else { 0u16 };
            let chunks = Layout::vertical([
                Constraint::Length(hh),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);
            if let Some(hdr) = header {
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        hdr,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))),
                    Rect::new(1, chunks[0].y, area.width.saturating_sub(2), 1),
                );
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "─".repeat(area.width as usize),
                        Style::default().fg(Color::DarkGray),
                    ))),
                    Rect::new(0, chunks[0].y + 1, area.width, 1),
                );
            }
            let list_items: Vec<ListItem> =
                items.iter().map(|i| ListItem::new(i.as_str())).collect();
            frame.render_stateful_widget(
                List::new(list_items)
                    .block(
                        Block::default()
                            .title(title)
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    )
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Green)),
                chunks[1],
                &mut list_state,
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    hint_key("↑↓"),
                    hint_sep("  navigate  "),
                    hint_key("Enter"),
                    hint_sep("  select  "),
                    hint_key("Esc"),
                    hint_sep("  cancel"),
                ]))
                .alignment(Alignment::Center),
                chunks[2],
            );
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = list_state.selected().unwrap_or(0);
                        list_state.select(Some(i.saturating_sub(1)));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = list_state.selected().unwrap_or(0);
                        if i + 1 < items.len() {
                            list_state.select(Some(i + 1));
                        }
                    }
                    KeyCode::Enter => break Ok(list_state.selected()),
                    KeyCode::Esc | KeyCode::Char('q') => break Ok(None),
                    _ => {}
                }
            }
        }
    };
    ratatui::restore();
    result
}

pub fn confirm(title: &str, message: &str) -> io::Result<bool> {
    let mut terminal = ratatui::init();
    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let v = Layout::vertical([
                Constraint::Percentage(35),
                Constraint::Length(7),
                Constraint::Percentage(35),
            ])
            .split(area);
            let h = Layout::horizontal([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(v[1]);
            frame.render_widget(
                Paragraph::new(format!("{}\n\n  [Y] Yes    [N] No", message))
                    .block(
                        Block::default()
                            .title(title)
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Green)),
                    )
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false }),
                h[1],
            );
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('y' | 'Y') | KeyCode::Enter => break Ok(true),
                    KeyCode::Char('n' | 'N') | KeyCode::Esc | KeyCode::Char('q') => {
                        break Ok(false);
                    }
                    _ => {}
                }
            }
        }
    };
    ratatui::restore();
    result
}

// ── UI helpers ───────────────────────────────────────────

fn styled_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(title, Style::default().fg(Color::DarkGray)))
}

fn field_style(is_active: bool, has_focus: bool) -> Style {
    if is_active && has_focus {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn hint_key(k: &str) -> Span<'_> {
    Span::styled(
        k,
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )
}

fn hint_sep(s: &'static str) -> Span<'static> {
    Span::styled(s, Style::default().fg(Color::DarkGray))
}

fn expand_home(path: &str) -> std::path::PathBuf {
    if path == "~" || path == "~/" {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home);
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    std::path::PathBuf::from(path)
}
