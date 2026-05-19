//! UI rendering — three-tab layout with Explore and MindMap panels.

use tui_tree_widget::Tree;

use ratzilla::ratatui::Frame;
use ratzilla::ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style};
use ratzilla::ratatui::text::{Line, Span, Text};
use ratzilla::ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Tabs, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::{AppState, ColumnFocus, gherkin, markdown, mindmap};

const TAB_NAMES: &[&str] = &[" Explore [1] ", " MindMap [2] ", " AI [3] "];

// ── Color palette matching the desktop TUI ──
// Keywords match the desktop TUI's Color::* intent, using RGB approximates
// that look like a modern dark terminal theme.
const KWD_GIVEN: Color = Color::Rgb(86, 156, 214); // ~Color::Blue (readable blue)
const KWD_WHEN: Color = Color::Rgb(226, 183, 20); // ~Color::Yellow (amber)
const KWD_THEN: Color = Color::Rgb(106, 153, 85); // ~Color::Green (muted)
const KWD_AND_BUT: Color = Color::Rgb(128, 128, 128); //  Color::Gray
const HEADER_CYAN: Color = Color::Rgb(86, 182, 194); // ~Color::Cyan
// Status dots
const STAT_PASSED: Color = Color::Rgb(39, 201, 63); // ~Color::Green (bright)
const STAT_FAILED: Color = Color::Rgb(224, 108, 117); // ~Color::Red (soft)
const STAT_RUNNING: Color = Color::Rgb(226, 183, 20); // ~Color::Yellow
const STAT_IDLE: Color = Color::Rgb(136, 136, 136); //  Color::DarkGray

// Selection / highlight
const SEL_FOCUSED_FG: Color = Color::Rgb(226, 183, 20); // ~Color::Yellow + Bold
const SEL_UNFOCUSED_FG: Color = Color::Rgb(86, 182, 194); // ~Color::Cyan

// Text
const TEXT_MAIN: Color = Color::Rgb(212, 212, 212); // ~Color::White
const TEXT_MUTED: Color = Color::Rgb(136, 136, 136); //  Color::DarkGray
const TEXT_ERROR: Color = Color::Rgb(224, 108, 117); // ~Color::Red

// AI Chat roles
const AI_USER: Color = Color::Rgb(86, 182, 194); // ~Color::Cyan
const AI_ASSISTANT: Color = Color::Rgb(106, 153, 85); // ~Color::Green
const AI_WAITING: Color = Color::Rgb(226, 183, 20); // ~Color::Yellow

pub struct AppUi;

impl AppUi {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, f: &mut Frame, state: &mut AppState) {
        // Fresh slate for clickable regions each frame
        state.clickable_regions.clear();

        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tab bar
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status footer (separator + text combined)
            ])
            .split(area);

        self.render_tabs(f, chunks[0], state);
        self.render_content(f, chunks[1], state);
        self.render_footer(f, chunks[2], state);

        // Help overlay on top
        if state.show_help {
            self.render_help(f, area);
        }
    }

    // ── Help overlay ──

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(Span::styled(
                "Keyboard Shortcuts",
                Style::default()
                    .fg(HEADER_CYAN)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Tab / ← →    Focus columns (Explore)",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  ↑ / ↓         Navigate items",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  Enter         View raw feature / Toggle tree node",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  Space         Toggle tree expand (MindMap)",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  Backspace     Exit raw view",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  1 / 2 / 3     Switch tabs",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  r / R / t      Ping Runner / Reload / Simulate tests",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  ?             Toggle this help",
                Style::default(),
            )),
            Line::from(Span::styled(
                "  q / Esc       Exit to homepage",
                Style::default(),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Mouse clicks work on tabs and explore items.",
                Style::default().fg(TEXT_MUTED),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Press ? or Esc to close",
                Style::default().fg(TEXT_MUTED),
            )),
        ];

        let h = help_text.len() as u16 + 2;
        let w = 50;
        let x = area.width.saturating_sub(w) / 2;
        let y = area.height.saturating_sub(h) / 2;
        let help_area = Rect {
            x,
            y,
            width: w,
            height: h,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        f.render_widget(Clear, area);
        f.render_widget(Paragraph::new(help_text).block(block), help_area);
    }

    // ── Tab bar ──

    fn render_tabs(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        f.render_widget(
            Tabs::new(
                TAB_NAMES
                    .iter()
                    .map(|&n| Line::from(Span::styled(n, Style::default())))
                    .collect::<Vec<_>>(),
            )
            .select(s.active_tab)
            .highlight_style(Style::default().fg(TEXT_MAIN).add_modifier(Modifier::BOLD))
            .style(Style::default().fg(TEXT_MUTED))
            .divider(" "),
            area,
        );
        // Register tab regions for mouse hit-testing
        s.clickable_regions.push(crate::ClickableRegion::Tab(0));
        s.clickable_regions.push(crate::ClickableRegion::Tab(1));
        s.clickable_regions.push(crate::ClickableRegion::Tab(2));
    }

    // ── Content dispatch ──

    fn render_content(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        match s.active_tab {
            0 => self.render_explore(f, area, s),
            1 => self.render_mindmap(f, area, s),
            2 => self.render_ai_tab(f, area, s),
            _ => {}
        }
    }

    // ── Explore tab ──

    fn render_explore(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        if s.show_raw_feature {
            self.render_raw_feature(f, area, s);
            return;
        }
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(45),
                Constraint::Percentage(40),
            ])
            .split(area);
        self.feature_list(f, cols[0], s);
        self.scenario_list(f, cols[1], s);
        self.step_view(f, cols[2], s);
    }

    fn render_raw_feature(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let feat = s.project.features.get(s.raw_feature_index);
        let name = feat
            .map(|f| {
                f.file_path
                    .file_stem()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(format!(" {} (raw) ", name))
            .border_style(Style::default().fg(HEADER_CYAN));

        let content = match feat {
            Some(f) => {
                let mut lines: Vec<String> = Vec::new();
                if !f.tags.is_empty() {
                    lines.push(f.tags.join(" "));
                }
                lines.push(format!("Feature: {}", f.name));
                for desc in &f.description {
                    lines.push(format!("  {}", desc));
                }
                lines.push(String::new());
                if let Some(bg) = &f.background {
                    lines.push("  Background:".into());
                    for s in &bg.steps {
                        lines.push(format!("    {} {}", s.keyword, s.text));
                    }
                    lines.push(String::new());
                }
                for sc in &f.scenarios {
                    if !sc.tags.is_empty() {
                        lines.push(format!("  {}", sc.tags.join(" ")));
                    }
                    let header = match sc.kind {
                        gherkin::ScenarioKind::Scenario => "Scenario:",
                        gherkin::ScenarioKind::ScenarioOutline => "Scenario Outline:",
                    };
                    lines.push(format!("  {} {}", header, sc.name));
                    for s in &sc.steps {
                        lines.push(format!("    {} {}", s.keyword, s.text));
                    }
                    for ex in &sc.examples {
                        lines.push(format!("    Examples:"));
                        if !ex.headers.is_empty() {
                            lines.push(format!("      | {} |", ex.headers.join(" | ")));
                        }
                        for row in &ex.rows {
                            lines.push(format!("      | {} |", row.join(" | ")));
                        }
                    }
                    lines.push(String::new());
                }
                lines.join("\n")
            }
            None => "Feature not found.".to_string(),
        };

        let inner = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(inner, area);
    }

    /// Title style for explore column blocks (desktop: focused=Yellow+Bold, unfocused=DarkGray)
    fn block_title_style(&self, focused: bool) -> Style {
        if focused {
            Style::default()
                .fg(SEL_FOCUSED_FG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_MUTED)
        }
    }

    /// Selection style for list items (desktop: focused=Yellow+Bold fg, unfocused=Cyan fg, no bg)
    fn selected_style(&self, focused: bool) -> Style {
        if focused {
            Style::default()
                .fg(SEL_FOCUSED_FG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SEL_UNFOCUSED_FG)
        }
    }

    fn feature_list(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Features ")
            .title_style(self.block_title_style(s.explore_focus == ColumnFocus::Feature));
        let inner = b.inner(area);
        let mut lines: Vec<Line> = Vec::new();
        let col_width = inner.width;

        let line_rows = |content: &str| -> u16 {
            if col_width < 2 {
                return 1;
            }
            let w = content.len();
            if w == 0 {
                return 1;
            }
            ((w + col_width as usize - 1) / col_width as usize).max(1) as u16
        };

        // Pre-collect names to avoid borrow conflict with clickable_regions
        let names: Vec<String> = s
            .project
            .features
            .iter()
            .map(|f| {
                f.file_path
                    .file_stem()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_else(|| "?".into())
            })
            .collect();
        let selected = s.explore_selected_feature;
        let focused_col = s.explore_focus;

        let mut y_pos = inner.y;
        let mut feature_rows: Vec<(u16, u16)> = Vec::with_capacity(names.len());

        for (i, name) in names.iter().enumerate() {
            let sel = i == selected;
            let is_focused = sel && focused_col == ColumnFocus::Feature;
            let st = if is_focused {
                self.selected_style(true)
            } else if sel {
                self.selected_style(false)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(format!(" {}", name), st)));
            let start = y_pos;
            y_pos += line_rows(&format!(" {}", name));
            feature_rows.push((start, y_pos));
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no features)",
                Style::default().fg(TEXT_MUTED),
            )));
        }
        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );

        // Register clickable regions for each feature row
        for (i, (start, end)) in feature_rows.iter().enumerate() {
            s.clickable_regions
                .push(crate::ClickableRegion::ExploreFeature {
                    feature_idx: i,
                    row_y_start: *start,
                    row_y_end: *end,
                    col_x: inner.x,
                    col_right: inner.right(),
                });
        }
    }

    fn scenario_list(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        // Pre-extract data to avoid borrow conflicts
        let fi = s.selected_feature_index();
        let scenario_count = s.selected_feature().map(|f| f.scenarios.len()).unwrap_or(0);
        let selected_scenario = s.explore_selected_scenario;
        let focused_col = s.explore_focus;

        let n = scenario_count;
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(format!(" Scenarios ({}) ", n))
            .title_style(self.block_title_style(focused_col == ColumnFocus::Scenario));
        let inner = b.inner(area);
        let mut lines: Vec<Line> = Vec::new();
        let col_width = inner.width;

        let line_rows = |content: &str| -> u16 {
            if col_width < 2 {
                return 1;
            }
            let w = content.len();
            if w == 0 {
                return 1;
            }
            ((w + col_width as usize - 1) / col_width as usize).max(1) as u16
        };

        let mut y_pos = inner.y;
        let mut scenario_rows: Vec<(u16, u16)> = Vec::with_capacity(scenario_count);

        if let Some(feat) = s.selected_feature() {
            for (i, sc) in feat.scenarios.iter().enumerate() {
                let sel = i == selected_scenario;
                let is_focused = sel && focused_col == ColumnFocus::Scenario;
                let st = if is_focused {
                    self.selected_style(true)
                } else if sel {
                    self.selected_style(false)
                } else {
                    Style::default()
                };

                let status = s
                    .scenario_status
                    .get(&(fi, i))
                    .map(|s| s.as_str())
                    .unwrap_or("idle");
                let (dot, dot_color) = match status {
                    "passed" => ("●", STAT_PASSED),
                    "failed" => ("●", STAT_FAILED),
                    "running" => ("●", STAT_RUNNING),
                    _ => ("●", STAT_IDLE),
                };

                let kind_icon = match sc.kind {
                    gherkin::ScenarioKind::Scenario => "",
                    gherkin::ScenarioKind::ScenarioOutline => "◈ ",
                };
                lines.push(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(dot, Style::default().fg(dot_color)),
                    Span::styled(format!(" {}{}", kind_icon, sc.name), st),
                ]));
                let start = y_pos;
                y_pos += line_rows(&format!(" {}{}", kind_icon, sc.name));
                scenario_rows.push((start, y_pos));
            }
            if scenario_count == 0 {
                lines.push(Line::from(Span::styled(
                    "  (no scenarios)",
                    Style::default().fg(TEXT_MUTED),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  (select a feature)",
                Style::default().fg(TEXT_MUTED),
            )));
        }
        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );

        // Register clickable regions for each scenario
        for (i, (start, end)) in scenario_rows.iter().enumerate() {
            s.clickable_regions
                .push(crate::ClickableRegion::ExploreScenario {
                    scenario_idx: i,
                    row_y_start: *start,
                    row_y_end: *end,
                    col_x: inner.x,
                    col_right: inner.right(),
                });
        }
    }

    fn step_view(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Steps ")
            .title_style(self.block_title_style(s.explore_focus == ColumnFocus::Step));
        let inner = b.inner(area);
        let mut lines: Vec<Line> = Vec::new();

        // Pre-extract data to avoid borrow conflicts
        let feature = s.selected_feature();
        let scenario = s.selected_scenario();
        let background_steps = feature
            .and_then(|f| f.background.as_ref())
            .map(|bg| bg.steps.as_slice())
            .unwrap_or(&[]);
        let scenario_steps = scenario.map(|s| s.steps.as_slice()).unwrap_or(&[]);
        let bg_count = background_steps.len();
        let sc_step_count = scenario_steps.len();
        let is_focused = s.explore_focus == ColumnFocus::Step;
        let highlight_style = self.selected_style(is_focused);
        let selected_step = s.explore_selected_step;
        let col_width = inner.width;

        // Estimate how many terminal rows a rendered line occupies after wrapping.
        let line_rows = |content: &str| -> u16 {
            if col_width < 2 {
                return 1;
            }
            let w = content.len();
            if w == 0 {
                return 1;
            }
            ((w + col_width as usize - 1) / col_width as usize).max(1) as u16
        };

        // Track actual rendered row position through all elements
        let mut y_pos = inner.y;
        // Store step boundaries: (row_y_start, row_y_end)
        let mut step_rows: Vec<(u16, u16)> = Vec::with_capacity(sc_step_count);

        if bg_count == 0 && sc_step_count == 0 {
            lines.push(Line::from(Span::styled(
                "  (no steps)",
                Style::default().fg(TEXT_MUTED),
            )));
            y_pos += 1;
        } else {
            let mut last_major: Option<Color> = None;

            // ── Background steps ──
            if bg_count > 0 {
                lines.push(Line::from(Span::styled(
                    " Background:",
                    Style::default().fg(TEXT_MUTED).add_modifier(Modifier::BOLD),
                )));
                y_pos += line_rows(" Background:");
                for step in background_steps {
                    let kw_color = self.keyword_color(step.keyword_type, &mut last_major);
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {:>6}", step.keyword),
                            Style::default().fg(kw_color),
                        ),
                        Span::styled(format!(" {}", step.text), Style::default().fg(TEXT_MUTED)),
                    ]));
                    y_pos += line_rows(&format!(" {:>6} {}", step.keyword, step.text));
                }
                lines.push(Line::raw(""));
                y_pos += 1;
            }

            // ── Scenario tags ──
            if let Some(sc) = scenario {
                if !sc.tags.is_empty() {
                    let tag_line = format!("  {}", sc.tags.join(" "));
                    lines.push(Line::from(Span::styled(
                        tag_line.clone(),
                        Style::default().fg(TEXT_MUTED),
                    )));
                    y_pos += line_rows(&tag_line);
                }
            }

            // ── Scenario steps ──
            last_major = None;
            for (i, step) in scenario_steps.iter().enumerate() {
                let kw_color = self.keyword_color(step.keyword_type, &mut last_major);
                let is_selected = i == selected_step;
                let body_span = if is_selected {
                    Span::styled(format!(" {}", step.text), highlight_style)
                } else {
                    Span::raw(format!(" {}", step.text))
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {:>6}", step.keyword),
                        Style::default().fg(kw_color),
                    ),
                    body_span,
                ]));
                let start = y_pos;
                let step_rows_len = line_rows(&format!(" {:>6} {}", step.keyword, step.text));
                y_pos += step_rows_len;
                step_rows.push((start, y_pos));
            }

            // ── Examples tables ──
            if let Some(sc) = scenario {
                for table in &sc.examples {
                    lines.push(Line::raw(""));
                    y_pos += 1;
                    lines.push(Line::from(Span::styled(
                        " Examples:",
                        Style::default().fg(HEADER_CYAN),
                    )));
                    y_pos += line_rows(" Examples:");
                    for row in render_examples_table_lines(&table.headers, &table.rows) {
                        let row_line = format!("  {}", row);
                        lines.push(Line::from(Span::styled(
                            row_line.clone(),
                            Style::default().fg(TEXT_MUTED),
                        )));
                        y_pos += line_rows(&row_line);
                    }
                }
            }
        }

        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );

        // Register clickable step regions using the tracked row ranges
        for (i, (start, end)) in step_rows.iter().enumerate() {
            s.clickable_regions
                .push(crate::ClickableRegion::ExploreStep {
                    step_idx: i,
                    row_y_start: *start,
                    row_y_end: *end,
                    col_x: inner.x,
                    col_right: inner.right(),
                });
        }
    }

    /// Look up keyword colour and update `last_major` so And/But inherit the
    /// preceding Given/When/Then colour.
    fn keyword_color(
        &self,
        kw_type: gherkin::StepKeywordType,
        last_major: &mut Option<Color>,
    ) -> Color {
        match kw_type {
            gherkin::StepKeywordType::Given => {
                *last_major = Some(KWD_GIVEN);
                KWD_GIVEN
            }
            gherkin::StepKeywordType::When => {
                *last_major = Some(KWD_WHEN);
                KWD_WHEN
            }
            gherkin::StepKeywordType::Then => {
                *last_major = Some(KWD_THEN);
                KWD_THEN
            }
            gherkin::StepKeywordType::And => last_major.unwrap_or(KWD_AND_BUT),
            gherkin::StepKeywordType::But => last_major.unwrap_or(KWD_AND_BUT),
        }
    }

    // ── MindMap tab ──

    fn render_mindmap(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        let has_filter = s.mindmap_index.has_active_filter();
        let has_highlight = s.mindmap_index.has_active_highlights();
        let mut title = " MindMap ".to_string();
        if has_filter {
            title.push_str("[filtered] ");
        }
        if has_highlight {
            title.push_str("[highlighted] ");
        }

        // Horizontal split: tree 55% | preview 45%
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // ── Left: tree panel ──
        let tree_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(title)
            .border_style(Style::default().fg(HEADER_CYAN));

        if s.mindmap_index.items.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  (no features loaded for mindmap)",
                    Style::default().fg(TEXT_MUTED),
                )))
                .block(tree_block),
                horiz[0],
            );
        } else {
            let hl = Style::default()
                .fg(SEL_FOCUSED_FG)
                .add_modifier(Modifier::BOLD);

            let tree = match Tree::new(&s.mindmap_index.items) {
                Ok(t) => t.block(tree_block).highlight_style(hl),
                Err(_) => {
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            "  (tree error)",
                            Style::default().fg(TEXT_ERROR),
                        )))
                        .block(tree_block),
                        horiz[0],
                    );
                    return;
                }
            };

            // Store tree panel area for mouse hit-testing
            s.tree_panel_rect = Some(horiz[0]);
            s.clickable_regions.push(crate::ClickableRegion::Tree);

            f.render_stateful_widget(tree, horiz[0], &mut s.tree_state);
        }

        // ── Right: preview panel ──
        s.preview_panel_rect = Some(horiz[1]);
        self.render_mindmap_preview(f, horiz[1], s);
    }

    /// Renders the scenario preview panel in the MindMap tab (right side).
    fn render_mindmap_preview(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let title = if s.preview_title.is_empty() {
            "Preview"
        } else {
            s.preview_title.as_str()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(title)
            .title_style(Style::default().fg(HEADER_CYAN));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        f.render_widget(Clear, inner);

        // Paint full-width spaces to avoid stale pixels
        let buf = f.buffer_mut();
        for i in 0..inner.height {
            let y = inner.y.saturating_add(i);
            if y >= inner.bottom() {
                break;
            }
            buf.set_string(
                inner.x,
                y,
                " ".repeat(inner.width as usize),
                Style::default(),
            );
        }

        // ── Scenario location dropdown (replaces content when open) ──
        if s.scenario_dropdown_open
            && let Some(id) = mindmap::selected_node_id(&s.tree_state)
            && let Some(locations) = s.mindmap_index.locations_for(id)
            && !locations.is_empty()
        {
            let count = locations.len();
            let selection = s.scenario_dropdown_selection.min(count.saturating_sub(1));
            let max_items = (inner.height as usize).saturating_sub(2).min(count);
            let list_height = (max_items + 2).min(inner.height as usize) as u16;

            let dropdown_area =
                Rect::new(inner.x, inner.y, inner.width, list_height.min(inner.height));
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Open Scenario")
                .border_style(Style::default().fg(HEADER_CYAN));
            let di = block.inner(dropdown_area);
            f.render_widget(Clear, dropdown_area);
            f.render_widget(block, dropdown_area);

            let mut items: Vec<String> = Vec::with_capacity(count);
            for loc in locations.iter().take(count) {
                let feature_name = s
                    .project
                    .features
                    .get(loc.feature_idx)
                    .and_then(|f| f.file_path.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("feature_{}", loc.feature_idx));
                let label = match loc.context {
                    mindmap::LocationContext::Scenario(sci) => {
                        let scenario_name = s
                            .project
                            .features
                            .get(loc.feature_idx)
                            .and_then(|f| f.scenarios.get(sci))
                            .map(|s| s.name.as_str())
                            .unwrap_or("?");
                        format!("{}: Scenario: {}", feature_name, scenario_name)
                    }
                    mindmap::LocationContext::Background => {
                        format!("{}: Background", feature_name)
                    }
                };
                items.push(label);
            }

            let visible_items = di.height as usize;
            let scroll_start = selection.saturating_sub(visible_items.saturating_sub(1) / 2);
            let scroll_end = (scroll_start + visible_items).min(items.len());
            let mut dd_lines: Vec<Line<'static>> = Vec::with_capacity(visible_items);

            for (i, item) in items[scroll_start..scroll_end].iter().enumerate() {
                let idx = scroll_start + i;
                let is_selected = idx == selection;
                let prefix = if is_selected { "▸ " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(SEL_FOCUSED_FG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT_MAIN)
                };
                let text = if item.len() > di.width.saturating_sub(2) as usize {
                    let take = di.width.saturating_sub(5) as usize;
                    let truncated: String = item.chars().take(take).collect();
                    format!("{}", truncated)
                } else {
                    item.to_string()
                };
                dd_lines.push(Line::styled(format!("{}{}", prefix, text), style));
            }
            f.render_widget(Paragraph::new(Text::from(dd_lines)), di);

            // Hint line below dropdown
            if dropdown_area.y + dropdown_area.height < inner.y + inner.height {
                let hint_y = dropdown_area.y + dropdown_area.height;
                let hint_line = Line::styled(
                    " ↑↓ select · Enter go · Esc close ",
                    Style::default().fg(TEXT_MUTED),
                );
                f.render_widget(
                    Paragraph::new(hint_line),
                    Rect::new(inner.x, hint_y, inner.width, 1),
                );
            }
            return;
        }

        // ── Normal preview content ──
        let cursor_row = s.preview_cursor_row;
        let visible_lines = inner.height as usize;
        let line_count = s.preview_lines.len();

        // Auto-scroll: center cursor row
        let max_scroll = line_count.saturating_sub(visible_lines);
        let actual_scroll = if cursor_row < visible_lines / 2 || line_count <= visible_lines {
            0
        } else {
            (cursor_row - visible_lines / 2).min(max_scroll)
        };
        s.preview_scroll_row = actual_scroll;

        let cursor_style = Style::default().fg(HEADER_CYAN);
        let mut in_doc_string = false;
        let mut last_major: Option<Color> = None;

        for i in 0..visible_lines {
            let buf_row = actual_scroll + i;
            let y = inner.y.saturating_add(i as u16);
            if y >= inner.bottom() {
                break;
            }

            let line_str = if buf_row < line_count {
                s.preview_lines[buf_row].as_str()
            } else {
                buf.set_string(
                    inner.x,
                    y,
                    " ".repeat(inner.width as usize),
                    Style::default(),
                );
                continue;
            };

            let is_cursor = buf_row == cursor_row;
            let spans = self.highlight_gherkin_line(line_str, &mut in_doc_string, &mut last_major);

            let styled_line = if is_cursor {
                Line::from(Span::styled(line_str.to_string(), cursor_style))
            } else {
                Line::from(spans)
            };

            let styled_line = self.truncate_or_pad(styled_line, inner.width);
            buf.set_line(inner.x, y, &styled_line, inner.width);
        }
    }

    /// Apply inline Gherkin syntax coloring to a single line.
    fn highlight_gherkin_line(
        &self,
        line: &str,
        in_doc_string: &mut bool,
        last_major: &mut Option<Color>,
    ) -> Vec<Span<'static>> {
        let trimmed = line.trim_start();
        let leading_ws = line.len().saturating_sub(trimmed.len());

        // Doc string markers
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("```") {
            *in_doc_string = !*in_doc_string;
            let ws: String = line.chars().take(leading_ws).collect();
            return vec![
                Span::raw(ws),
                Span::styled(trimmed.to_string(), Style::default().fg(TEXT_MUTED)),
            ];
        }
        if *in_doc_string {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(TEXT_MUTED),
            )];
        }

        // Comment
        if trimmed.starts_with('#') {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(TEXT_MUTED),
            )];
        }

        // Data table
        if trimmed.starts_with('|') {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(TEXT_MUTED),
            )];
        }

        // Tags
        if trimmed.starts_with('@') {
            let ws: String = line.chars().take(leading_ws).collect();
            let mut spans = vec![Span::raw(ws)];
            for part in trimmed.split_whitespace() {
                if part.starts_with('@') {
                    spans.push(Span::styled(
                        part.to_string(),
                        Style::default().fg(Color::Rgb(226, 183, 20)),
                    ));
                } else {
                    spans.push(Span::raw(part.to_string()));
                }
                spans.push(Span::raw(" "));
            }
            return spans;
        }

        // Structural headers
        let header_kws = [
            "Feature:",
            "Scenario:",
            "Scenario Outline:",
            "Background:",
            "Examples:",
        ];
        for kw in &header_kws {
            if let Some(rest) = trimmed.strip_prefix(kw) {
                let ws: String = line.chars().take(leading_ws).collect();
                *last_major = None;
                return vec![
                    Span::raw(ws),
                    Span::styled(
                        kw.to_string(),
                        Style::default()
                            .fg(HEADER_CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(rest.to_string()),
                ];
            }
        }

        // Step keywords
        use gherkin::StepKeywordType;
        let step_kws: &[(StepKeywordType, &str, Color)] = &[
            (StepKeywordType::Given, "Given", KWD_GIVEN),
            (StepKeywordType::When, "When", KWD_WHEN),
            (StepKeywordType::Then, "Then", KWD_THEN),
            (StepKeywordType::And, "And", KWD_AND_BUT),
            (StepKeywordType::But, "But", KWD_AND_BUT),
        ];
        for &(_, kw_text, kw_color) in step_kws {
            if let Some(rest) = trimmed.strip_prefix(kw_text) {
                if !rest.is_empty() && !rest.starts_with(' ') {
                    continue;
                }
                let ws: String = line.chars().take(leading_ws).collect();
                let color = if kw_text == "And" || kw_text == "But" {
                    last_major.unwrap_or(kw_color)
                } else {
                    *last_major = Some(kw_color);
                    kw_color
                };
                return vec![
                    Span::raw(ws),
                    Span::styled(kw_text.to_string(), Style::default().fg(color)),
                    Span::raw(rest.to_string()),
                ];
            }
        }

        // Fallback: single plain span
        vec![Span::raw(line.to_string())]
    }

    /// Truncate or pad a Line to exactly `width` columns.
    fn truncate_or_pad(&self, line: Line<'static>, width: u16) -> Line<'static> {
        let w = width as usize;
        let line_w = line.width();
        if line_w > w {
            let mut budget = w;
            let mut out = Vec::new();
            for span in line.spans {
                if budget == 0 {
                    break;
                }
                let s = span.content.to_string();
                let sw = unicode_width::UnicodeWidthStr::width(s.as_str());
                if sw <= budget {
                    out.push(span);
                    budget -= sw;
                } else {
                    let clipped: String = s.chars().take(budget).collect();
                    out.push(Span::styled(clipped, span.style));
                    budget = 0;
                }
            }
            Line::from(out)
        } else if line_w < w {
            let mut line = line;
            line.push_span(Span::styled(" ".repeat(w - line_w), Style::default()));
            line
        } else {
            line
        }
    }

    // ── AI Chat tab ──

    fn render_ai_tab(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        // Layout: sidebar (left, 18 cols) + main (right)
        if area.width < 25 || area.height < 3 {
            return;
        }
        let [sidebar_area, main_area] =
            Layout::horizontal([Constraint::Length(18), Constraint::Min(10)]).areas(area);
        self.render_agent_sidebar(f, sidebar_area, s);
        self.render_agent_chat(f, main_area, s);
    }

    fn render_agent_sidebar(&self, f: &mut Frame, area: Rect, _s: &AppState) {
        if area.width < 5 || area.height < 3 {
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Agents ")
            .style(Style::default());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        // Show a single "Default" agent (the website has only one conversation)
        let prefix = "▸";
        let status_char = "○";
        let title = "Default";
        let text = format!("{prefix} {status_char} {title}");
        let style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        lines.push(Line::styled(text, style));

        lines.push(Line::raw(""));
        lines.push(Line::styled(" 1 agent ", Style::default().fg(TEXT_MUTED)));

        let visible: Vec<Line<'static>> = lines.into_iter().take(inner.height as usize).collect();
        f.render_widget(Paragraph::new(Text::from(visible)), inner);
    }

    fn render_agent_chat(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let block = Block::default().borders(Borders::ALL).title("AI Chat");
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Layout: chat history (top) + status bar (1) + input bar (bottom, min 3)
        let status_height: u16 = 1;
        let input_text_rows = s.chat_input.lines().count().max(1) as u16;
        let input_height: u16 = (input_text_rows + 2).min((inner.height / 3).max(3));
        let chat_height = inner.height.saturating_sub(status_height + input_height);

        let chat_area = Rect::new(inner.x, inner.y, inner.width, chat_height);
        let status_area = Rect::new(inner.x, inner.y + chat_height, inner.width, status_height);
        let input_area = Rect::new(
            inner.x,
            inner.y + chat_height + status_height,
            inner.width,
            input_height,
        );

        // ── Chat history ──
        let mut chat_lines: Vec<Line<'static>> = Vec::new();

        if s.chat_messages.is_empty() {
            chat_lines.push(Line::raw(
                "Welcome to AI Chat! Type a message below and press Enter.",
            ));
            chat_lines.push(Line::raw(""));
        }

        for (role, content) in &s.chat_messages {
            let is_user = role == "user";
            let is_system = role == "system";
            let prefix = if is_user {
                "▶ You"
            } else if is_system {
                " ●"
            } else {
                "> 🥰"
            };
            let role_color = if is_user {
                AI_USER
            } else if is_system {
                TEXT_MUTED
            } else {
                AI_ASSISTANT
            };

            chat_lines.push(
                Line::raw(prefix)
                    .style(Style::default().fg(role_color).add_modifier(Modifier::BOLD)),
            );

            // Render content through markdown
            let md_lines = markdown::render_markdown(content);
            for md_line in md_lines {
                let mut spans = vec![Span::raw("  ")];
                spans.extend(md_line.spans.into_iter());
                let mut line = Line::from(spans);
                line.style = md_line.style;
                chat_lines.push(line);
            }
            chat_lines.push(Line::raw(""));
        }

        // Streaming indicator
        if s.chat_waiting {
            chat_lines.push(
                Line::raw("> 🥰:").style(
                    Style::default()
                        .fg(AI_ASSISTANT)
                        .add_modifier(Modifier::BOLD),
                ),
            );
            chat_lines.push(Line::styled(
                "  Thinking...",
                Style::default().fg(AI_WAITING),
            ));
            chat_lines.push(Line::raw(""));
        }

        // Slice to visible area
        let total_lines = chat_lines.len();
        let max_start = total_lines.saturating_sub(chat_area.height as usize);
        let start = max_start;
        let end = (start + chat_area.height as usize).min(total_lines);
        let visible_lines: Vec<Line<'static>> = chat_lines[start..end].to_vec();

        f.render_widget(
            Paragraph::new(Text::from(visible_lines))
                .wrap(Wrap { trim: false })
                .style(Style::default()),
            chat_area,
        );

        // ── Status bar ──
        let status_text: String = if s.chat_waiting {
            " Teshi is thinking...".into()
        } else {
            String::new()
        };
        if !status_text.is_empty() {
            f.render_widget(
                Paragraph::new(Text::from(
                    Line::raw(status_text).style(Style::default().fg(Color::Yellow)),
                )),
                status_area,
            );
        }

        // ── Input bar ──
        let input_border_style = Style::default().fg(Color::DarkGray);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(input_border_style);
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_display: Text<'static> = if s.chat_input.is_empty() {
            Text::raw("Type your message...")
        } else {
            let lines: Vec<Line<'static>> = s
                .chat_input
                .lines()
                .map(|l| Line::from(Span::raw(l.to_string())))
                .collect();
            Text::from(lines)
        };
        f.render_widget(
            Paragraph::new(input_display).style(if s.chat_waiting {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            }),
            input_inner,
        );
    }

    // ── Status footer (separator + text on same row) ──

    /// Render the 1-line status footer, matching the desktop TUI's per-tab footers.
    /// The row starts with a "─" separator character followed by the status text.
    /// Styled key-hint pill matching the desktop's footer_pill style.
    fn footer_pill(&self, label: &'static str) -> Span<'static> {
        Span::styled(label, self.selected_style(false))
    }

    fn render_footer(&self, f: &mut Frame, area: Rect, s: &AppState) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let sep = Span::styled("─", Style::default().fg(TEXT_MUTED));
        let mut pills = vec![sep, Span::raw(" ")];
        pills.extend(match s.active_tab {
            0 => self.explore_footer_hints(s),
            1 => self.mindmap_footer_hints(),
            2 => self.ai_footer_hints(s),
            _ => vec![],
        });
        f.render_widget(Paragraph::new(Line::from(pills)), area);
    }

    /// Per-tab key-hint pills for the Explore tab.
    fn explore_footer_hints(&self, s: &AppState) -> Vec<Span<'static>> {
        if s.show_raw_feature {
            return vec![
                self.footer_pill(" Enter/Back "),
                self.footer_pill(" 1-3 tabs "),
            ];
        }
        // Left side: feature name + scenario name
        let feat_name = s
            .project
            .features
            .get(s.explore_selected_feature)
            .and_then(|f| f.file_path.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "-".to_string());
        let scen_name = s
            .project
            .features
            .get(s.explore_selected_feature)
            .and_then(|f| f.scenarios.get(s.explore_selected_scenario))
            .map(|s| s.name.as_str())
            .unwrap_or("-");
        let left = format!(" {}  {} ", feat_name, scen_name);
        let mut hints = vec![Span::styled(left, Style::default().fg(TEXT_MUTED))];

        // Right side: key hints
        hints.push(self.footer_pill(" Tab "));
        hints.push(self.footer_pill(" ↑↓ "));
        hints.push(self.footer_pill(" Enter "));
        hints.push(self.footer_pill(" ? "));

        // Run results if available
        let total = s
            .project
            .features
            .get(s.explore_selected_feature)
            .map(|f| f.scenarios.len())
            .unwrap_or(0);
        if total > 0 {
            let passed = s
                .scenario_status
                .values()
                .filter(|v| *v == "passed")
                .count();
            let failed = s
                .scenario_status
                .values()
                .filter(|v| *v == "failed")
                .count();
            let stats = format!(" {}/{} passed, {} failed ", passed, total, failed);
            hints.push(Span::styled(stats, Style::default().fg(TEXT_MUTED)));
        }
        hints
    }

    fn mindmap_footer_hints(&self) -> Vec<Span<'static>> {
        vec![
            self.footer_pill(" ↑↓ select "),
            self.footer_pill(" ←→ expand "),
            self.footer_pill(" Space toggle "),
        ]
    }

    fn ai_footer_hints(&self, s: &AppState) -> Vec<Span<'static>> {
        if s.chat_waiting {
            vec![Span::styled(
                " Thinking...",
                Style::default().fg(AI_WAITING),
            )]
        } else {
            vec![
                self.footer_pill(" Enter send "),
                self.footer_pill(" Esc clear "),
            ]
        }
    }
}

/// Renders an Examples table with aligned column widths (port from desktop).
fn render_examples_table_lines(headers: &[String], rows: &[Vec<String>]) -> Vec<String> {
    if headers.is_empty() {
        return Vec::new();
    }
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|h| UnicodeWidthStr::width(h.as_str()))
        .collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(i) {
                *width = (*width).max(UnicodeWidthStr::width(cell.as_str()));
            }
        }
    }
    let format_row = |cells: &[String]| {
        let mut out = String::from("|");
        for (i, width) in widths.iter().enumerate() {
            let cell = cells.get(i).map_or("", String::as_str);
            let cell_w = UnicodeWidthStr::width(cell);
            let pad = width.saturating_sub(cell_w);
            out.push(' ');
            out.push_str(cell);
            out.push_str(&" ".repeat(pad));
            out.push(' ');
            out.push('|');
        }
        out
    };
    let mut out = Vec::with_capacity(rows.len() + 2);
    out.push(format_row(headers));
    for row in rows {
        out.push(format_row(row));
    }
    out
}
