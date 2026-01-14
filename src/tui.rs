//! Provides interactive TUI

use clap::ValueEnum;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{self, Block, Borders, HighlightSpacing, Paragraph, Wrap},
};
use std::{collections::HashMap, sync::Arc};
use std::{str::FromStr, thread};
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const COLOR_DIM: Color = Color::DarkGray;
const COLOR_FOCUS: Color = Color::LightBlue;
const MAX_HISTORY_SIZE: usize = 100;
const SELECTION_PREFIX: &str = "> ";

/// Displays an interactive selection UI with search and filtering.
///
/// The `fetch` function receives a page number and fetch options.
/// Users can search with `@key=value` fetch options or plain text queries.
///
/// # Errors
///
/// Returns an error if the selection was cancelled or the fetch fails.
pub fn select_item_with<T, F>(fetch: F) -> anyhow::Result<T>
where
    T: ListableItem,
    F: Fn(u32, &FetchOptions) -> anyhow::Result<FetchResult<T>> + Send + Sync + 'static,
{
    let mut app = App::new(fetch);
    let selected_index = ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| app.render(frame))?;
            app.update()?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key_event) = event::read()?
            {
                match app.handle_key_event(key_event) {
                    UserAction::None => {}
                    UserAction::Quit => anyhow::bail!("Selection aborted"),
                    UserAction::Select(index) => return Ok(index),
                }
            }
        }
    })?;

    app.into_item(selected_index)
        .ok_or_else(|| anyhow::anyhow!("Invalid selection"))
}

/// Items that can be displayed in the selection UI.
pub trait ListableItem: Clone + Send + 'static {
    /// Returns the display text for this item.
    fn get_display_text(&self) -> String;
}

/// Options to configure the fetch function.
#[derive(Clone, Default)]
pub struct FetchOptions(HashMap<String, String>);

impl FetchOptions {
    /// Parses a simple value from the options map.
    pub fn parse<T: FromStr>(&self, key: &str) -> Option<T> {
        self.0.get(key).and_then(|v| v.parse::<T>().ok())
    }

    /// Parses a clap ValueEnum value from the options map.
    pub fn parse_enum<T: ValueEnum>(&self, key: &str) -> Option<T> {
        self.0.get(key).and_then(|s| T::from_str(s, true).ok())
    }

    /// Parses a comma-separated list from the options map.
    pub fn parse_list<T: FromStr>(&self, key: &str) -> Option<Vec<T>> {
        self.0.get(key).and_then(|list| {
            list.split(',')
                .map(T::from_str)
                .collect::<Result<Vec<_>, _>>()
                .ok()
        })
    }

    /// Parses a str value from the options map.
    pub fn parse_str<'a>(&'a self, key: &str) -> Option<&'a str> {
        self.0.get(key).map(|s| s.as_str())
    }

    fn new() -> Self {
        FetchOptions(HashMap::default())
    }

    fn as_hash_map(&self) -> &HashMap<String, String> {
        &self.0
    }

    fn as_hash_map_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.0
    }
}

/// The response returned by the fetch function.
pub struct FetchResult<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Whether more items are available.
    pub has_more: bool,
}

#[derive(Default)]
enum FetchStatus<T> {
    #[default]
    Idle,
    Fetching(Receiver<anyhow::Result<ParsedFetchResult<T>>>),
}

struct ParsedFetchResult<T> {
    append_items: bool,
    items: Vec<T>,
    has_more: bool,
    page: u32,
}

type FetchFn<T> = Arc<dyn Fn(u32, &FetchOptions) -> anyhow::Result<FetchResult<T>> + Send + Sync>;

struct ItemFetcher<T> {
    fetch: FetchFn<T>,
    status: FetchStatus<T>,
    options: FetchOptions,
}

impl<T: ListableItem> ItemFetcher<T> {
    fn new<F>(fetch: F) -> Self
    where
        F: Fn(u32, &FetchOptions) -> anyhow::Result<FetchResult<T>> + Send + Sync + 'static,
    {
        Self {
            status: FetchStatus::default(),
            options: FetchOptions::default(),
            fetch: Arc::new(fetch),
        }
    }

    fn fetch(&mut self, filters: FetchOptions, page: u32, append_items: bool) {
        self.options = filters.clone();

        let (tx, rx) = mpsc::channel();
        let fetch = Arc::clone(&self.fetch);

        thread::spawn(move || {
            let fetch_result = fetch(page, &filters).map(|result| ParsedFetchResult {
                page,
                items: result.items,
                append_items,
                has_more: result.has_more,
            });

            // Ignore send errors - e.g. receiver may have been dropped if user
            // started a new search... which we don't care about.
            tx.send(fetch_result).ok();
        });

        self.status = FetchStatus::Fetching(rx);
    }

    fn is_fetching(&self) -> bool {
        matches!(self.status, FetchStatus::Fetching { .. })
    }

    fn poll_result(&mut self) -> Option<anyhow::Result<ParsedFetchResult<T>>> {
        if let FetchStatus::Fetching(rx) = &self.status
            && let Ok(result) = rx.try_recv()
        {
            self.status = FetchStatus::default();

            Some(result)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.status = FetchStatus::default();
    }
}

#[derive(Default)]
struct ListState<T> {
    items: Vec<T>,
    state: widgets::ListState,
}

impl<T> ListState<T> {
    fn new() -> Self {
        Self {
            items: vec![],
            state: widgets::ListState::default(),
        }
    }

    fn get_state(&mut self) -> &mut widgets::ListState {
        &mut self.state
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn items(&self) -> &[T] {
        &self.items
    }

    fn push_items(&mut self, new_items: Vec<T>) {
        self.items.extend(new_items);
    }

    fn set_items(&mut self, new_items: Vec<T>) {
        self.items = new_items;
        self.state
            .select(if self.items.is_empty() { None } else { Some(0) });
    }

    fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    fn select_next(&mut self) {
        self.state.select_next();
    }

    fn select_previous(&mut self) {
        self.state.select_previous();
    }

    fn select_page_up(&mut self, page_size: u16) {
        if page_size == 0 || self.items.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        let jump = (page_size.saturating_sub(1)) as usize;
        let new_index = current.saturating_sub(jump);

        self.state.select(Some(new_index));
    }

    fn select_page_down(&mut self, page_size: u16) {
        if page_size == 0 || self.items.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        let jump = (page_size.saturating_sub(1)) as usize;
        let new_index = current.saturating_add(jump);

        self.state.select(Some(new_index));
    }
}

#[derive(Default)]
struct SearchState {
    cursor_pos: usize,
    query: String,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl SearchState {
    fn clear(&mut self) {
        self.exit_history_browsing();
        self.query.clear();
        self.cursor_pos = 0;
    }

    fn grapheme_count(&self) -> usize {
        self.query.graphemes(true).count()
    }

    fn grapheme_index_to_byte_index(&self, grapheme_idx: usize) -> usize {
        self.query
            .grapheme_indices(true)
            .nth(grapheme_idx)
            .map(|(idx, _)| idx)
            .unwrap_or(self.query.len())
    }

    fn has_query(&self) -> bool {
        !self.query.is_empty()
    }

    fn display_width_up_to_cursor(&self) -> usize {
        let byte_index = self.grapheme_index_to_byte_index(self.cursor_pos);

        UnicodeWidthStr::width(&self.query[..byte_index])
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            let byte_start = self.grapheme_index_to_byte_index(self.cursor_pos - 1);
            let byte_end = self.grapheme_index_to_byte_index(self.cursor_pos);

            self.query.replace_range(byte_start..byte_end, "");
            self.cursor_pos -= 1;
        }
    }

    fn delete_char_after_cursor(&mut self) {
        if self.cursor_pos < self.grapheme_count() {
            let byte_start = self.grapheme_index_to_byte_index(self.cursor_pos);
            let byte_end = self.grapheme_index_to_byte_index(self.cursor_pos + 1);

            self.query.replace_range(byte_start..byte_end, "");
        }
    }

    fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }

        let byte_cursor = self.grapheme_index_to_byte_index(self.cursor_pos);
        let before_cursor = &self.query[..byte_cursor];
        let trimmed = before_cursor.trim_end();

        if let Some(pos) = trimmed.rfind(|char: char| char.is_whitespace()) {
            let byte_start = pos + 1;

            self.query.replace_range(byte_start..byte_cursor, "");
            self.cursor_pos = self.query[..byte_start].graphemes(true).count();
        } else {
            self.query.replace_range(..byte_cursor, "");
            self.cursor_pos = 0;
        }
    }

    fn delete_word_after_cursor(&mut self) {
        if self.cursor_pos >= self.grapheme_count() {
            return;
        }

        let byte_cursor = self.grapheme_index_to_byte_index(self.cursor_pos);
        let after_cursor = &self.query[byte_cursor..];
        let trimmed = after_cursor.trim_start();
        let trimmed_offset = after_cursor.len() - trimmed.len();

        if let Some(pos) = trimmed.find(|c: char| c.is_whitespace()) {
            let byte_end = byte_cursor + trimmed_offset + pos;

            self.query.replace_range(byte_cursor..byte_end, "");
        } else {
            self.query.truncate(byte_cursor);
        }
    }

    fn insert_char_at_cursor(&mut self, char: char) {
        let byte_pos = self.grapheme_index_to_byte_index(self.cursor_pos);

        self.query.insert(byte_pos, char);
        self.cursor_pos += 1;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.grapheme_count() {
            self.cursor_pos += 1;
        }
    }

    fn move_cursor_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.grapheme_count();
    }

    fn move_cursor_left_word(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }

        let byte_cursor = self.grapheme_index_to_byte_index(self.cursor_pos);
        let before_cursor = &self.query[..byte_cursor];
        let trimmed = before_cursor.trim_end();

        if trimmed.is_empty() {
            self.cursor_pos = 0;

            return;
        }

        if let Some(pos) = trimmed.rfind(|c: char| c.is_whitespace()) {
            self.cursor_pos = self.query[..pos + 1].graphemes(true).count();
        } else {
            self.cursor_pos = 0;
        }
    }

    fn move_cursor_right_word(&mut self) {
        let grapheme_count = self.grapheme_count();

        if self.cursor_pos >= grapheme_count {
            return;
        }

        let byte_cursor = self.grapheme_index_to_byte_index(self.cursor_pos);
        let after_cursor = &self.query[byte_cursor..];
        let trimmed = after_cursor.trim_start_matches(|c: char| !c.is_whitespace());

        if trimmed.is_empty() {
            self.cursor_pos = grapheme_count;

            return;
        }

        let trimmed = trimmed.trim_start_matches(|c: char| c.is_whitespace());
        let bytes_skipped = after_cursor.len() - trimmed.len();
        let byte_pos = byte_cursor + bytes_skipped;

        self.cursor_pos = self.query[..byte_pos].graphemes(true).count();
    }

    fn navigate_history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => self.history_index = Some(self.history.len() - 1),
            Some(index) if index > 0 => self.history_index = Some(index - 1),
            Some(_) => return,
        }

        if let Some(index) = self.history_index {
            self.query = self.history[index].clone();
            self.cursor_pos = self.grapheme_count();
        }
    }

    fn navigate_history_down(&mut self) {
        match self.history_index {
            None => {}
            Some(index) if index < self.history.len() - 1 => {
                self.history_index = Some(index + 1);
                self.query = self.history[index + 1].clone();
                self.cursor_pos = self.grapheme_count();
            }
            Some(_) => {
                self.history_index = None;
                self.query.clear();
                self.cursor_pos = 0;
            }
        }
    }

    fn save_to_history(&mut self) {
        if self.query.is_empty() {
            return;
        }

        if let Some(pos) = self.history.iter().position(|s| s == &self.query) {
            self.history.remove(pos);
        }

        self.history.push(self.query.clone());

        if self.history.len() > MAX_HISTORY_SIZE {
            self.history.remove(0);
        }
    }

    fn exit_history_browsing(&mut self) {
        self.history_index = None;
    }
}

struct PaginationState {
    current_page: u32,
    /// The number used for scrolling with PageUp/Down.
    ///
    /// We init this with 0 because we have no better value to use at this
    /// moment. It actually needs to be set from the outside because it depends
    /// on the terminal height.
    page_size: u16,
    /// API has more items to fetch.
    has_more: bool,
}

impl PaginationState {
    fn reset(&mut self) {
        self.current_page = 0;
        self.has_more = true;
    }
}

impl Default for PaginationState {
    fn default() -> Self {
        PaginationState {
            has_more: true, // default to true for initial fetch
            current_page: Default::default(),
            page_size: Default::default(),
        }
    }
}

#[derive(Default, PartialEq)]
enum Focus {
    #[default]
    List,
    SearchBar,
}

enum UserAction {
    None,
    Quit,
    Select(usize),
}

struct App<T: ListableItem> {
    focus: Focus,
    item_fetcher: ItemFetcher<T>,
    list: ListState<T>,
    pagination: PaginationState,
    search: SearchState,
}

impl<T: ListableItem> App<T> {
    fn new<F>(fetch: F) -> Self
    where
        F: Fn(u32, &FetchOptions) -> anyhow::Result<FetchResult<T>> + Send + Sync + 'static,
    {
        Self {
            focus: Focus::default(),
            item_fetcher: ItemFetcher::new(fetch),
            list: ListState::new(),
            pagination: PaginationState::default(),
            search: SearchState::default(),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> UserAction {
        let KeyEvent {
            code,
            modifiers,
            kind,
            ..
        } = key_event;

        if kind != KeyEventKind::Press {
            return UserAction::None;
        }

        match self.focus {
            Focus::List => self.handle_key_event_list_widget(code, modifiers),
            Focus::SearchBar => self.handle_key_event_search_bar_widget(code, modifiers),
        }
    }

    fn into_item(self, selected_index: usize) -> Option<T> {
        self.list.items.into_iter().nth(selected_index)
    }

    fn render(&mut self, frame: &mut Frame) {
        let rects = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

        self.render_list(frame, rects[0]);
        self.render_search_bar(frame, rects[1]);
        self.render_info_bar(frame, rects[2]);
    }

    fn update(&mut self) -> anyhow::Result<()> {
        // We should initialize a fetch on initial draw or if the user reached
        // the end of the currently shown items.
        let should_initialize_fetching = {
            if self.item_fetcher.is_fetching()
                || !self.pagination.has_more
                || matches!(self.focus, Focus::SearchBar)
            {
                false
            } else if let Some(selected) = self.list.selected_index() {
                const THRESHOLD: usize = 3;

                self.list.items.len().saturating_sub(selected + 1) < THRESHOLD
            } else {
                true
            }
        };

        if should_initialize_fetching {
            self.fetch_items(self.item_fetcher.options.clone(), true);
        }

        if let Some(fetch_result) = self.item_fetcher.poll_result() {
            let fetch_result = fetch_result?;

            if fetch_result.append_items {
                self.list.push_items(fetch_result.items);
            } else {
                self.list.set_items(fetch_result.items);
            }

            self.pagination.current_page = fetch_result.page;
            self.pagination.has_more = fetch_result.has_more;

            if self.list.selected_index().is_none() {
                self.list.select_next();
            }
        };

        Ok(())
    }

    fn fetch_items(&mut self, filters: FetchOptions, append_items: bool) {
        if append_items {
            self.item_fetcher
                .fetch(filters, self.pagination.current_page + 1, true);
        } else {
            // Reset any in-flight fetch, e.g. when starting searches back to
            // back quickly
            self.item_fetcher.reset();
            self.pagination.reset();

            self.item_fetcher.fetch(filters, 1, false);
        }
    }

    fn handle_key_event_list_widget(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> UserAction {
        match code {
            KeyCode::Esc => UserAction::Quit,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => UserAction::Quit,
            KeyCode::Char(char) => {
                self.focus = Focus::SearchBar;

                self.search.insert_char_at_cursor(char);

                UserAction::None
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = Focus::SearchBar;

                UserAction::None
            }
            KeyCode::Up => {
                self.list.select_previous();

                UserAction::None
            }
            KeyCode::Down => {
                self.list.select_next();

                UserAction::None
            }
            KeyCode::PageUp => {
                self.list.select_page_up(self.pagination.page_size);

                UserAction::None
            }
            KeyCode::PageDown => {
                self.list.select_page_down(self.pagination.page_size);

                UserAction::None
            }
            KeyCode::Enter => self
                .list
                .selected_index()
                .map(UserAction::Select)
                .unwrap_or(UserAction::None),
            _ => UserAction::None,
        }
    }

    fn handle_key_event_search_bar_widget(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> UserAction {
        match code {
            KeyCode::Esc => {
                if self.search.has_query() {
                    self.search.clear();

                    UserAction::None
                } else {
                    UserAction::Quit
                }
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => UserAction::Quit,
            KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.move_cursor_to_start();

                UserAction::None
            }
            KeyCode::Char('e') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.move_cursor_to_end();

                UserAction::None
            }
            KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.clear();

                UserAction::None
            }
            KeyCode::Char(char) => {
                self.search.exit_history_browsing();
                self.search.insert_char_at_cursor(char);

                UserAction::None
            }
            KeyCode::Backspace => {
                self.search.exit_history_browsing();

                if modifiers.contains(KeyModifiers::ALT) {
                    self.search.delete_word_before_cursor();
                } else {
                    self.search.delete_char_before_cursor();
                }

                UserAction::None
            }
            KeyCode::Delete => {
                self.search.exit_history_browsing();

                if modifiers.contains(KeyModifiers::ALT) {
                    self.search.delete_word_after_cursor();
                } else {
                    self.search.delete_char_after_cursor();
                }

                UserAction::None
            }
            KeyCode::Left => {
                if modifiers.contains(KeyModifiers::ALT)
                    || modifiers.contains(KeyModifiers::CONTROL)
                {
                    self.search.move_cursor_left_word();
                } else {
                    self.search.move_cursor_left();
                }

                UserAction::None
            }
            KeyCode::Right => {
                if modifiers.contains(KeyModifiers::ALT)
                    || modifiers.contains(KeyModifiers::CONTROL)
                {
                    self.search.move_cursor_right_word();
                } else {
                    self.search.move_cursor_right();
                }

                UserAction::None
            }
            KeyCode::Up => {
                self.search.navigate_history_up();

                UserAction::None
            }
            KeyCode::Down => {
                self.search.navigate_history_down();

                UserAction::None
            }
            KeyCode::Home => {
                self.search.move_cursor_to_start();

                UserAction::None
            }
            KeyCode::End => {
                self.search.move_cursor_to_end();

                UserAction::None
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = Focus::List;

                UserAction::None
            }
            KeyCode::Enter => {
                let filters = parse_filters(&self.search.query);

                self.search.save_to_history();
                self.search.clear();
                self.focus = Focus::List;

                self.fetch_items(filters, false);

                UserAction::None
            }
            _ => UserAction::None,
        }
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        self.pagination.page_size = area.height;

        let list = if self.list.is_empty() {
            let message = if self.item_fetcher.is_fetching() {
                "  Loading items..."
            } else {
                "  No items found"
            };

            widgets::List::new(vec![
                widgets::ListItem::new(message).style(Style::new().fg(COLOR_DIM)),
            ])
            .block(Block::new())
        } else {
            let mut list_items: Vec<widgets::ListItem> = self
                .list
                .items()
                .iter()
                .map(|item| widgets::ListItem::new(item.get_display_text()))
                .collect();
            let item_count = list_items.len();
            let max_item_count = area.height as usize;

            if self.pagination.has_more {
                for _ in item_count..max_item_count {
                    list_items.push(widgets::ListItem::new("·").style(Style::new().fg(COLOR_DIM)));
                }
            }

            let mut widget = widgets::List::new(list_items)
                .block(Block::new())
                .highlight_symbol(SELECTION_PREFIX)
                .highlight_spacing(HighlightSpacing::Always);

            if self.focus == Focus::List {
                widget = widget.highlight_style(Style::new().fg(COLOR_FOCUS).bold());
            }

            widget
        };

        frame.render_stateful_widget(list, area, self.list.get_state());
    }

    fn render_search_bar(&self, frame: &mut Frame, area: Rect) {
        let prefix = "> ";
        let focus_color = if self.focus == Focus::SearchBar {
            COLOR_FOCUS
        } else {
            COLOR_DIM
        };
        let search_box = Paragraph::new(Line::from(vec![
            Span::styled(prefix, Style::new().fg(focus_color)),
            Span::raw(&self.search.query),
        ]))
        .block(
            Block::new()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::new().fg(focus_color)),
        );

        frame.render_widget(search_box, area);

        if self.focus == Focus::SearchBar {
            let cursor_x = area
                .x
                .saturating_add(prefix.len() as u16)
                .saturating_add(self.search.display_width_up_to_cursor() as u16);
            // Move one line down, from the border to the input line
            let cursor_y = area.y + 1;

            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn render_info_bar(&self, frame: &mut Frame, area: Rect) {
        let options = self.item_fetcher.options.as_hash_map();
        let status_text = if self.item_fetcher.is_fetching() {
            String::from("  Loading items...")
        } else if !options.is_empty() {
            let mut status = String::new();

            for (key, value) in options {
                if key != "query" {
                    status.push_str(&format!("@{key}={value} "));
                }
            }

            if let Some(query) = options.get("query") {
                status.push_str(query);
            }

            status
        } else {
            String::new()
        };

        let nav_text = match self.focus {
            Focus::List => "↑↓: Navigate | Enter: Select | Esc: Quit | Tab: Switch Focus",
            Focus::SearchBar => {
                if self.search.has_query() {
                    "Enter: Search | Esc: Clear Search | Tab: Switch Focus"
                } else {
                    "Enter: Search | Esc: Quit | Tab: Switch Focus"
                }
            }
        };

        let areas = Layout::horizontal([
            Constraint::Min(status_text.len().saturating_add(5) as u16),
            Constraint::Percentage(100),
        ])
        .split(area);

        let status_bar = Paragraph::new(status_text)
            .block(Block::new().style(Style::new().fg(COLOR_DIM)))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

        let nav_bar = Paragraph::new(nav_text)
            .block(Block::new().style(Style::new().fg(COLOR_DIM)))
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false });

        frame.render_widget(status_bar, areas[0]);
        frame.render_widget(nav_bar, areas[1]);
    }
}

fn parse_filters(query: &str) -> FetchOptions {
    let mut options = FetchOptions::new();
    let mut remaining_text = String::new();

    for word in query.split_whitespace() {
        if let Some(filter_str) = word.strip_prefix('@')
            && let Some((key, value)) = filter_str.split_once('=')
        {
            options
                .as_hash_map_mut()
                .insert(String::from(key), String::from(value));

            continue;
        }

        if !remaining_text.is_empty() {
            remaining_text.push(' ');
        }

        remaining_text.push_str(word);
    }

    if !remaining_text.is_empty() {
        options
            .as_hash_map_mut()
            .insert(String::from("query"), remaining_text);
    }

    options
}
