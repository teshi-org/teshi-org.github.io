//! UI rendering — three-tab layout with Explore and MindMap panels.

use tui_tree_widget::Tree;

use ratzilla::ratatui::Frame;
use ratzilla::ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style};
use ratzilla::ratatui::text::{Line, Span};
use ratzilla::ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Tabs, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::{AppState, ColumnFocus, gherkin, mindmap};

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
        s.clickable_regions
            .push(crate::ClickableRegion::Tab(0));
        s.clickable_regions
            .push(crate::ClickableRegion::Tab(1));
        s.clickable_regions
            .push(crate::ClickableRegion::Tab(2));
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
                Constraint::Percentage(20),
                Constraint::Percentage(30),
                Constraint::Percentage(50),
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
        for i in 0..names.len() {
            s.clickable_regions.push(crate::ClickableRegion::ExploreFeature {
                feature_idx: i,
                row_y: inner.y + i as u16,
                col_x: inner.x,
                col_right: inner.right(),
            });
        }
    }

    fn scenario_list(&self, f: &mut Frame, area: Rect, s: &mut AppState) {
        // Pre-extract data to avoid borrow conflicts
        let feat_exists = s.selected_feature().is_some();
        let fi = s.selected_feature_index();
        let scenario_count = s
            .selected_feature()
            .map(|f| f.scenarios.len())
            .unwrap_or(0);
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

        // Register clickable regions after rendering (no borrow conflict)
        if feat_exists {
            for i in 0..scenario_count {
                s.clickable_regions.push(crate::ClickableRegion::ExploreScenario {
                    scenario_idx: i,
                    row_y: inner.y + i as u16,
                    col_x: inner.x,
                    col_right: inner.right(),
                });
            }
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

        if bg_count == 0 && sc_step_count == 0 {
            lines.push(Line::from(Span::styled(
                "  (no steps)",
                Style::default().fg(TEXT_MUTED),
            )));
        } else {
            let mut last_major: Option<Color> = None;

            // ── Background steps ──
            if bg_count > 0 {
                lines.push(Line::from(Span::styled(
                    " Background:",
                    Style::default()
                        .fg(TEXT_MUTED)
                        .add_modifier(Modifier::BOLD),
                )));
                for step in background_steps {
                    let kw_color =
                        self.keyword_color(step.keyword_type, &mut last_major);
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {:>6}", step.keyword),
                            Style::default().fg(kw_color),
                        ),
                        Span::styled(
                            format!(" {}", step.text),
                            Style::default().fg(TEXT_MUTED),
                        ),
                    ]));
                }
                lines.push(Line::raw(""));
            }

            // ── Scenario tags ──
            if let Some(sc) = scenario {
                if !sc.tags.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", sc.tags.join(" ")),
                        Style::default().fg(TEXT_MUTED),
                    )));
                }
            }

            // ── Scenario steps ──
            last_major = None;
            for (i, step) in scenario_steps.iter().enumerate() {
                let kw_color =
                    self.keyword_color(step.keyword_type, &mut last_major);
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
            }

            // ── Examples tables ──
            if let Some(sc) = scenario {
                for table in &sc.examples {
                    lines.push(Line::raw(""));
                    lines.push(Line::from(Span::styled(
                        " Examples:",
                        Style::default().fg(HEADER_CYAN),
                    )));
                    for row in render_examples_table_lines(&table.headers, &table.rows) {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", row),
                            Style::default().fg(TEXT_MUTED),
                        )));
                    }
                }
            }
        }

        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );

        // Register clickable step regions after rendering
        // Recompute row position for the first scenario step
        let mut step_row = inner.y;
        if bg_count > 0 {
            step_row += 1; // "Background:" header
            step_row += bg_count as u16; // background step lines
            step_row += 1; // empty line after background
        }
        if let Some(sc) = scenario
            && !sc.tags.is_empty()
        {
            step_row += 1;
        }
        for i in 0..sc_step_count {
            s.clickable_regions.push(crate::ClickableRegion::ExploreStep {
                step_idx: i,
                row_y: step_row + i as u16,
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

        let block = Block::default()
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
                .block(block),
                area,
            );
            return;
        }

        let hl = Style::default()
            .fg(SEL_FOCUSED_FG)
            .add_modifier(Modifier::BOLD);

        let tree = match Tree::new(&s.mindmap_index.items) {
            Ok(t) => t.block(block).highlight_style(hl),
            Err(_) => {
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "  (tree error)",
                        Style::default().fg(TEXT_ERROR),
                    )))
                    .block(block),
                    area,
                );
                return;
            }
        };

        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(6)])
            .split(area);

        // Store tree panel area for mouse hit-testing
        s.tree_panel_rect = Some(vert[0]);
        s.clickable_regions.push(crate::ClickableRegion::Tree);

        f.render_stateful_widget(tree, vert[0], &mut s.tree_state);
        self.mindmap_context(f, vert[1], s);
    }

    fn mindmap_context(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let ctx = s
            .mindmap_selected_id
            .as_ref()
            .and_then(|id| mindmap::selected_node_context(&s.tree_state, &s.mindmap_index));
        let b = Block::default()
            .borders(Borders::TOP)
            .padding(Padding::horizontal(1))
            .title(" Node ")
            .border_style(Style::default().fg(TEXT_MUTED));
        let lines = match ctx {
            Some(c) => vec![
                Line::from(Span::styled(
                    format!(" Step: {}", c.step_text),
                    Style::default(),
                )),
                Line::from(Span::styled(
                    format!(" Locations: {}", c.location_count),
                    Style::default().fg(TEXT_MUTED),
                )),
                Line::from(Span::styled(
                    format!(" Path: {}", c.path_labels.join(" → ")),
                    Style::default().fg(TEXT_MUTED),
                )),
            ],
            None => vec![Line::from(Span::styled(
                "  (select a node)",
                Style::default().fg(TEXT_MUTED),
            ))],
        };
        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );
    }

    // ── AI Chat tab ──

    fn render_ai_tab(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        self.render_chat_messages(f, chunks[0], s);
        self.render_chat_input(f, chunks[1], s);
    }

    fn render_chat_messages(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Chat ")
            .border_style(Style::default().fg(HEADER_CYAN));

        let mut lines: Vec<Line> = Vec::new();
        for (role, content) in &s.chat_messages {
            let is_user = role == "user";
            let is_system = role == "system";
            let role_label = if is_user {
                " You"
            } else if is_system {
                " ●"
            } else {
                " AI"
            };
            let role_color = if is_user {
                AI_USER
            } else if is_system {
                TEXT_MUTED
            } else {
                AI_ASSISTANT
            };

            lines.push(Line::from(Span::styled(
                format!("{}", role_label),
                Style::default().fg(role_color).add_modifier(Modifier::BOLD),
            )));

            for line in content.lines() {
                if is_system {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", line),
                        Style::default().fg(TEXT_MUTED),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", line),
                        Style::default(),
                    )));
                }
            }
            lines.push(Line::from(Span::raw("")));
        }

        if s.chat_waiting {
            lines.push(Line::from(Span::styled(
                " Thinking...",
                Style::default().fg(AI_WAITING),
            )));
        }

        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_chat_input(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Input ")
            .border_style(Style::default().fg(HEADER_CYAN));

        let input_display = if s.chat_input.is_empty() {
            " Type a message and press Enter...".to_string()
        } else {
            format!(" {}", s.chat_input)
        };

        let input_style = if s.chat_input.is_empty() {
            Style::default().fg(TEXT_MUTED)
        } else {
            Style::default().fg(TEXT_MAIN)
        };

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(input_display, input_style))).block(block),
            area,
        );
    }

    // ── Status footer (separator + text on same row) ──

    /// Render the 1-line status footer, matching the desktop TUI's per-tab footers.
    /// The row starts with a "─" separator character followed by the status text.
    fn render_footer(&self, f: &mut Frame, area: Rect, s: &AppState) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let sep = Span::styled("─", Style::default().fg(TEXT_MUTED));
        let (text, style) = match s.active_tab {
            0 => self.explore_footer_text(s),
            1 => (self.mindmap_footer_text(), Style::default().fg(TEXT_MUTED)),
            2 => (self.ai_footer_text(s), Style::default().fg(TEXT_MUTED)),
            _ => (String::new(), Style::default()),
        };
        let full = format!(" {}", text);
        let max_chars = area.width.saturating_sub(1) as usize;
        let clipped: String = if full.chars().count() > max_chars {
            let keep = max_chars.saturating_sub(3);
            if keep > 0 {
                let truncated: String = full.chars().take(keep).collect();
                format!("{}...", truncated)
            } else {
                String::new()
            }
        } else {
            full
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![sep, Span::styled(clipped, style)])),
            area,
        );
    }

    /// Build the explore footer line: feature/scenario name + test results.
    fn explore_footer_text(&self, s: &AppState) -> (String, Style) {
        if s.show_raw_feature {
            return (
                " [Enter/Back] exit raw view  [1-3] tabs".into(),
                Style::default().fg(TEXT_MUTED),
            );
        }
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
        let left = format!("{}  {}", feat_name, scen_name);
        let total = s
            .project
            .features
            .get(s.explore_selected_feature)
            .map(|f| f.scenarios.len())
            .unwrap_or(0);
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
        let right = if total > 0 {
            format!("  {}/{} passed, {} failed", passed, total, failed)
        } else {
            String::new()
        };
        (format!("{left}{right}"), Style::default().fg(TEXT_MUTED))
    }

    fn mindmap_footer_text(&self) -> String {
        " [↑↓] select  [←] collapse  [→] expand  [Space] toggle  [1-3] tabs".into()
    }

    fn ai_footer_text(&self, s: &AppState) -> String {
        if s.chat_waiting {
            " Thinking...".into()
        } else {
            " Type & press Enter to send  [Esc] clear input  [1-3] tabs".into()
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
