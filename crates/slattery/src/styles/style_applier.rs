//! Style Applier
//! 
//! Applies styles to UI components in the egui renderer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use egui;

use crate::sla_interpreter::{UiComponent, UiValue};
use super::style_engine::StyleEngine;
use super::style_interpreter::{StyleState, StyleValue};

pub struct StyleApplier {
    engine: Rc<RefCell<StyleEngine>>,
}

impl StyleApplier {
    pub fn new(engine: Rc<RefCell<StyleEngine>>) -> Self {
        Self { engine }
    }

    // Load styles from .st files only
    pub fn load_styles_from_directory(&mut self, styles_dir: &str) -> Result<(), String> {
        if !std::path::Path::new(styles_dir).exists() {
            return Ok(());
        }
        let entries = std::fs::read_dir(styles_dir)
            .map_err(|e| format!("Failed to read styles directory '{}': {}", styles_dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            // Only load .st files
            if path.extension().and_then(|ext| ext.to_str()) == Some("st") {
                let path_str = path.to_string_lossy();
                self.engine.borrow_mut().load_from_file(&path_str)?;
            }
        }
        Ok(())
    }

    // Load styles directly from .st file content
    pub fn load_styles_from_source(&mut self, source: &str) -> Result<(), String> {
        self.engine.borrow_mut().parse_style_blocks(source)
    }

    /// Resolve the computed styles for a component (used by devtools).
    pub fn resolve_styles(&self, component: &UiComponent) -> HashMap<String, StyleValue> {
        let mut classes = Vec::new();
        if let Some(UiValue::String(class_str)) = component.get_property("class") {
            classes = class_str.split_whitespace().map(|s| s.to_string()).collect();
        }
        self.engine.borrow().get_style_for_component(
            &component.component_type,
            component.identity.as_deref(),
            &classes,
            StyleState::Normal,
        )
    }

    pub fn apply_text_styles(&self, component: &UiComponent, ui: &mut egui::Ui, text: &str) {
        let styles = self.resolve_styles(component);

        // Check for text-outline style
        let has_outline = styles.get("text-outline").is_some() || 
                           styles.get("-webkit-text-stroke").is_some() ||
                           styles.get("text-stroke").is_some();

        if has_outline {
            self.render_outlined_text(ui, text, &styles);
        } else {
            let mut rich_text = egui::RichText::new(text);

            // color
            if let Some(StyleValue::Color(color)) = styles.get("color") {
                if let Some(c) = self.engine.borrow().parse_color(color) {
                    rich_text = rich_text.color(c);
                }
            } else if let Some(StyleValue::String(color)) = styles.get("color") {
                if let Some(c) = self.engine.borrow().parse_color(color) {
                    rich_text = rich_text.color(c);
                }
            }

            // font-size — stored as Number(f64) or Unit(f64, "px")
            rich_text = self.apply_font_size(rich_text, &styles);

            // font-weight bold
            if let Some(StyleValue::String(w)) = styles.get("font-weight") {
                if w == "bold" {
                    rich_text = rich_text.strong();
                }
            }

            // font-style italic
            if let Some(StyleValue::String(s)) = styles.get("font-style") {
                if s == "italic" {
                    rich_text = rich_text.italics();
                }
            }

            ui.label(rich_text);
        }
    }

    fn render_outlined_text(&self, ui: &mut egui::Ui, text: &str, styles: &HashMap<String, StyleValue>) {
        // Get outline color and width
        let outline_color = styles.get("text-outline-color")
            .or_else(|| styles.get("-webkit-text-stroke-color"))
            .or_else(|| styles.get("text-stroke-color"))
            .and_then(|v| match v {
                StyleValue::Color(c) => self.engine.borrow().parse_color(c),
                StyleValue::String(c) => self.engine.borrow().parse_color(c),
                _ => None,
            })
            .unwrap_or(egui::Color32::BLACK);

        let outline_width = styles.get("text-outline-width")
            .or_else(|| styles.get("-webkit-text-stroke-width"))
            .or_else(|| styles.get("text-stroke-width"))
            .and_then(|v| match v {
                StyleValue::Number(n) => Some(*n as f32),
                StyleValue::Unit(n, _) => Some(*n as f32),
                _ => None,
            })
            .unwrap_or(2.0);

        // Get fill color
        let fill_color = styles.get("color")
            .and_then(|v| match v {
                StyleValue::Color(c) => self.engine.borrow().parse_color(c),
                StyleValue::String(c) => self.engine.borrow().parse_color(c),
                _ => None,
            })
            .unwrap_or(egui::Color32::WHITE);

        // Get font size
        let font_size = self.get_font_size(styles);

        // Draw outline by rendering text at multiple offsets
        let offsets = [
            (-outline_width, -outline_width),
            (outline_width, -outline_width),
            (-outline_width, outline_width),
            (outline_width, outline_width),
            (0.0, -outline_width),
            (0.0, outline_width),
            (-outline_width, 0.0),
            (outline_width, 0.0),
        ];

        ui.horizontal(|ui| {
            // Use a fixed size for the text area
            let desired_size = egui::vec2(font_size * text.len() as f32 * 0.6, font_size * 1.5);
            let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

            // Draw outline layers
            for (dx, dy) in &offsets {
                let outline_rect = rect.translate(egui::vec2(*dx, *dy));
                ui.painter().text(
                    outline_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(font_size),
                    outline_color,
                );
            }

            // Draw fill text on top
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(font_size),
                fill_color,
            );
        });
    }

    fn get_font_size(&self, styles: &HashMap<String, StyleValue>) -> f32 {
        if let Some(StyleValue::Unit(n, unit)) = styles.get("font-size") {
            if unit == "px" {
                return *n as f32;
            }
        }
        if let Some(StyleValue::Number(n)) = styles.get("font-size") {
            return *n as f32;
        }
        if let Some(StyleValue::String(s)) = styles.get("font-size") {
            if let Ok(n) = s.parse::<f64>() {
                return n as f32;
            }
        }
        14.0 // default
    }

    pub fn apply_button_styles(&self, component: &UiComponent, ui: &mut egui::Ui) -> bool {
        let styles = self.resolve_styles(component);

        let label = component
            .get_property("label")
            .and_then(|v| if let UiValue::String(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_else(|| "Button".to_string());

        // Build the label as RichText so we can colour it
        let mut rich_label = egui::RichText::new(&label);

        // text color
        if let Some(StyleValue::Color(color)) = styles.get("color") {
            if let Some(c) = self.engine.borrow().parse_color(color) {
                rich_label = rich_label.color(c);
            }
        } else if let Some(StyleValue::String(color)) = styles.get("color") {
            if let Some(c) = self.engine.borrow().parse_color(color) {
                rich_label = rich_label.color(c);
            }
        }

        // font-size
        rich_label = self.apply_font_size(rich_label, &styles);

        // Build button
        let mut button = egui::Button::new(rich_label);

        // background-color
        if let Some(StyleValue::Color(color)) = styles.get("background-color") {
            if let Some(c) = self.engine.borrow().parse_color(color) {
                button = button.fill(c);
            }
        } else if let Some(StyleValue::String(color)) = styles.get("background-color") {
            if let Some(c) = self.engine.borrow().parse_color(color) {
                button = button.fill(c);
            }
        }

        // min-width / padding via sense area — egui doesn't expose padding directly,
        // but we can wrap in a sized container if needed in the future.

        ui.add(button).clicked()
    }

    fn apply_font_size(&self, mut text: egui::RichText, styles: &HashMap<String, StyleValue>) -> egui::RichText {
        match styles.get("font-size") {
            Some(StyleValue::Unit(size, _unit)) => {
                text = text.size(*size as f32);
            }
            Some(StyleValue::Number(size)) => {
                text = text.size(*size as f32);
            }
            _ => {}
        }
        text
    }

    pub fn apply_window_styles(&self, _component: &UiComponent, styles: &HashMap<String, StyleValue>, ui: &mut egui::Ui) {
        if let Some(StyleValue::Color(color)) = styles.get("background-color") {
            if let Some(c) = self.engine.borrow().parse_color(color) {
                ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, c);
            }
        }
    }

    fn apply_layout_styles(&self, _component: &UiComponent, styles: &HashMap<String, StyleValue>, ui: &mut egui::Ui) {
        if let Some(StyleValue::Unit(padding, _)) = styles.get("padding") {
            ui.add_space(*padding as f32);
        }
    }

    pub fn get_engine(&self) -> Rc<RefCell<StyleEngine>> {
        self.engine.clone()
    }
}