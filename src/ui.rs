use base64::{engine::general_purpose::STANDARD as B64, Engine};
use iced::{
    event, keyboard, time,
    widget::{column, container, image as iced_image, row, scrollable, text, text_input, Column},
    Alignment, Background, Border, Color, Element, Event, Length, Size, Subscription, Task, Theme,
};
use std::time::Duration;
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::db::{ClipEntry, ClipKind, Db};

// ─── Clipboard helper ─────────────────────────────────────────────────────────
// On Wayland, arboard loses content when the window closes because the app
// must stay alive to "serve" the clipboard. wl-copy solves this by forking
// a background process that keeps the content alive. xclip is the X11 fallback.

fn copy_text(text: &str) {
    // 1. wl-copy (Wayland — background process keeps clipboard alive after exit)
    if std::process::Command::new("wl-copy")
        .arg(text)
        .spawn()
        .is_ok()
    {
        return;
    }
    // 2. xclip (X11 fallback)
    if let Ok(mut child) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        return;
    }
    // 3. arboard last resort (works on X11; may lose content on Wayland at exit)
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

// ─── Palette ──────────────────────────────────────────────────────────────────

const BG: Color = Color::from_rgb(0.10, 0.10, 0.11);
const SURFACE: Color = Color::from_rgb(0.14, 0.14, 0.16);
const SELECTED: Color = Color::from_rgb(0.17, 0.33, 0.55);
const BORDER: Color = Color::from_rgb(0.22, 0.22, 0.24);
const TEXT_DIM: Color = Color::from_rgb(0.42, 0.42, 0.46);
const TEXT_MAIN: Color = Color::from_rgb(0.88, 0.88, 0.90);
const ACCENT: Color = Color::from_rgb(0.28, 0.56, 1.0);

// ─── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Vec<ClipEntry>),
    SearchChanged(String),
    SelectNext,
    SelectPrev,
    CopySelected,
    DeleteSelected,
    PinSelected,
    Close,
    Tick,
    EventOccurred(Event),
}

// ─── State ────────────────────────────────────────────────────────────────────

pub struct ClipApp {
    pub db: Arc<Mutex<Db>>,
    pub entries: Vec<ClipEntry>,
    pub search: String,
    pub selected: usize,
}

impl ClipApp {
    pub fn new(db: Arc<Mutex<Db>>) -> (Self, Task<Msg>) {
        let app = ClipApp {
            db: db.clone(),
            entries: vec![],
            search: String::new(),
            selected: 0,
        };
        (app, Task::perform(load_entries(db), Msg::Loaded))
    }

    fn filtered(&self) -> Vec<&ClipEntry> {
        let q = self.search.to_lowercase();
        self.entries
            .iter()
            .filter(|e| q.is_empty() || e.preview.to_lowercase().contains(&q))
            .collect()
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Loaded(entries) => {
                self.entries = entries;
                // Clamp selection so it stays valid after new items arrive
                let max = self.filtered().len().saturating_sub(1);
                self.selected = self.selected.min(max);
                Task::none()
            }
            Msg::SearchChanged(s) => {
                self.search = s;
                self.selected = 0;
                Task::none()
            }
            Msg::SelectNext => {
                let max = self.filtered().len().saturating_sub(1);
                self.selected = (self.selected + 1).min(max);
                Task::none()
            }
            Msg::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
                Task::none()
            }
            Msg::CopySelected => {
                if let Some(entry) = self.filtered().get(self.selected).copied() {
                    if entry.kind == ClipKind::Text {
                        copy_text(&entry.content);
                    }
                }
                iced::exit()
            }
            Msg::DeleteSelected => {
                if let Some(entry) = self.filtered().get(self.selected).copied() {
                    let id = entry.id;
                    if let Ok(db) = self.db.lock() {
                        let _ = db.delete(id);
                    }
                }
                Task::perform(load_entries(self.db.clone()), Msg::Loaded)
            }
            Msg::PinSelected => {
                if let Some(entry) = self.filtered().get(self.selected).copied() {
                    let id = entry.id;
                    if let Ok(db) = self.db.lock() {
                        let _ = db.toggle_pin(id);
                    }
                }
                Task::perform(load_entries(self.db.clone()), Msg::Loaded)
            }
            Msg::Close => iced::exit(),
            Msg::Tick => {
                return Task::perform(load_entries(self.db.clone()), Msg::Loaded);
            }
            Msg::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed { key, .. })) => {
                match key {
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                        self.update(Msg::SelectNext)
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                        self.update(Msg::SelectPrev)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Enter) => {
                        self.update(Msg::CopySelected)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Delete) => {
                        self.update(Msg::DeleteSelected)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        self.update(Msg::Close)
                    }
                    keyboard::Key::Character(c) if c.as_str() == "p" => {
                        self.update(Msg::PinSelected)
                    }
                    _ => Task::none(),
                }
            }
            Msg::EventOccurred(_) => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        Subscription::batch([
            event::listen().map(Msg::EventOccurred),
            time::every(Duration::from_millis(500)).map(|_| Msg::Tick),
        ])
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let filtered = self.filtered();

        // Search bar
        let search = container(
            text_input("Search clipboard…", &self.search)
                .on_input(Msg::SearchChanged)
                .padding([9, 12])
                .size(14)
                .style(|_theme, _status| iced::widget::text_input::Style {
                    background: Background::Color(SURFACE),
                    border: Border {
                        color: BORDER,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    icon: TEXT_DIM,
                    placeholder: TEXT_DIM,
                    value: TEXT_MAIN,
                    selection: SELECTED,
                }),
        )
        .padding(iced::Padding { top: 10.0, right: 12.0, bottom: 6.0, left: 12.0 });

        // Item list
        let items: Column<Msg> =
            filtered
                .iter()
                .enumerate()
                .fold(Column::new().spacing(2).padding(iced::Padding { top: 0.0, right: 8.0, bottom: 8.0, left: 8.0 }), |col, (i, entry)| {
                    col.push(clip_row(entry, i == self.selected))
                });

        // Hint bar
        let hints = container(
            row![
                hint("↑↓", "navigate"),
                hint("↵", "copy"),
                hint("Del", "delete"),
                hint("P", "pin"),
                hint("Esc", "close"),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .padding([5, 12])
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(SURFACE)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        container(
            column![
                search,
                scrollable(items).height(Length::Fill),
                hints,
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(BG)),
            ..Default::default()
        })
        .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn run(db: Arc<Mutex<Db>>) -> iced::Result {
    iced::application("Clipboard", ClipApp::update, ClipApp::view)
        .theme(ClipApp::theme)
        .subscription(ClipApp::subscription)
        .window(iced::window::Settings {
            size: Size::new(440.0, 520.0),
            resizable: false,
            decorations: true,
            ..Default::default()
        })
        .run_with(move || ClipApp::new(db.clone()))
}

// ─── Clip row ─────────────────────────────────────────────────────────────────

fn clip_row(entry: &ClipEntry, selected: bool) -> Element<'_, Msg> {
    let preview: Element<Msg> = match entry.kind {
        ClipKind::Text => text(&entry.preview)
            .size(13)
            .color(TEXT_MAIN)
            .width(Length::Fill)
            .into(),
        ClipKind::Image => match B64.decode(&entry.content) {
            Ok(bytes) => iced_image::Image::new(iced_image::Handle::from_bytes(bytes))
                .width(Length::Fixed(80.0))
                .height(Length::Fixed(48.0))
                .into(),
            Err(_) => text("[image]").size(13).color(TEXT_DIM).into(),
        },
    };

    let pin_dot: Element<Msg> = if entry.pinned {
        text("●").size(8).color(ACCENT).into()
    } else {
        text("").size(8).into()
    };

    let time = text(entry.created_at.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .size(11)
        .color(TEXT_DIM);

    let inner = row![
        pin_dot,
        preview,
        column![time].align_x(Alignment::End),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding([7, 10]);

    container(inner)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if selected { SELECTED } else { SURFACE })),
            border: Border {
                color: if selected { ACCENT } else { Color::TRANSPARENT },
                width: if selected { 1.0 } else { 0.0 },
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ─── Hint widget ──────────────────────────────────────────────────────────────

fn hint<'a>(key: &'a str, label: &'a str) -> Element<'a, Msg> {
    row![
        container(text(key).size(11).color(TEXT_MAIN))
            .style(|_theme| container::Style {
                background: Some(Background::Color(BORDER)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding([1, 5]),
        text(label).size(11).color(TEXT_DIM),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

// ─── Async helpers ────────────────────────────────────────────────────────────

async fn load_entries(db: Arc<Mutex<Db>>) -> Vec<ClipEntry> {
    tokio::task::spawn_blocking(move || {
        db.lock()
            .ok()
            .and_then(|db| db.all().ok())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}
