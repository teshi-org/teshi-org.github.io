//! UI rendering — three-tab layout with Explore and MindMap panels.

use tui_tree_widget::Tree;

use ratzilla::ratatui::Frame;
use ratzilla::ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style};
use ratzilla::ratatui::text::{Line, Span};
use ratzilla::ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Tabs, Wrap};

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

    fn render_tabs(&self, f: &mut Frame, area: Rect, s: &AppState) {
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

    fn render_explore(&self, f: &mut Frame, area: Rect, s: &AppState) {
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

    fn feature_list(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Features ")
            .title_style(self.block_title_style(s.explore_focus == ColumnFocus::Feature));
        let mut lines: Vec<Line> = Vec::new();
        for (i, feat) in s.project.features.iter().enumerate() {
            let name = feat
                .file_path
                .file_stem()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_else(|| "?".into());
            let sel = i == s.explore_selected_feature;
            let is_focused = sel && s.explore_focus == ColumnFocus::Feature;
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
    }

    fn scenario_list(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let feat = s.selected_feature();
        let fi = s.selected_feature_index();
        let n = feat.map(|f| f.scenarios.len()).unwrap_or(0);
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(format!(" Scenarios ({}) ", n))
            .title_style(self.block_title_style(s.explore_focus == ColumnFocus::Scenario));
        let mut lines: Vec<Line> = Vec::new();
        if let Some(feat) = feat {
            for (i, sc) in feat.scenarios.iter().enumerate() {
                let sel = i == s.explore_selected_scenario;
                let is_focused = sel && s.explore_focus == ColumnFocus::Scenario;
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
            if feat.scenarios.is_empty() {
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
    }

    fn step_view(&self, f: &mut Frame, area: Rect, s: &AppState) {
        let b = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Steps ")
            .title_style(self.block_title_style(s.explore_focus == ColumnFocus::Step));
        let mut lines: Vec<Line> = Vec::new();
        if let Some(feat) = s.selected_feature() {
            if let Some(bg) = &feat.background {
                lines.push(Line::from(Span::styled(
                    " Background:",
                    Style::default().fg(TEXT_MUTED).add_modifier(Modifier::BOLD),
                )));
                for step in &bg.steps {
                    lines.push(self.sl(step));
                }
            }
            if let Some(sc) = s.selected_scenario() {
                if !sc.tags.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", sc.tags.join(" ")),
                        Style::default().fg(TEXT_MUTED),
                    )));
                }
                for step in &sc.steps {
                    lines.push(self.sl(step));
                }
                for ex in &sc.examples {
                    lines.push(Line::from(Span::raw("")));
                    lines.push(Line::from(Span::styled(
                        " Examples:",
                        Style::default().fg(HEADER_CYAN),
                    )));
                    if !ex.headers.is_empty() {
                        let h = ex
                            .headers
                            .iter()
                            .map(|h| format!("| {}", h))
                            .collect::<Vec<_>>()
                            .join(" ");
                        lines.push(Line::from(Span::styled(
                            format!("  {}", h),
                            Style::default().fg(TEXT_MUTED),
                        )));
                    }
                    for row in &ex.rows {
                        let r = row
                            .iter()
                            .map(|c| format!("| {}", c))
                            .collect::<Vec<_>>()
                            .join(" ");
                        lines.push(Line::from(Span::styled(
                            format!("  {}", r),
                            Style::default().fg(TEXT_MUTED),
                        )));
                    }
                }
            }
            if lines.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (no steps)",
                    Style::default().fg(TEXT_MUTED),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  (no feature selected)",
                Style::default().fg(TEXT_MUTED),
            )));
        }
        f.render_widget(
            Paragraph::new(lines).block(b).wrap(Wrap { trim: false }),
            area,
        );
    }

    /// Step line with keyword coloring matching the desktop TUI
    fn sl(&self, step: &gherkin::BddStep) -> Line<'static> {
        let kwc = match step.keyword.as_str() {
            "Given" => KWD_GIVEN, // Color::Blue in desktop TUI
            "When" => KWD_WHEN,   // Color::Yellow
            "Then" => KWD_THEN,   // Color::Green
            "And" => KWD_AND_BUT, // Color::Gray
            "But" => KWD_AND_BUT, // Color::Gray
            _ => TEXT_MAIN,
        };
        Line::from(vec![
            Span::styled(format!(" {} ", step.keyword), Style::default().fg(kwc)),
            Span::styled(step.text.clone(), Style::default()),
        ])
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
