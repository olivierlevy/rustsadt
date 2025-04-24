// src/app.rs

// Imports nécessaires
use crate::generator::CodeGenerator; // On a besoin de CodeGenerator::new()
use crate::persistence::{load_diagram_dialog, save_diagram, save_diagram_dialog};
use crate::sadt_model::SadtDiagram;
use crate::ui::drawing::{draw_diagram, UiState};
use crate::ui::interaction::handle_canvas_interactions;
use eframe::egui;
use egui::{CentralPanel, Frame, Id, LayerId, Order, Pos2, Sense, TopBottomPanel, RichText};
use std::path::PathBuf; // Utilisé par current_file_path

// ------------ AJOUT : Définition de AppState ------------
pub struct AppState { // <<<<< AJOUTER pub
    pub diagram: SadtDiagram,
    pub ui_state: UiState,
    pub current_file_path: Option<PathBuf>,
    pub code_generator: Option<CodeGenerator>,
    pub generated_code: Option<String>,
    pub generated_doc: Option<String>,
}
// -------------------------------------------------------

// ------------ AJOUT : Définition de RustSadtApp ------------
pub struct RustSadtApp { // <<<<< AJOUTER pub
    state: AppState,
}
// ---------------------------------------------------------


impl Default for AppState { // <<<< Il faut que AppState soit défini avant
    fn default() -> Self {
        Self {
            diagram: SadtDiagram::new(),
            ui_state: UiState::default(),
            current_file_path: None,
            code_generator: None,
            generated_code: None,
            generated_doc: None,
        }
    }
}

impl Default for RustSadtApp { // <<<< Il faut que RustSadtApp et AppState soient définis avant
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}


impl RustSadtApp {
    // Méthode pour afficher les erreurs
     fn show_error_popup(&mut self, ctx: &egui::Context, error: &crate::error::RustSadtError) { // On a besoin de crate::error::RustSadtError ici
        egui::Window::new("Erreur")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("{}", error));
                if ui.button("Ok").clicked() { }
            });
        log::error!("Erreur applicative: {}", error);
    }

    // Méthode helper pour le générateur de code
    fn ensure_code_generator(&mut self, ctx: &egui::Context) -> Option<&CodeGenerator> {
        if self.state.code_generator.is_none() {
             match CodeGenerator::new() { // Référence à CodeGenerator::new
                 Ok(generator) => {
                     self.state.code_generator = Some(generator);
                     log::info!("CodeGenerator initialisé.");
                 }
                 Err(e) => {
                     self.show_error_popup(ctx, &e); // show_error_popup a besoin de RustSadtError
                     return None;
                 }
             }
        }
        self.state.code_generator.as_ref()
    }


    // --- Actions du menu ---
    fn file_new(&mut self) {
        self.state.diagram = SadtDiagram::new();
        self.state.ui_state = UiState::default();
        self.state.current_file_path = None;
        self.state.generated_code = None;
        self.state.generated_doc = None;
         log::info!("Nouveau diagramme créé.");
    }

    fn file_open(&mut self, ctx: &egui::Context) {
        match load_diagram_dialog() {
            Ok(Some((diagram, path))) => {
                self.state.diagram = diagram;
                self.state.current_file_path = Some(path);
                self.state.ui_state = UiState::default();
                self.state.generated_code = None;
                self.state.generated_doc = None;
            }
            Ok(None) => { /* User cancelled */ }
            Err(e) => self.show_error_popup(ctx, &e), // Référence à RustSadtError
        }
    }

    fn file_save(&mut self, ctx: &egui::Context) {
        if let Some(path) = &self.state.current_file_path.clone() {
            match save_diagram(&self.state.diagram, path) {
                Ok(()) => { /* Success */ }
                Err(e) => self.show_error_popup(ctx, &e), // Référence à RustSadtError
            }
        } else {
            self.file_save_as(ctx);
        }
    }

     fn file_save_as(&mut self, ctx: &egui::Context) {
        match save_diagram_dialog(&self.state.diagram) {
            Ok(Some(path)) => {
                self.state.current_file_path = Some(path);
            }
            Ok(None) => { /* User cancelled */ }
             Err(e) => self.show_error_popup(ctx, &e), // Référence à RustSadtError
        }
    }

    fn generate_code(&mut self, ctx: &egui::Context) {
        if self.ensure_code_generator(ctx).is_some() {
            let generator = self.state.code_generator.as_ref().unwrap();
            let diagram = &self.state.diagram;
            let module_name = self.state.current_file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("generated_module");

             match generator.generate_rust_module(diagram, module_name) {
                Ok(code) => {
                    log::info!("Code Rust généré avec succès.");
                    self.state.generated_code = Some(code);
                }
                Err(e) => self.show_error_popup(ctx, &e), // Référence à RustSadtError
            }
         }
     }

    fn generate_docs(&mut self, ctx: &egui::Context) {
        if self.ensure_code_generator(ctx).is_some() {
             let generator = self.state.code_generator.as_ref().unwrap();
             let diagram = &self.state.diagram;

             match generator.generate_markdown_doc(diagram) {
                Ok(doc) => {
                    log::info!("Documentation Markdown générée avec succès.");
                    self.state.generated_doc = Some(doc);
                }
                Err(e) => self.show_error_popup(ctx, &e), // Référence à RustSadtError
            }
         }
     }
}

// --- Implémentation du trait eframe::App ---
impl eframe::App for RustSadtApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Menu Bar ---
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                 ui.menu_button("Fichier", |ui| {
                    if ui.button("Nouveau").clicked() { self.file_new(); ui.close_menu(); }
                    if ui.button("Ouvrir...").clicked() { self.file_open(ctx); ui.close_menu(); }
                    if ui.button("Sauvegarder").clicked() { self.file_save(ctx); ui.close_menu(); }
                    if ui.button("Sauvegarder Sous...").clicked() { self.file_save_as(ctx); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Quitter").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
                 ui.menu_button("Edition", |ui| {
                    if ui.button("Annuler (Undo)").clicked() { /* TODO */ ui.close_menu();}
                    if ui.button("Rétablir (Redo)").clicked() { /* TODO */ ui.close_menu();}
                     ui.separator();
                     if ui.button("Ajouter Nœud").clicked() {
                          let pos = Pos2::new(300.0, 200.0);
                         self.state.diagram.add_node("Nouveau".to_string(), pos);
                         ui.close_menu();
                     }
                });
                 ui.menu_button("Générer", |ui| {
                    if ui.button("Générer Code Rust").clicked() { self.generate_code(ctx); ui.close_menu(); }
                    if ui.button("Générer Documentation Markdown").clicked() { self.generate_docs(ctx); ui.close_menu(); }
                });
            });
        });

        // --- Main Canvas ---
        CentralPanel::default()
            .frame(Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| {
                let _canvas_layer_id = LayerId::new(Order::Background, Id::new("sadt_canvas"));
                let painter = ui.painter_at(ui.max_rect());
                draw_diagram(&self.state.diagram, &painter, &self.state.ui_state);
                let response = ui.allocate_response(ui.available_size(), Sense::click_and_drag());
                // Passe ui qui est nécessaire pour les context menus et text edit
                handle_canvas_interactions(ctx, ui, &response, &mut self.state);
            });


        // --- Fenêtre Optionnelle pour Code Généré ---
        if let Some(code) = &self.state.generated_code {
             let mut is_open = true;
             egui::Window::new("Code Rust Généré")
                 .open(&mut is_open)
                 .default_width(600.0)
                 .default_height(400.0)
                 .show(ctx, |ui| {
                     egui::ScrollArea::vertical().show(ui, |ui| {
                         ui.label(RichText::new(code).monospace());
                     });
                 });
             if !is_open { self.state.generated_code = None; }
        }

        // --- Fenêtre Optionnelle pour Doc Générée ---
         if let Some(doc) = &self.state.generated_doc {
             let mut is_open = true;
             egui::Window::new("Documentation Markdown Générée")
                 .open(&mut is_open)
                 .default_width(600.0)
                 .default_height(400.0)
                 .show(ctx, |ui| {
                     egui::ScrollArea::vertical().show(ui, |ui| {
                         ui.label(RichText::new(doc).monospace());
                     });
                 });
             if !is_open { self.state.generated_doc = None; }
         }

        ctx.request_repaint();
    }
}