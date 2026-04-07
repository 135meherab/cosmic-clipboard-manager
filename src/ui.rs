use base64::{engine::general_purpose::STANDARD as B64, Engine};
use iced::{
    widget::{
        button, column, container, row, scrollable, text, text_input, Column,
        image as iced_image,
    },
    Alignment, Application, Color, Element, Length, Settings, Size, Task, Theme,
};
use std::sync::{Arc, Mutex};

use crate::db::{ClipEntry, ClipKind, Db};

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run(db: Arc<Mutex<Db>>) -> iced::Result {
    ClipApp::run(Settings {
        window: iced::window::Settings {
            size: Size::new(520.0, 600.0),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        flags: db,
        ..Default::default()
    })
}

// ─── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Vec<ClipEntry>),
    SearchChanged(String),
    CopyItem(i64),
    TogglePin(i64),
    DeleteItem(i64),
    ClearHistory,
    Refresh,
    Noop,
}

// ─── Application state ────────────────────────────────────────────────────────

struct ClipApp {
    db: Arc<Mutex<Db>>,
    entries: Vec<ClipEntry>,
    search: String,
}

impl Application for ClipApp {
    type Executor = iced::executor::Default;
    type Message = Msg;
    type Theme = Theme;
    type Flags = Arc<Mutex<Db>>;

    fn new(db: Self::Flags) -> (Self, Task<Msg>) {
        let app = ClipApp {
            db: db.clone(),
            entries: vec![],
            search: String::new(),
        };
        (app, Task::perform(load_entries(db), Msg::Loaded))
    }

    fn title(&self) -> String {
        "Clipboard Manager".into()
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Loaded(entries) => {
                self.entries = entries;
                Task::none()
            }
            Msg::SearchChanged(s) => {
                self.search = s;
                Task::none()
            }
            Msg::CopyItem(id) => {
                if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
                    if entry.kind == ClipKind::Text {
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            let _ = cb.set_text(entry.content.clone());
                        }
                    }
                }
                Task::none()
            }
            Msg::TogglePin(id) => {
                if let Ok(db) = self.db.lock() {
                    let _ = db.toggle_pin(id);
                }
                Task::perform(load_entries(self.db.clone()), Msg::Loaded)
            }
            Msg::DeleteItem(id) => {
                if let Ok(db) = self.db.lock() {
                    let _ = db.delete(id);
                }
                Task::perform(load_entries(self.db.clone()), Msg::Loaded)
            }
            Msg::ClearHistory => {
                if let Ok(db) = self.db.lock() {
                    let _ = db.clear_unpinned();
                }
                Task::perform(load_entries(self.db.clone()), Msg::Loaded)
            }
            Msg::Refresh => Task::perform(load_entries(self.db.clone()), Msg::Loaded),
            Msg::Noop => Task::none(),
        }
    }

    fn view(&self) -> Element<Msg> {
        let search_bar = text_input("Search clips…", &self.search)
            .on_input(Msg::SearchChanged)
            .padding(10);

        let query = self.search.to_lowercase();
        let filtered: Vec<&ClipEntry> = self
            .entries
            .iter()
            .filter(|e| query.is_empty() || e.preview.to_lowercase().contains(&query))
            .collect();

        let items: Column<Msg> = filtered
            .iter()
            .fold(Column::new().spacing(6).padding(8), |col, entry| {
                col.push(entry_card(entry))
            });

        let content = column![
            // Header
            container(
                row![
                    text("Clipboard Manager").size(20).width(Length::Fill),
                    button("Clear All")
                        .on_press(Msg::ClearHistory)
                        .style(iced::theme::Button::Destructive),
                ]
                .align_y(Alignment::Center)
                .padding(12)
                .spacing(8)
            )
            .style(iced::theme::Container::Box),
            // Search
            container(search_bar).padding([8, 12]),
            // Clip list
            scrollable(items).height(Length::Fill),
            // Footer
            container(
                text(format!("{} item(s)", filtered.len()))
                    .size(12)
                    .color(Color::from_rgb(0.5, 0.5, 0.5))
            )
            .padding([4, 12]),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// ─── Entry card widget ────────────────────────────────────────────────────────

fn entry_card(entry: &ClipEntry) -> Element<Msg> {
    let pin_label = if entry.pinned { "Unpin" } else { "Pin" };
    let time_str = entry.created_at.format("%H:%M").to_string();

    let content_preview: Element<Msg> = match entry.kind {
        ClipKind::Text => text(&entry.preview).size(14).width(Length::Fill).into(),
        ClipKind::Image => match B64.decode(&entry.content) {
            Ok(bytes) => {
                let handle = iced_image::Handle::from_bytes(bytes);
                iced_image::Image::new(handle)
                    .width(Length::Fixed(120.0))
                    .height(Length::Fixed(80.0))
                    .into()
            }
            Err(_) => text("[Invalid image]").size(14).into(),
        },
    };

    let id = entry.id;
    let copy_btn = if entry.kind == ClipKind::Text {
        button("Copy")
            .on_press(Msg::CopyItem(id))
            .style(iced::theme::Button::Primary)
    } else {
        button("Copy").style(iced::theme::Button::Secondary)
    };

    let pin_style = if entry.pinned {
        iced::theme::Button::Positive
    } else {
        iced::theme::Button::Secondary
    };

    container(
        row![
            column![
                content_preview,
                text(&time_str)
                    .size(11)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                copy_btn,
                button(pin_label)
                    .on_press(Msg::TogglePin(id))
                    .style(pin_style),
                button("✕")
                    .on_press(Msg::DeleteItem(id))
                    .style(iced::theme::Button::Destructive),
            ]
            .spacing(4)
            .align_x(Alignment::End),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding(8),
    )
    .style(iced::theme::Container::Box)
    .width(Length::Fill)
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
