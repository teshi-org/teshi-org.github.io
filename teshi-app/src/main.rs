//! Teshi TUI — WASM entry point.

use std::cell::RefCell;
use std::rc::Rc;

use ratzilla::event::{KeyCode, MouseEvent, MouseEventKind, MouseButton};
use ratzilla::ratatui::Terminal;
use ratzilla::ratatui::layout::Rect;
use ratzilla::{DomBackend, WebRenderer};

use web_sys;

mod demo;
mod diff;
mod gherkin;
mod llm;
mod mindmap;
mod network;
mod step_index;
mod storage;
mod ui;

use network::{RunnerCommand, RunnerConnection};
use ui::AppUi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnFocus { Feature, Scenario, Step }

/// A clickable region registered during rendering for mouse hit-testing.
/// Stored in cell coordinates (col/row).
#[derive(Debug, Clone)]
pub enum ClickableRegion {
    Tab(usize),
    Tree,
    ExploreFeature {
        feature_idx: usize,
        row_y: u16,
        col_x: u16,
        col_right: u16,
    },
    ExploreScenario {
        scenario_idx: usize,
        row_y: u16,
        col_x: u16,
        col_right: u16,
    },
    ExploreStep {
        step_idx: usize,
        row_y: u16,
        col_x: u16,
        col_right: u16,
    },
}

pub struct AppState {
    pub runner: RunnerConnection,
    pub active_tab: usize,
    pub status: String,
    pub project: gherkin::BddProject,
    // Explore tab
    pub explore_focus: ColumnFocus,
    pub explore_selected_feature: usize,
    pub explore_selected_scenario: usize,
    pub explore_selected_step: usize,
    // MindMap tab
    pub mindmap_index: mindmap::MindMapIndex,
    pub tree_state: tui_tree_widget::TreeState<String>,
    pub mindmap_selected_id: Option<String>,
    // AI Chat tab
    pub chat_messages: Vec<(String, String)>,  // (role, content)
    pub chat_input: String,
    pub chat_api_key: String,
    pub chat_waiting: bool,
    // Scenario run status tracking (feature_idx, scenario_idx) -> status
    pub scenario_status: std::collections::HashMap<(usize, usize), String>,
    // Feature detail view
    pub show_raw_feature: bool,
    pub raw_feature_index: usize,
    // Pending AI response (processed on next frame so "Thinking..." is visible)
    pub pending_chat_response: Option<String>,
    // Help overlay
    pub show_help: bool,
    // Clickable regions (re-registered every render frame)
    pub clickable_regions: Vec<ClickableRegion>,
    // MindMap tree panel area for mouse hit-testing
    pub tree_panel_rect: Option<Rect>,
}

impl AppState {
    fn new() -> Self {
        let runner = RunnerConnection::connect();
        let api_key = storage::load_settings().unwrap_or_default();
        let feature_names = storage::list_features();
        let mut features = Vec::new();
        for name in &feature_names {
            if let Some(content) = storage::load_feature(name) {
                let path = std::path::PathBuf::from(name);
                features.push(gherkin::parse_feature(&content, path));
            }
        }
        let project = gherkin::BddProject {
            root_dir: std::path::PathBuf::from("local"),
            features,
        };

        let mindmap_index = mindmap::build_index(&project);
        let mut tree_state: tui_tree_widget::TreeState<String> = tui_tree_widget::TreeState::default();
        if !mindmap_index.items.is_empty() {
            tree_state.select(vec!["root".to_string()]);
        }
        let mindmap_selected_id = mindmap::selected_node_id(&tree_state).map(|s| s.to_string());

        Self {
            runner,
            active_tab: 0,
            status: "Ready".into(),
            project,
            explore_focus: ColumnFocus::Feature,
            explore_selected_feature: 0,
            explore_selected_scenario: 0,
            explore_selected_step: 0,
            mindmap_index,
            tree_state,
            mindmap_selected_id,
            chat_messages: vec![
                ("system".into(), "Welcome to Teshi AI! I can help you analyze your Gherkin feature files, generate test scenarios, and answer questions about BDD. Try asking something about your features.".into())
            ],
            chat_input: String::new(),
            chat_api_key: api_key,
            chat_waiting: false,
            scenario_status: std::collections::HashMap::new(),
            show_raw_feature: false,
            raw_feature_index: 0,
            pending_chat_response: None,
            show_help: false,
            clickable_regions: Vec::new(),
            tree_panel_rect: None,
        }
    }

    /// Reload feature files from localStorage and rebuild project + mindmap.
    pub fn reload_features(&mut self) {
        let feature_names = storage::list_features();
        let mut features = Vec::new();
        for name in &feature_names {
            if let Some(content) = storage::load_feature(name) {
                let path = std::path::PathBuf::from(name);
                features.push(gherkin::parse_feature(&content, path));
            }
        }
        self.project = gherkin::BddProject {
            root_dir: std::path::PathBuf::from("local"),
            features,
        };
        self.mindmap_index = mindmap::build_index(&self.project);
        let mut new_state: tui_tree_widget::TreeState<String> = tui_tree_widget::TreeState::default();
        if !self.mindmap_index.items.is_empty() {
            new_state.select(vec!["root".to_string()]);
        }
        self.tree_state = new_state;
        self.mindmap_selected_id = mindmap::selected_node_id(&self.tree_state).map(|s| s.to_string());
        self.explore_selected_feature = 0;
        self.explore_selected_scenario = 0;
        self.explore_selected_step = 0;
        self.show_raw_feature = false;
    }

    /// Simulate running all scenarios with random pass/fail results.
    fn simulate_run(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
        let mut rng = seed;
        let mut passed = 0u32;
        let mut failed = 0u32;
        for (fi, feat) in self.project.features.iter().enumerate() {
            for (si, _sc) in feat.scenarios.iter().enumerate() {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let roll = (rng >> 33) as u8;
                if roll < 200 { // ~78% pass rate
                    self.scenario_status.insert((fi, si), "passed".into());
                    passed += 1;
                } else {
                    self.scenario_status.insert((fi, si), "failed".into());
                    failed += 1;
                }
            }
        }
        let total = passed + failed;
        self.status = format!("Simulated {} scenarios: {} passed, {} failed", total, passed, failed);
    }

    pub fn poll_runner(&mut self) {
        // Process pending AI response (one frame delay so "Thinking..." is visible)
        if let Some(response) = self.pending_chat_response.take() {
            self.chat_messages.pop(); // remove "Thinking..."
            self.chat_messages.push(("assistant".into(), response));
            self.chat_waiting = false;
            self.status = "Ready".into();
        }
        let events = self.runner.poll().to_vec();
        for event in &events {
            match event {
                network::RunnerEvent::TestStarted { scenario } => {
                    self.status = format!("Running: {}", scenario);
                    // Mark scenario as running if we can find it
                    self.mark_scenario_status(scenario, "running");
                }
                network::RunnerEvent::TestStep { step, status } => {
                    self.status = format!("{} → {}", step, status);
                }
                network::RunnerEvent::TestOutput { line } => {
                    self.status = format!("Output: {}", line);
                }
                network::RunnerEvent::TestFinished { scenario, status, duration_ms } => {
                    self.status = format!("{}: {} ({}ms)", scenario, status, duration_ms);
                    self.mark_scenario_status(scenario, &status);
                }
                network::RunnerEvent::Pong => {
                    self.status = "Connected".into();
                }
            }
        }
        if !self.runner.connected {
            self.status = "Runner disconnected".into();
        }
    }

    fn sync_tree_selection(&mut self) {
        self.mindmap_selected_id = mindmap::selected_node_id(&self.tree_state).map(|s| s.to_string());
        if let Some(ref id) = self.mindmap_selected_id.clone() {
            self.mindmap_index.apply_highlight_categories(id);
        }
    }

    /// Find a scenario by name across all features and set its run status.
    fn mark_scenario_status(&mut self, scenario_name: &str, status: &str) {
        for (fi, feat) in self.project.features.iter().enumerate() {
            for (si, sc) in feat.scenarios.iter().enumerate() {
                if sc.name == scenario_name || sc.name.contains(&scenario_name) {
                    self.scenario_status.insert((fi, si), status.to_string());
                    return;
                }
            }
        }
    }

    /// Convert viewport pixel coordinates to terminal cell coordinates.
    fn pixel_to_cell(&self, px: u32, py: u32) -> (Option<u16>, Option<u16>) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return (None, None),
        };
        let doc = match window.document() {
            Some(d) => d,
            None => return (None, None),
        };
        let terminal = match doc.get_element_by_id("terminal-body") {
            Some(el) => el,
            None => return (None, None),
        };
        let rect = terminal.get_bounding_client_rect();
        let cell_x = (px as f64 - rect.left()) / 10.0;
        let cell_y = (py as f64 - rect.top()) / 20.0;
        if cell_x < 0.0 || cell_y < 0.0 {
            return (None, None);
        }
        (Some(cell_x as u16), Some(cell_y as u16))
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent) {
        if event.event != MouseEventKind::Pressed || event.button != MouseButton::Left {
            return;
        }
        let (col, row) = self.pixel_to_cell(event.x, event.y);
        let (col, row) = match (col, row) {
            (Some(c), Some(r)) => (c, r),
            _ => return,
        };

        // Hit-test against regions registered during rendering
        for region in &self.clickable_regions {
            match region {
                ClickableRegion::Tab(tab_idx) => {
                    // Tab labels on row 0 with hardcoded x-offsets matching Tabs widget
                    // " Explore [1] " (14) + " " + " MindMap [2] " (14) + " " + " AI [3] " (9)
                    let tab_starts: &[u16] = &[0, 15, 30];
                    let tab_ends: &[u16] = &[14, 29, 39];
                    if row == 0
                        && col >= tab_starts[*tab_idx]
                        && col < tab_ends[*tab_idx]
                    {
                        self.active_tab = *tab_idx;
                        return;
                    }
                }
                ClickableRegion::Tree => {
                    if self.active_tab == 1
                        && let Some(rect) = self.tree_panel_rect
                        && col >= rect.x
                        && col < rect.right()
                        && row >= rect.y
                        && row < rect.bottom()
                    {
                        let pos = ratzilla::ratatui::layout::Position::new(col, row);
                        if self.tree_state.click_at(pos)
                            && let Some(id) = mindmap::selected_node_id(&self.tree_state)
                        {
                            self.mindmap_index.apply_highlight_categories(id);
                        }
                        self.mindmap_selected_id = mindmap::selected_node_id(&self.tree_state)
                            .map(|s| s.to_string());
                        return;
                    }
                }
                ClickableRegion::ExploreFeature {
                    feature_idx,
                    row_y,
                    col_x,
                    col_right,
                } => {
                    if row == *row_y && col >= *col_x && col < *col_right {
                        self.explore_selected_feature = *feature_idx;
                        self.explore_focus = ColumnFocus::Feature;
                        self.explore_selected_scenario = 0;
                        self.explore_selected_step = 0;
                        return;
                    }
                }
                ClickableRegion::ExploreScenario {
                    scenario_idx,
                    row_y,
                    col_x,
                    col_right,
                } => {
                    if row == *row_y && col >= *col_x && col < *col_right {
                        self.explore_selected_scenario = *scenario_idx;
                        self.explore_focus = ColumnFocus::Scenario;
                        self.explore_selected_step = 0;
                        return;
                    }
                }
                ClickableRegion::ExploreStep {
                    step_idx,
                    row_y,
                    col_x,
                    col_right,
                } => {
                    if row == *row_y && col >= *col_x && col < *col_right {
                        self.explore_selected_step = *step_idx;
                        self.explore_focus = ColumnFocus::Step;
                        return;
                    }
                }
            }
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('1') => { self.active_tab = 0; true }
            KeyCode::Char('2') => { self.active_tab = 1; true }
            KeyCode::Char('3') => { self.active_tab = 2; true }
            KeyCode::Char('r') => {
                self.runner.send(&RunnerCommand::Ping);
                self.status = "Ping sent...".into();
                true
            }
            KeyCode::Char('R') => {
                self.reload_features();
                self.status = "Features reloaded.".into();
                true
            }
            KeyCode::Char('t') => {
                self.simulate_run();
                true
            }
            // ── Explore tab navigation ──
            KeyCode::Tab if self.active_tab == 0 => {
                self.explore_focus = match self.explore_focus {
                    ColumnFocus::Feature => ColumnFocus::Scenario,
                    ColumnFocus::Scenario => ColumnFocus::Step,
                    ColumnFocus::Step => ColumnFocus::Feature,
                };
                true
            }
            KeyCode::Left if self.active_tab == 0 => {
                self.explore_focus = match self.explore_focus {
                    ColumnFocus::Scenario => ColumnFocus::Feature,
                    ColumnFocus::Step => ColumnFocus::Scenario,
                    _ => ColumnFocus::Feature,
                };
                true
            }
            KeyCode::Right if self.active_tab == 0 => {
                self.explore_focus = match self.explore_focus {
                    ColumnFocus::Feature => ColumnFocus::Scenario,
                    ColumnFocus::Scenario => ColumnFocus::Step,
                    _ => ColumnFocus::Step,
                };
                true
            }
            KeyCode::Up if self.active_tab == 0 => {
                match self.explore_focus {
                    ColumnFocus::Feature => {
                        self.explore_selected_feature = self.explore_selected_feature.saturating_sub(1);
                    }
                    ColumnFocus::Scenario => {
                        self.explore_selected_scenario = self.explore_selected_scenario.saturating_sub(1);
                    }
                    ColumnFocus::Step => {
                        self.explore_selected_step = self.explore_selected_step.saturating_sub(1);
                    }
                }
                true
            }
            KeyCode::Down if self.active_tab == 0 => {
                match self.explore_focus {
                    ColumnFocus::Feature => {
                        let max = self.project.features.len().saturating_sub(1);
                        self.explore_selected_feature = self.explore_selected_feature.min(max);
                        if self.explore_selected_feature < max { self.explore_selected_feature += 1; }
                    }
                    ColumnFocus::Scenario => {
                        let n = self.project.features.get(self.explore_selected_feature)
                            .map(|f| f.scenarios.len()).unwrap_or(0);
                        let max = n.saturating_sub(1);
                        self.explore_selected_scenario = self.explore_selected_scenario.min(max);
                        if self.explore_selected_scenario < max { self.explore_selected_scenario += 1; }
                    }
                    ColumnFocus::Step => {
                        let n = self.feature_step_or_bg_count();
                        let max = n.saturating_sub(1);
                        self.explore_selected_step = self.explore_selected_step.min(max);
                        if self.explore_selected_step < max { self.explore_selected_step += 1; }
                    }
                }
                true
            }
            // ── MindMap tab navigation ──
            KeyCode::Up if self.active_tab == 1 => {
                self.tree_state.key_up();
                self.sync_tree_selection();
                true
            }
            KeyCode::Down if self.active_tab == 1 => {
                self.tree_state.key_down();
                self.sync_tree_selection();
                true
            }
            KeyCode::Left if self.active_tab == 1 => {
                self.tree_state.key_left();
                self.sync_tree_selection();
                true
            }
            KeyCode::Right if self.active_tab == 1 => {
                self.tree_state.key_right();
                self.sync_tree_selection();
                true
            }
            KeyCode::Enter if self.active_tab == 0 && self.explore_focus == ColumnFocus::Feature => {
                // Toggle raw feature detail view
                self.show_raw_feature = !self.show_raw_feature;
                self.raw_feature_index = self.explore_selected_feature;
                true
            }
            KeyCode::Backspace if self.active_tab == 0 && self.show_raw_feature => {
                self.show_raw_feature = false;
                true
            }
            KeyCode::Enter if self.active_tab == 0 => {
                self.show_raw_feature = false;
                true
            }
            KeyCode::Enter if self.active_tab == 1 => {
                self.tree_state.toggle_selected();
                self.sync_tree_selection();
                true
            }
            KeyCode::Char(' ') if self.active_tab == 1 => {
                self.tree_state.toggle_selected();
                self.sync_tree_selection();
                true
            }
            // ── AI Chat tab ──
            KeyCode::Char(c) if self.active_tab == 2 && !self.chat_waiting => {
                if c == '\n' {
                    // Send message on Enter
                    let msg = std::mem::take(&mut self.chat_input);
                    if !msg.is_empty() {
                        self.chat_messages.push(("user".into(), msg.clone()));
                        self.chat_waiting = true;
                        self.chat_messages.push(("assistant".into(), "Thinking...".into()));
                        self.status = "Thinking...".into();
                        // Simulate a response after a delay via setTimeout
                        self.simulate_chat_response(msg);
                    }
                } else {
                    self.chat_input.push(c);
                }
                true
            }
            KeyCode::Backspace if self.active_tab == 2 => {
                self.chat_input.pop();
                true
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                true
            }
            KeyCode::Esc if self.show_help => {
                self.show_help = false;
                true
            }
            KeyCode::Enter if self.active_tab == 2 && !self.chat_waiting => {
                let msg = std::mem::take(&mut self.chat_input);
                if !msg.is_empty() {
                    self.chat_messages.push(("user".into(), msg.clone()));
                    self.chat_waiting = true;
                    self.chat_messages.push(("assistant".into(), "Thinking...".into()));
                    self.status = "Thinking...".into();
                    self.simulate_chat_response(msg);
                }
                true
            }
            _ => false,
        }
    }

    /// Generate a contextual mock AI response based on the user message and loaded features.
    fn simulate_chat_response(&mut self, user_msg: String) {
        let msg_lower = user_msg.to_lowercase();

        let response = if msg_lower.contains("hello") || msg_lower.contains("hi ") || msg_lower == "hi" {
            "Hello! I'm Teshi AI. I can help you analyze your Gherkin feature files, suggest test scenarios, and answer BDD questions. What would you like to know?".to_string()
        } else if msg_lower.contains("feature") || msg_lower.contains("project") || msg_lower.contains("what") {
            // Summarize features
            let lines: Vec<String> = self.project.features.iter().map(|f| {
                let name = f.file_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let sc_count = f.scenarios.len();
                let tags = if f.tags.is_empty() { String::new() } else { format!(" [{}]", f.tags.join(", ")) };
                format!("  • {}{} — {} scenarios", name, tags, sc_count)
            }).collect();
            if lines.is_empty() {
                "No features loaded yet. Go to the homepage and click 'Load Demo Data' to get started.".to_string()
            } else {
                format!("Your project has {} feature file(s):\n{}\n\nPress Enter on a feature in the Explore tab to see its raw content. Switch to the MindMap tab (press 2) to see the step-sequence tree.", self.project.features.len(), lines.join("\n"))
            }
        } else if msg_lower.contains("scenario") || msg_lower.contains("test") {
            // List all scenarios
            let lines: Vec<String> = self.project.features.iter().flat_map(|f| {
                let fname = f.file_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                f.scenarios.iter().map(move |sc| {
                    format!("  • [{}] {} ({})", fname, sc.name, match sc.kind { gherkin::ScenarioKind::Scenario => "Scenario", gherkin::ScenarioKind::ScenarioOutline => "Outline" })
                })
            }).collect();
            if lines.is_empty() {
                "No scenarios found. Load some feature files first.".to_string()
            } else {
                format!("Here are all scenarios across your project:\n{}\n\nUse ↑↓ in the Explore tab to browse them.", lines.join("\n"))
            }
        } else if msg_lower.contains("step") || msg_lower.contains("given") || msg_lower.contains("when") || msg_lower.contains("then") {
            "Steps in Gherkin are organized with keywords: **Given** (precondition), **When** (action), **Then** (expected outcome), **And**/**But** (conjunction).\n\nThe MindMap tab builds a step-sequence tree so you can see how steps flow across scenarios.".to_string()
        } else if msg_lower.contains("help") || msg_lower.contains("?") {
            "**Keyboard shortcuts:**\n  • 1/2/3 — Switch tabs\n  • Tab/←→ — Focus columns (Explore)\n  • ↑↓ — Navigate items\n  • Enter — View raw / Toggle tree node\n  • ? — Toggle this help\n  • q/Esc — Exit to homepage\n\n**Tips:**\n  • Click features/scenarios in Explore tab\n  • Build step-sequence trees in MindMap tab\n  • I can answer questions about your features!".to_string()
        } else {
            // Check for feature name matches
            let matched: Vec<String> = self.project.features.iter().filter_map(|f| {
                let name = f.file_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if msg_lower.contains(&name.to_lowercase()) || name.to_lowercase().contains(&msg_lower) {
                    let sc_list: Vec<String> = f.scenarios.iter().map(|sc| format!("    • {}", sc.name)).collect();
                    Some(format!("**{}**\n{} scenarios\n{}", name, f.scenarios.len(), sc_list.join("\n")))
                } else { None }
            }).collect();

            if !matched.is_empty() {
                format!("I found matching features:\n\n{}", matched.join("\n\n"))
            } else {
                let lines: Vec<String> = self.project.features.iter().map(|f| {
                    let name = f.file_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    format!("  • {}", name)
                }).collect();
                format!("I'm not sure I understand. I can help with questions about your {} feature file(s):\n{}\n\nTry asking about a specific feature or type 'help' for keyboard shortcuts.", self.project.features.len(), lines.join("\n"))
            }
        };

        self.pending_chat_response = Some(response);
    }

    pub fn selected_feature(&self) -> Option<&gherkin::BddFeature> {
        self.project.features.get(self.explore_selected_feature)
    }

    pub fn selected_scenario(&self) -> Option<&gherkin::BddScenario> {
        self.selected_feature().and_then(|f| f.scenarios.get(self.explore_selected_scenario))
    }

    fn feature_step_or_bg_count(&self) -> usize {
        self.selected_feature()
            .map(|f| {
                let bg = f.background.as_ref().map(|b| b.steps.len()).unwrap_or(0);
                let sc = f.scenarios.get(self.explore_selected_scenario)
                    .map(|s| s.steps.len()).unwrap_or(0);
                bg + sc
            })
            .unwrap_or(0)
    }

    pub fn selected_feature_index(&self) -> usize { self.explore_selected_feature }
    pub fn selected_scenario_index(&self) -> usize { self.explore_selected_scenario }
}

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Teshi TUI starting...");

    // Auto-load demo data on first visit
    let mut force_demo = false;
    if let Some(window) = web_sys::window() {
        force_demo = window.location().search().unwrap_or_default().contains("demo=true");
    }
    if force_demo || storage::list_features().is_empty() {
        log::info!("Loading demo data...");
        for (name, content) in demo::DEMO_FEATURES {
            storage::save_feature(name, content);
        }
        if force_demo {
            if let Some(window) = web_sys::window() {
                let base = window.location().origin().unwrap_or_default();
                let path = window.location().pathname().unwrap_or_default();
                let _ = window.location().replace(&format!("{}{}", base, path));
            }
        }
    }

    let state = Rc::new(RefCell::new(AppState::new()));

    let backend = match DomBackend::new_by_id("terminal-body") {
        Ok(b) => b,
        Err(e) => { log::error!("Failed to create DomBackend: {:?}", e); return; }
    };
    let terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => { log::error!("Failed to create Terminal: {:?}", e); return; }
    };

    // Exit handler: navigate back to homepage
    {
        let base_url = web_sys::window()
            .and_then(|w| {
                let origin = w.location().origin().ok();
                let path = w.location().pathname().ok();
                match (origin, path) {
                    (Some(o), Some(p)) => {
                        // Go up one level from /app/ to /
                        Some(if p.ends_with("/app/") || p == "/app/" {
                            o
                        } else {
                            let mut parts: Vec<&str> = p.rsplitn(2, '/').collect();
                            format!("{}/{}", o, parts.last().unwrap_or(&""))
                        })
                    }
                    _ => None,
                }
            })
            .unwrap_or_else(|| "/".to_string());

        let state_keys = state.clone();
        terminal.on_key_event(move |key_event| {
            let mut s = state_keys.borrow_mut();
            match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    log::info!("Navigating to homepage");
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().set_href(&base_url);
                    }
                }
                code => { s.handle_key(code); }
            }
        });
    }

    let state_mouse = state.clone();
    terminal.on_mouse_event(move |mouse_event| {
        let mut s = state_mouse.borrow_mut();
        s.handle_mouse(&mouse_event);
    });

    let mut app_ui = AppUi::new();
    terminal.draw_web(move |f| {
        let mut s = state.borrow_mut();
        s.poll_runner();
        app_ui.render(f, &mut s);
    });
}
