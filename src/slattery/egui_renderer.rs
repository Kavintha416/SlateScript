use crate::slattery::sla_interpreter::{UiComponent, UiValue};
use crate::slattery::styles::{StyleApplier, StyleEngine};
use crate::ast_interpreter::AstInterpreter;  // Changed from Interpreter
use crate::value::Value;  // Add this import
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use egui;

pub struct EguiRenderer {
    components: HashMap<String, Rc<RefCell<UiComponent>>>,
    ui_state: HashMap<String, String>,
    style_engine: Rc<RefCell<StyleEngine>>,
    style_applier: StyleApplier,
    pub interpreter: AstInterpreter,  // Add 'pub' here
}

impl EguiRenderer {
    pub fn new() -> Self {
        let style_engine = Rc::new(RefCell::new(StyleEngine::new()));
        let style_applier = StyleApplier::new(style_engine.clone());
        Self {
            components: HashMap::new(),
            ui_state: HashMap::new(),
            style_engine,
            style_applier,
            interpreter: AstInterpreter::new(),  // Changed
        }
    }

    pub fn set_components(&mut self, components: HashMap<String, Rc<RefCell<UiComponent>>>) {
        self.components = components;
    }

    fn execute_slate_function(&mut self, function_name: &str, args: &[Value]) -> Result<(), String> {
        let mut tokens = Vec::new();
        tokens.push(crate::lexer::Token::Identifier(function_name.to_string()));
        if !args.is_empty() {
            tokens.push(crate::lexer::Token::LessThan);
            for (i, arg) in args.iter().enumerate() {
                if i > 0 { tokens.push(crate::lexer::Token::Comma); }
                // In src/slattery/egui_renderer.rs, around line 42
                match arg {
                    Value::String(s) => tokens.push(crate::lexer::Token::String(s.clone())),
                    Value::Int(n)    => tokens.push(crate::lexer::Token::Number(*n)),
                    Value::Float(f)  => tokens.push(crate::lexer::Token::Float(*f)),
                    Value::Bool(b)   => tokens.push(if *b { crate::lexer::Token::True } else { crate::lexer::Token::False }),
                    Value::Null      => tokens.push(crate::lexer::Token::Identifier("null".to_string())),
                    Value::Array(_a) => {
                        tokens.push(crate::lexer::Token::Identifier("array".to_string()));
                    }
                    Value::Object(_o) => {
                        tokens.push(crate::lexer::Token::Identifier("object".to_string()));
                    }
                }
            }
            tokens.push(crate::lexer::Token::GreaterThan);
        } else {
            tokens.push(crate::lexer::Token::LessThan);
            tokens.push(crate::lexer::Token::GreaterThan);
        }
        self.interpreter.run(&tokens).map_err(|e| format!("Failed to execute function: {}", e))
    }

    pub fn load_styles(&mut self, style_files: &[String]) {
        for file_path in style_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Err(e) = self.style_engine.borrow_mut().parse_styles(&content) {
                    eprintln!("Failed to load styles from {}: {}", file_path, e);
                } else {
                    println!("[INFO] Loaded styles from: {}", file_path);
                }
            }
        }
    }

    fn render_text_container(&mut self, ui: &mut egui::Ui, comp: &UiComponent, title: &str) {
        // Outlined container style - thick borders like outlined letters
        let frame = egui::Frame::none()
            .stroke(egui::Stroke::new(4.0, egui::Color32::BLACK))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(20.0, 16.0))
            .outer_margin(egui::Margin::symmetric(6.0, 8.0))
            .fill(egui::Color32::WHITE);

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Title with bold outlined appearance
                ui.label(
                    egui::RichText::new(title)
                        .size(18.0)
                        .color(egui::Color32::BLACK)
                        .strong()
                );

                ui.add_space(8.0);

                // Render children inside the outlined container
                for child in &comp.children {
                    self.render_component(ui, child);
                }

                // Also render referenced child (from Child: <Component> syntax)
                let child_to_render = if let Some(UiValue::String(child_ref)) = comp.get_property("child") {
                    self.components.get(child_ref).cloned()
                } else {
                    None
                };
                if let Some(child_comp) = child_to_render {
                    self.render_component(ui, &child_comp);
                }

                // Render children array if present
                if let Some(UiValue::String(children_ref)) = comp.get_property("children") {
                    if children_ref.starts_with("array:") {
                        let current_id = comp.identity.as_ref().map(|s| s.as_str()).unwrap_or("");
                        let children_to_render: Vec<_> = self.components.iter()
                            .filter(|(name, child_comp)| {
                                let comp_type = child_comp.borrow().component_type.clone();
                                comp_type != "Window" && name.as_str() != current_id
                            })
                            .map(|(_, child_comp)| child_comp.clone())
                            .collect();
                        for child_comp in children_to_render {
                            self.render_component(ui, &child_comp);
                        }
                    }
                }
            });
        });
    }
    
    pub fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let window_components: Vec<_> = self.components.iter()
                .filter(|(_, c)| c.borrow().component_type == "Window")
                .map(|(_, c)| c.clone())
                .collect();
            for wc in window_components {
                self.render_window(ui, &wc);
            }
        });
    }

    fn render_window(&mut self, ui: &mut egui::Ui, window_component: &Rc<RefCell<UiComponent>>) {
        let window = window_component.borrow();
        if let Some(UiValue::String(title)) = window.get_property("title") {
            ui.heading(title);
        }
        for child in &window.children {
            self.render_component(ui, child);
        }
    }

    fn render_component(&mut self, ui: &mut egui::Ui, component: &Rc<RefCell<UiComponent>>) {
        let comp = component.borrow();
        match comp.component_type.as_str() {
            "Column" => {
                ui.vertical(|ui| { self.render_column_children(ui, &comp); });
            }
            "Row" => {
                ui.horizontal(|ui| { self.render_row_children(ui, &comp); });
            }
            "Text" => {
                let text_value = comp.get_property("value")
                    .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "Text".to_string());
                
                let display_text = if let Some(id) = &comp.identity {
                    self.ui_state.get(id).cloned().unwrap_or(text_value)
                } else { text_value };

                
                if comp.children.is_empty() {
                    // Plain text label (original behavior)
                    self.style_applier.apply_text_styles(&comp, ui, &display_text);
                } else {
                    // Text with children becomes a bordered container
                    self.render_text_container(ui, &comp, &display_text);
                }
            }
            "Button" => {
                let clicked = self.style_applier.apply_button_styles(&comp, ui);
                if clicked {
                    if let Some(handler_name) = comp.events.get("on_click").cloned() {
                        if let Err(e) = self.execute_slate_function(&handler_name, &[]) {
                            eprintln!("[ERROR] Event handler '{}' failed: {}", handler_name, e);
                        }
                    }
                }
            }
            "Input" => {
                let mut text = self.ui_state.get("input_field").cloned().unwrap_or_default();
                if ui.text_edit_singleline(&mut text).changed() {
                    self.ui_state.insert("input_field".to_string(), text);
                }
            }
            _ => { ui.label(format!("Unknown component: {}", comp.component_type)); }
        }
    }

    fn render_column_children(&mut self, ui: &mut egui::Ui, column: &UiComponent) {
        for child in &column.children {
            self.render_component(ui, child);
        }
    }

    fn render_row_children(&mut self, ui: &mut egui::Ui, row: &UiComponent) {
        for child in &row.children {
            self.render_component(ui, child);
        }
    }

    pub fn rewrite_component(&mut self, identity: &str, _property: &str, value: &str) {
        self.ui_state.insert(identity.to_string(), value.to_string());
    }

    /// Collect a flat snapshot of all components for the devtools panel.
    pub fn collect_component_tree(&self) -> Vec<DevtoolsEntry> {
        let mut entries = Vec::new();
        // Walk window children recursively
        let windows: Vec<_> = self.components.iter()
            .filter(|(_, c)| c.borrow().component_type == "Window")
            .map(|(_, c)| c.clone())
            .collect();
        for w in windows {
            self.collect_recursive(&w, 0, &mut entries);
        }
        entries
    }

    fn collect_recursive(&self, comp: &Rc<RefCell<UiComponent>>, depth: usize, out: &mut Vec<DevtoolsEntry>) {
        let c = comp.borrow();
        let styles = self.style_applier.resolve_styles(&c);
        out.push(DevtoolsEntry {
            depth,
            component_type: c.component_type.clone(),
            identity: c.identity.clone(),
            properties: c.properties.clone(),
            styles,
        });
        for child in &c.children {
            self.collect_recursive(child, depth + 1, out);
        }
    }
}

/// One row in the devtools component tree.
pub struct DevtoolsEntry {
    pub depth: usize,
    pub component_type: String,
    pub identity: Option<String>,
    pub properties: HashMap<String, UiValue>,
    pub styles: HashMap<String, crate::slattery::styles::StyleValue>,
}

// ─── App ────────────────────────────────────────────────────────────────────

pub struct SlatteryApp {
    renderer: EguiRenderer,
    components: HashMap<String, Rc<RefCell<UiComponent>>>,
    /// Whether the devtools panel is open.
    devtools_open: bool,
    /// Which entry is selected in the component tree.
    devtools_selected: Option<usize>,
    /// Log messages shown in the console tab.
    console_log: Vec<String>,
    /// Active devtools tab.
    devtools_tab: DevtoolsTab,
}

#[derive(PartialEq)]
enum DevtoolsTab { Elements, Styles, Console }

impl SlatteryApp {
    pub fn new(components: HashMap<String, Rc<RefCell<UiComponent>>>) -> Self {
        let mut renderer = EguiRenderer::new();
        renderer.set_components(components.clone());
        Self {
            renderer,
            components,
            devtools_open: false,
            devtools_selected: None,
            console_log: vec!["Slattery DevTools ready.".to_string()],
            devtools_tab: DevtoolsTab::Elements,
        }
    }

    pub fn new_with_styles(
        components: HashMap<String, Rc<RefCell<UiComponent>>>,
        style_files: Vec<String>,
    ) -> Self {
        let mut app = Self::new(components);
        app.renderer.load_styles(&style_files);
        app
    }

    pub fn new_with_renderer(renderer: EguiRenderer) -> Self {
        let components = renderer.components.clone();
        Self {
            renderer,
            components,
            devtools_open: false,
            devtools_selected: None,
            console_log: vec!["Slattery DevTools ready.".to_string()],
            devtools_tab: DevtoolsTab::Elements,
        }
    }
}

impl eframe::App for SlatteryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Toggle devtools with Ctrl+Shift+I
        if ctx.input(|i| {
            i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::I)
        }) {
            self.devtools_open = !self.devtools_open;
        }

        // ── Main app panel ──────────────────────────────────────────────────
        if self.devtools_open {
            // Shrink the main area to leave room for the devtools at the bottom
            egui::TopBottomPanel::bottom("devtools_panel")
                .resizable(true)
                .min_height(200.0)
                .default_height(280.0)
                .show(ctx, |ui| {
                    self.render_devtools(ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let window_components: Vec<_> = self.renderer.components.iter()
                .filter(|(_, c)| c.borrow().component_type == "Window")
                .map(|(_, c)| c.clone())
                .collect();
            for wc in window_components {
                self.renderer.render_window(ui, &wc);
            }
        });
    }
}

impl SlatteryApp {
    fn render_devtools(&mut self, ui: &mut egui::Ui) {
        // ── Header bar ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⚙ Slattery DevTools")
                .color(egui::Color32::from_rgb(255, 59, 48))
                .strong());
            ui.separator();
            ui.selectable_value(&mut self.devtools_tab, DevtoolsTab::Elements, "Elements");
            ui.selectable_value(&mut self.devtools_tab, DevtoolsTab::Styles,   "Styles");
            ui.selectable_value(&mut self.devtools_tab, DevtoolsTab::Console,  "Console");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
                    self.devtools_open = false;
                }
            });
        });
        ui.separator();

        match self.devtools_tab {
            DevtoolsTab::Elements => self.render_elements_tab(ui),
            DevtoolsTab::Styles   => self.render_styles_tab(ui),
            DevtoolsTab::Console  => self.render_console_tab(ui),
        }
    }

    fn render_elements_tab(&mut self, ui: &mut egui::Ui) {
        let tree = self.renderer.collect_component_tree();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, entry) in tree.iter().enumerate() {
                let indent = entry.depth as f32 * 16.0;
                ui.horizontal(|ui| {
                    ui.add_space(indent);

                    let tag = match entry.identity.as_deref() {
                        Some(id) => format!("<{} #{}>", entry.component_type, id),
                        None     => format!("<{}>", entry.component_type),
                    };

                    let selected = self.devtools_selected == Some(idx);
                    let label = egui::RichText::new(&tag)
                        .color(if selected {
                            egui::Color32::from_rgb(255, 59, 48)
                        } else {
                            egui::Color32::from_rgb(100, 180, 255)
                        })
                        .monospace();

                    if ui.selectable_label(selected, label).clicked() {
                        self.devtools_selected = Some(idx);
                        self.devtools_tab = DevtoolsTab::Styles;
                    }

                    // Show key properties inline
                    for (k, v) in &entry.properties {
                        let val_str = match v {
                            UiValue::String(s)  => s.clone(),
                            UiValue::Number(n)  => format!("{}", n),
                            UiValue::Boolean(b) => format!("{}", b),
                            _ => "…".to_string(),
                        };
                        ui.label(
                            egui::RichText::new(format!(" {}=\"{}\"", k, val_str))
                                .color(egui::Color32::GRAY)
                                .monospace()
                                .small(),
                        );
                    }
                });
            }
        });
    }

    fn render_styles_tab(&mut self, ui: &mut egui::Ui) {
        let tree = self.renderer.collect_component_tree();

        if let Some(idx) = self.devtools_selected {
            if let Some(entry) = tree.get(idx) {
                ui.label(egui::RichText::new(format!(
                    "Computed styles for <{}{}>",
                    entry.component_type,
                    entry.identity.as_deref().map(|id| format!(" #{}", id)).unwrap_or_default()
                )).strong());
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if entry.styles.is_empty() {
                        ui.label(egui::RichText::new("(no styles)").color(egui::Color32::GRAY));
                    } else {
                        // Sort for stable display
                        let mut sorted: Vec<_> = entry.styles.iter().collect();
                        sorted.sort_by_key(|(k, _)| k.as_str());
                        for (prop, val) in sorted {
                            let val_str = match val {
                                crate::slattery::styles::StyleValue::Color(c)     => c.clone(),
                                crate::slattery::styles::StyleValue::String(s)    => s.clone(),
                                crate::slattery::styles::StyleValue::Number(n)    => format!("{}", n),
                                crate::slattery::styles::StyleValue::Unit(n, u)   => format!("{}{}", n, u),
                                crate::slattery::styles::StyleValue::Boolean(b)   => format!("{}", b),
                                crate::slattery::styles::StyleValue::None         => "none".to_string(),
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("{}:", prop))
                                    .color(egui::Color32::from_rgb(150, 200, 255))
                                    .monospace());
                                ui.label(egui::RichText::new(&val_str)
                                    .color(egui::Color32::from_rgb(255, 200, 100))
                                    .monospace());
                            });
                        }
                    }
                });
            }
        } else {
            ui.label(egui::RichText::new("← Select an element in the Elements tab")
                .color(egui::Color32::GRAY));
        }
    }

    fn render_console_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in &self.console_log {
                    ui.label(egui::RichText::new(msg).monospace().small());
                }
            });
    }
}

// ─── Public entry points ─────────────────────────────────────────────────────

pub fn run_egui_app(components: HashMap<String, Rc<RefCell<UiComponent>>>) -> Result<(), String> {
    let style_files = crate::slattery::ui_integration::collect_style_files(None);
    run_egui_app_with_styles(components, style_files)
}

pub fn run_egui_app_with_styles(
    components: HashMap<String, Rc<RefCell<UiComponent>>>,
    style_files: Vec<String>,
) -> Result<(), String> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 320.0])
            .with_min_inner_size([320.0, 240.0])
            .with_icon(load_icon().unwrap_or_else(|| {
                let icon_size = 64u32;
                egui::IconData {
                    rgba: vec![0u8; (icon_size * icon_size * 4) as usize],
                    width: icon_size,
                    height: icon_size,
                }
            })),
        ..Default::default()
    };

    eframe::run_native(
        "SlateScript App",
        native_options,
        Box::new(|_cc| Box::new(SlatteryApp::new_with_styles(components, style_files))),
    )
    .map_err(|e| e.to_string())
}

pub fn run_egui_app_with_renderer(renderer: EguiRenderer) -> Result<(), String> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 320.0])
            .with_min_inner_size([320.0, 240.0])
            .with_icon(load_icon().unwrap_or_else(|| {
                let icon_size = 64u32;
                egui::IconData {
                    rgba: vec![0u8; (icon_size * icon_size * 4) as usize],
                    width: icon_size,
                    height: icon_size,
                }
            })),
        ..Default::default()
    };

    eframe::run_native(
        "SlateScript App",
        native_options,
        Box::new(|_cc| Box::new(SlatteryApp::new_with_renderer(renderer))),
    )
    .map_err(|e| e.to_string())
}

fn load_icon() -> Option<egui::IconData> {
    // Try to load from assets/logo.png
    let png_path = std::path::Path::new("assets/logo.png");
    if png_path.exists() {
        if let Ok(img) = image::open(png_path) {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            let rgba = img.into_raw();
            if w > 0 && h > 0 && rgba.len() == (w * h * 4) as usize {
                return Some(egui::IconData { rgba, width: w, height: h });
            }
        }
    }

    // Fallback: colored icon matching your brand
    let sz = 64u32;
    let mut rgba = vec![0u8; (sz * sz * 4) as usize];
    for y in 0..sz {
        for x in 0..sz {
            let idx = ((y * sz + x) * 4) as usize;
            let border = 8;
            if x < border || x >= sz - border || y < border || y >= sz - border {
                rgba[idx] = 255;
                rgba[idx+1] = 59;
                rgba[idx+2] = 48;
                rgba[idx+3] = 255;
            } else {
                rgba[idx] = 26;
                rgba[idx+1] = 26;
                rgba[idx+2] = 26;
                rgba[idx+3] = 255;
            }
        }
    }
    Some(egui::IconData { rgba, width: sz, height: sz })
}