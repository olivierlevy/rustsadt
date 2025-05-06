// src/app.rs

// ------------ AJOUT/CORRECTION Imports ------------
use crate::generator::CodeGenerator;
use crate::persistence::{load_diagram_dialog, save_diagram, save_diagram_dialog};
use crate::sadt_model::SadtDiagram;
// Imports pour l'UI qui manquaient :
use crate::ui::drawing::{draw_diagram, UiState}; // Importe draw_diagram et UiState
use crate::ui::interaction::handle_canvas_interactions; // Importe handle_canvas_interactions
use eframe::egui; // Importe egui (nécessaire pour utiliser ses types via egui::...)
// Imports spécifiques d'egui (alternative à egui::...)
use egui::{
    CentralPanel, Frame, Id, LayerId, Order, Pos2, RichText, Sense, TopBottomPanel,
    ViewportCommand, // Ajout de ViewportCommand pour ctx.send_viewport_cmd
};
use std::path::PathBuf;
// Import pour RustSadtError si utilisé directement (ici dans show_error_popup)
use crate::error::RustSadtError;
// ----------------------------------------------------

// ------------ Définition de AppState ------------
// Note : PAS de #[derive(Default)] ici car on a une implémentation manuelle plus bas
pub struct AppState {
    pub diagram: SadtDiagram,
    pub ui_state: UiState, // Utilise UiState importé
    pub current_file_path: Option<PathBuf>,
    pub code_generator: Option<CodeGenerator>,
    pub generated_code: Option<String>,
    pub generated_doc: Option<String>,
}
// -------------------------------------------------------

// ------------ Définition de RustSadtApp ------------
pub struct RustSadtApp {
    state: AppState, // Utilise AppState défini ci-dessus
}
// ---------------------------------------------------------

// --- Implémentations Default ---
impl Default for AppState {
    fn default() -> Self {
        Self {
            diagram: SadtDiagram::new(),
            ui_state: UiState::default(), // Utilise UiState::default()
            current_file_path: None,
            code_generator: None,
            generated_code: None,
            generated_doc: None,
        }
    }
}

impl Default for RustSadtApp {
    fn default() -> Self {
        Self {
            state: AppState::default(), // Utilise AppState::default()
        }
    }
}

// --- Implémentations des méthodes pour RustSadtApp ---
impl RustSadtApp {
    // Méthode pour afficher les erreurs
     fn show_error_popup(&mut self, ctx: &egui::Context, error: &RustSadtError) { // Utilise RustSadtError importé
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
             match CodeGenerator::new() {
                 Ok(generator) => {
                     self.state.code_generator = Some(generator);
                     log::info!("CodeGenerator initialisé.");
                 }
                 Err(e) => {
                     self.show_error_popup(ctx, &e);
                     return None;
                 }
             }
        }
        self.state.code_generator.as_ref()
    }

    // --- Actions du menu ---
    fn file_new(&mut self) {
        self.state.diagram = SadtDiagram::new();
        self.state.ui_state = UiState::default(); // Utilise UiState::default()
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
                self.state.ui_state = UiState::default(); // Utilise UiState::default()
                self.state.generated_code = None;
                self.state.generated_doc = None;
                // Correction: Accéder au chemin via self.state pour le log
                if let Some(p) = &self.state.current_file_path {
                    log::info!("Diagramme chargé depuis: {}", p.display());
                }
            }
            Ok(None) => { log::info!("Ouverture annulée par l'utilisateur."); }
            Err(e) => {
                log::error!("Erreur lors du chargement: {}", e);
                self.show_error_popup(ctx, &e);
            }
        }
    }

    fn file_save(&mut self, ctx: &egui::Context) {
        if let Some(path) = &self.state.current_file_path.clone() {
            log::info!("Tentative de sauvegarde vers: {}", path.display());
            match save_diagram(&self.state.diagram, path) {
                Ok(()) => { log::info!("Diagramme sauvegardé avec succès."); }
                Err(e) => {
                    log::error!("Erreur lors de la sauvegarde: {}", e);
                    self.show_error_popup(ctx, &e);
                }
            }
        } else {
            log::info!("Aucun fichier courant, appel de Sauvegarder Sous...");
            self.file_save_as(ctx);
        }
    }

     fn file_save_as(&mut self, ctx: &egui::Context) {
        match save_diagram_dialog(&self.state.diagram) {
            Ok(Some(path)) => {
                self.state.current_file_path = Some(path.clone());
                log::info!("Diagramme sauvegardé (sous...) dans: {}", path.display());
            }
            Ok(None) => { log::info!("Sauvegarde sous... annulée par l'utilisateur."); }
             Err(e) => {
                 log::error!("Erreur lors de la sauvegarde sous...: {}", e);
                 self.show_error_popup(ctx, &e);
             }
        }
    }

    fn generate_code(&mut self, ctx: &egui::Context) {
        if self.ensure_code_generator(ctx).is_some() {
            log::debug!("CodeGenerator obtenu, tentative de génération de code...");
            let generator = self.state.code_generator.as_ref().unwrap();
            let diagram = &self.state.diagram;
            let module_name = self.state.current_file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("generated_module");

             log::info!("Génération du code Rust pour le module: {}", module_name);
             match generator.generate_rust_module(diagram, module_name) {
                Ok(code) => {
                    log::info!("Code Rust généré avec succès.");
                    self.state.generated_code = Some(code);
                }
                Err(e) => {
                     log::error!("Erreur lors de la génération du code Rust: {}", e);
                     self.show_error_popup(ctx, &e);
                }
            }
         } else {
            log::warn!("Impossible d'obtenir CodeGenerator pour générer le code.");
         }
     }

    fn generate_docs(&mut self, ctx: &egui::Context) {
        if self.ensure_code_generator(ctx).is_some() {
             log::debug!("CodeGenerator obtenu, tentative de génération de doc...");
             let generator = self.state.code_generator.as_ref().unwrap();
             let diagram = &self.state.diagram;

             log::info!("Génération de la documentation Markdown...");
             match generator.generate_markdown_doc(diagram) {
                Ok(doc) => {
                    log::info!("Documentation Markdown générée avec succès.");
                    self.state.generated_doc = Some(doc);
                }
                Err(e) => {
                     log::error!("Erreur lors de la génération de la documentation: {}", e);
                     self.show_error_popup(ctx, &e);
                }
            }
         } else {
             log::warn!("Impossible d'obtenir CodeGenerator pour générer la doc.");
         }
     }
     // Action pour exporter en SVG
     fn file_export_svg(&mut self, ctx: &egui::Context) {
         log::info!("Début export SVG...");
         match crate::persistence::export_svg_dialog(&self.state.diagram) {
             Ok(Some(path)) => {
                 log::info!("Export SVG réussi vers: {}", path.display());
             }
             Ok(None) => {
                  log::info!("Export SVG annulé.");
             }
             Err(e) => {
                  log::error!("Erreur lors de l'export SVG: {}", e);
                 self.show_error_popup(ctx, &e);
             }
         }
     }
}

// --- Implémentation du trait eframe::App ---
impl eframe::App for RustSadtApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // --- Menu Bar ---
        TopBottomPanel::top("top_panel").show(ctx, |ui| { // Utilise TopBottomPanel importé
            egui::menu::bar(ui, |ui| {
                 ui.menu_button("Fichier", |ui| {
                    if ui.button("Nouveau").clicked() { self.file_new(); ui.close_menu(); }
                    if ui.button("Ouvrir...").clicked() { self.file_open(ctx); ui.close_menu(); }
                    if ui.button("Sauvegarder").clicked() { self.file_save(ctx); ui.close_menu(); }
                    if ui.button("Sauvegarder Sous...").clicked() { self.file_save_as(ctx); ui.close_menu(); }
                    ui.separator(); // Séparateur avant export
                    if ui.button("Exporter SVG...").clicked() {
                        self.file_export_svg(ctx); // Appel de la nouvelle fonction
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quitter").clicked() { ctx.send_viewport_cmd(ViewportCommand::Close); } // Utilise ViewportCommand importé
                });
                 ui.menu_button("Edition", |ui| {
                    if ui.button("Annuler (Undo)").clicked() { log::warn!("Undo non implémenté"); ui.close_menu();}
                    if ui.button("Rétablir (Redo)").clicked() { log::warn!("Redo non implémenté"); ui.close_menu();}
                     ui.separator();
                     if ui.button("Ajouter Nœud").clicked() {
                          let pos = Pos2::new(200.0, 150.0); // Utilise Pos2 importé
                         let node_name = format!("Activité {}", self.state.diagram.nodes.len() + 1);
                         self.state.diagram.add_node(node_name, pos);
                         log::info!("Nœud ajouté.");
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
        CentralPanel::default() // Utilise CentralPanel importé
            .frame(Frame::dark_canvas(&ctx.style())) // Utilise Frame importé
            .show(ctx, |ui| {
                let _canvas_layer_id = LayerId::new(Order::Background, Id::new("sadt_canvas")); // Utilise LayerId, Order, Id importés
                let painter = ui.painter_at(ui.max_rect());
                draw_diagram(&self.state.diagram, &painter, &self.state.ui_state); // Utilise draw_diagram et ui_state importés
                let response = ui.allocate_response(ui.available_size(), Sense::click_and_drag()); // Utilise Sense importé
                handle_canvas_interactions(ctx, ui, &response, &mut self.state); // Utilise handle_canvas_interactions importé
            });


        // --- Fenêtres Optionnelles ---
        if let Some(code) = &self.state.generated_code {
             let mut is_open = true;
             egui::Window::new("Code Rust Généré")
                 .open(&mut is_open)
                 .default_width(600.0)
                 .default_height(400.0)
                 .show(ctx, |ui| {
                     egui::ScrollArea::vertical().show(ui, |ui| {
                         ui.label(RichText::new(code).monospace()); // Utilise RichText importé
                     });
                 });
             if !is_open { self.state.generated_code = None; }
        }
        if let Some(doc) = &self.state.generated_doc {
             let mut is_open = true;
             egui::Window::new("Documentation Markdown Générée")
                 .open(&mut is_open)
                 .default_width(600.0)
                 .default_height(400.0)
                 .show(ctx, |ui| {
                     egui::ScrollArea::vertical().show(ui, |ui| {
                         ui.label(RichText::new(doc).monospace()); // Utilise RichText importé
                     });
                 });
             if !is_open { self.state.generated_doc = None; }
         }

        ctx.request_repaint();
    }
}