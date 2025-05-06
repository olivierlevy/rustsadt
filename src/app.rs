// src/app.rs

// Imports nécessaires
use crate::generator::CodeGenerator;
use crate::persistence::{load_diagram_dialog, save_diagram, save_diagram_dialog};
use crate::sadt_model::SadtDiagram;
use crate::ui::drawing::UiState; // Importe UiState
// Importe les fonctions/types nécessaires pour l'UI et les interactions
use crate::ui::interaction::handle_canvas_interactions;
use eframe::egui;
use egui::{
    Vec2, CentralPanel, Frame, Pos2, RichText, Sense, TopBottomPanel,
    PointerButton, // Ajout pour Pan
    ViewportCommand,
};
use std::path::PathBuf;
use crate::error::RustSadtError; // Pour show_error_popup

// ------------ Définition de AppState ------------
pub struct AppState {
    pub diagram: SadtDiagram,
    pub ui_state: UiState,
    pub current_file_path: Option<PathBuf>,
    pub code_generator: Option<CodeGenerator>,
    pub generated_code: Option<String>,
    pub generated_doc: Option<String>,
    pub zoom: f32,           // Niveau de zoom
    pub pan: Vec2,           // Décalage de la vue (en coordonnées monde)
}
// -------------------------------------------------------

// ------------ Définition de RustSadtApp ------------
pub struct RustSadtApp {
    state: AppState,
}
// ---------------------------------------------------------

// --- Implémentations Default ---
impl Default for AppState { // Implémentation manuelle conservée
    fn default() -> Self {
        Self {
            diagram: SadtDiagram::new(),
            ui_state: UiState::default(),
            current_file_path: None,
            code_generator: None,
            generated_code: None,
            generated_doc: None,
            zoom: 1.0,       // Zoom initial
            pan: Vec2::ZERO, // Pas de décalage initial
        }
    }
}

impl Default for RustSadtApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

// --- Implémentations des méthodes pour RustSadtApp ---
impl RustSadtApp {
    // Méthode pour afficher les erreurs
     fn show_error_popup(&mut self, ctx: &egui::Context, error: &RustSadtError) {
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
                     self.show_error_popup(ctx, &e.into()); // Assurer conversion en RustSadtError si nécessaire
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
        self.state.zoom = 1.0; // Réinitialiser zoom/pan aussi
        self.state.pan = Vec2::ZERO;
         log::info!("Nouveau diagramme créé.");
    }

    fn file_open(&mut self, ctx: &egui::Context) {
        match load_diagram_dialog() {
            Ok(Some((diagram, path))) => {
                self.state.diagram = diagram;
                self.state.current_file_path = Some(path.clone()); // Cloner car path est utilisé dans le log
                self.state.ui_state = UiState::default();
                self.state.generated_code = None;
                self.state.generated_doc = None;
                 self.state.zoom = 1.0; // Réinitialiser zoom/pan
                 self.state.pan = Vec2::ZERO;
                log::info!("Diagramme chargé depuis: {}", path.display());
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
                    log::error!("Erreur DANS generate_rust_module: {}", e);
                    self.show_error_popup(ctx, &e.into()); // Assurer conversion en RustSadtError
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
                    log::error!("Erreur DANS generate_markdown_doc: {}", e);
                     self.show_error_popup(ctx, &e.into()); // Assurer conversion en RustSadtError
                }
            }
        } else {
            log::warn!("Impossible d'obtenir CodeGenerator pour générer la doc.");
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
                    if ui.button("Exporter SVG...").clicked() { self.file_export_svg(ctx); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Quitter").clicked() { ctx.send_viewport_cmd(ViewportCommand::Close); }
                });
                 ui.menu_button("Edition", |ui| {
                    if ui.button("Annuler (Undo)").clicked() { log::warn!("Undo non implémenté"); ui.close_menu();}
                    if ui.button("Rétablir (Redo)").clicked() { log::warn!("Redo non implémenté"); ui.close_menu();}
                     ui.separator();
                     if ui.button("Ajouter Nœud").clicked() {
                          // Position ajout via menu: pour l'instant fixe dans le monde visible initial
                          // Idéalement, utiliser le centre de la vue actuelle transformé en monde
                          let pos_monde_vec = self.state.pan + egui::vec2(200.0, 150.0) / self.state.zoom; // Approximation Vec2
                          let pos_monde = Pos2::new(pos_monde_vec.x, pos_monde_vec.y); // <<< Conversion Vec2 -> Pos2
                         let node_name = format!("Activité {}", self.state.diagram.nodes.len() + 1);
                         self.state.diagram.add_node(node_name, pos_monde);
                         log::info!("Nœud ajouté via menu.");
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
                // Calculer transformation
                let canvas_rect = ui.clip_rect(); // Utiliser clip_rect pour la zone visible
                let screen_center = canvas_rect.center();
                let transform = crate::ui::transform::Transform::new(
                    self.state.pan,
                    self.state.zoom,
                );

                let painter = ui.painter_at(canvas_rect);

                // Dessiner
                crate::ui::drawing::draw_diagram(
                    &self.state.diagram,
                    &transform,
                    &painter,
                    &self.state.ui_state
                );

                // Allouer réponse pour interactions
                let response = ui.allocate_response(canvas_rect.size(), Sense::click_and_drag());

                // --- Gestion Zoom ---
                let scroll = ctx.input(|i| i.raw_scroll_delta);
                if response.hovered() && scroll.y != 0.0 {
                    let old_zoom = self.state.zoom;
                    let zoom_delta_factor = (scroll.y * 0.005).exp(); // Utilisez un facteur plus petit pour un zoom moins rapide, ex: 0.005
                    let new_zoom = (old_zoom * zoom_delta_factor).clamp(0.05, 20.0); // Augmenter la limite min pour éviter zoom trop petit

                    // Recalculer transform avec l'ANCIEN zoom pour trouver le point monde sous le curseur
                    let screen_center = response.rect.center(); // Utiliser le rect de la réponse (canvas)
                    let old_transform = crate::ui::transform::Transform::new(self.state.pan, old_zoom);

                    if let Some(hover_pos_screen) = response.hover_pos() {
                        let pivot_world = old_transform.screen_to_world(hover_pos_screen);

                        // Calcul du nouveau pan pour garder pivot_world sous hover_pos_screen avec new_zoom
                        // Formule: new_pan = pivot_world.to_vec2() - screen_delta / new_zoom
                        // où screen_delta est la position du curseur relative au coin haut-gauche écran (0,0)
                        // car screen_to_world simplifié utilise pan comme coord monde sous (0,0) écran.
                        let new_pan_vec = pivot_world.to_vec2() - hover_pos_screen.to_vec2() / new_zoom;

                        // Mettre à jour l'état SEULEMENT APRÈS les calculs
                        self.state.zoom = new_zoom;
                        self.state.pan = new_pan_vec;

                        log::debug!("Zoom: {:.3}, Pan: {:?}, PivotW: {:?}, CursorS: {:?}",
                            self.state.zoom, self.state.pan, pivot_world, hover_pos_screen);

                    } else {
                        // Si pas de curseur, zoom par rapport au centre (simple mise à l'échelle)
                        // Le pan ne change pas dans ce cas avec la formule simplifiée
                        self.state.zoom = new_zoom;
                        log::debug!("Zoom (center): {:.3}", self.state.zoom);
                    }
                    log::debug!("Zoom: {:.2}, Pan: {:?}", self.state.zoom, self.state.pan);
                }

                // --- Gestion Pan ---
                let pan_button = PointerButton::Middle; // Clic molette pour panner
                if response.dragged_by(pan_button) {
                     // Recalculer transform avec le zoom potentiellement mis à jour juste avant
                     let current_transform = crate::ui::transform::Transform::new(
                         self.state.pan, self.state.zoom
                     );
                     let delta_world = current_transform.screen_vec_to_world(response.drag_delta());
                     self.state.pan -= delta_world;
                     log::trace!("Panning by world delta {:?}", -delta_world);
                     // Optionnel: changer le curseur pendant le pan
                     // ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                }

                // Gérer les interactions (passe la transformation recalculée)
                 let final_transform = crate::ui::transform::Transform::new(
                    self.state.pan, self.state.zoom
                 );
                handle_canvas_interactions(
                    ctx,
                    ui, // Passer ui pour les menus/fenêtres contextuelles
                    &response,
                    &final_transform, // Passer la transformation finale
                    &mut self.state
                );
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
                         ui.label(RichText::new(code).monospace());
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
                         ui.label(RichText::new(doc).monospace());
                     });
                 });
             if !is_open { self.state.generated_doc = None; }
         }

        ctx.request_repaint(); // Important pour que le pan/zoom soit fluide
    }
}