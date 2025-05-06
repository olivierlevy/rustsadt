use crate::app::AppState; // On aura besoin de l'état global
use crate::sadt_elements::{ArrowType, Side};
use crate::ui::drawing::{find_closest_connection_point, get_connection_pos, CONNECTION_POINT_RADIUS};
use egui::{Context, Key, PointerButton, Pos2, Ui, Response, Vec2};

const DRAG_THRESHOLD: f32 = 5.0; // Distance minimale pour commencer un drag
const ARROW_SELECT_DISTANCE: f32 = 5.0; // Tolérance pour sélectionner une flèche

// Helper: Calcule la distance² d'un point à un segment de ligne
fn distance_sq_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let l2 = a.distance_sq(b);
    if l2 == 0.0 { return p.distance_sq(a); } // Segment de longueur nulle
    // Considère la projection du point p sur la ligne (a, b)
    // t = [(p - a) . (b - a)] / |b - a|^2
    let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
    let t = t.clamp(0.0, 1.0); // Limite t entre 0 et 1 pour rester sur le segment
    // Point projeté sur la ligne = a + t * (b - a)
    let projection = a + (b - a) * t;
    p.distance_sq(projection)
}

pub fn handle_canvas_interactions(
    ctx: &Context,
    ui: &mut Ui,
    response: &egui::Response, // Réponse du CentralPanel où l'on dessine
    app_state: &mut AppState, // Accès mutable au modèle et à l'état UI
) {
    app_state.ui_state.mouse_pos = response.hover_pos().unwrap_or_default(); // Mémoriser la position souris

    handle_node_drag_and_select(ctx, response, app_state);
    handle_arrow_creation(ctx, response, app_state);
    handle_arrow_selection(ctx, response, app_state);
    handle_rename(ctx, app_state);
    handle_deletion(ctx, app_state);

    // Clic droit pour le menu contextuel (pas encore implémenté ici)
    response.context_menu(|ui| {
        // Menu pour Nœud sélectionné
        if let Some(node_id) = app_state.ui_state.selected_node {
            if ui.button("Renommer Nœud").clicked() {
                 app_state.ui_state.renaming_node = Some(node_id);
                 // Pré-remplir le texte avec le nom actuel
                 if let Some(node) = app_state.diagram.get_node(node_id) {
                    app_state.ui_state.renaming_label_text = node.name.clone();
                 } else {
                    app_state.ui_state.renaming_label_text = String::new();
                 }
                 app_state.ui_state.renaming_arrow = None; // Désactiver renommage flèche
                 ui.close_menu();
            }
            ui.separator();
            if ui.button("Supprimer Nœud").clicked() {
                if let Some(id) = app_state.ui_state.selected_node.take() { // Take pour consommer l'option
                    app_state.diagram.remove_node(id); // Supprime le nœud et les flèches connectées
                    log::info!("Nœud {} supprimé via menu contextuel", id);
                }
                ui.close_menu();
            }
        }
        // Menu pour Flèche sélectionnée
        else if let Some(arrow_id) = app_state.ui_state.selected_arrow {
            if ui.button("Editer Label Flèche").clicked() {
                app_state.ui_state.renaming_arrow = Some(arrow_id);
                 // Pré-remplir le texte avec le label actuel ou vide
                 if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
                    app_state.ui_state.renaming_label_text = arrow.label.clone().unwrap_or_default();
                 } else {
                    app_state.ui_state.renaming_label_text = String::new();
                 }
                app_state.ui_state.renaming_node = None; // Désactiver renommage nœud
                ui.close_menu();
            }
             ui.separator();
            if ui.button("Supprimer Flèche").clicked() {
                 if let Some(id) = app_state.ui_state.selected_arrow.take() {
                    app_state.diagram.remove_arrow(id);
                    log::info!("Flèche {} supprimée via menu contextuel", id);
                 }
                ui.close_menu();
            }
        } else {
            // Menu pour Canvas (rien de sélectionné)
            if ui.button("Ajouter Nœud").clicked() {
                // Utiliser la position où le menu contextuel a été ouvert
                let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_else(|| response.rect.center());
                let node_name = format!("Activité {}", app_state.diagram.nodes.len() + 1);
                app_state.diagram.add_node(node_name, pos);
                log::info!("Nœud ajouté via menu contextuel à {:?}", pos);
                ui.close_menu();
            }
        }
    });
}

fn handle_node_drag_and_select(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    // Détection début de drag ou sélection
    if response.hovered() && pointer.button_pressed(PointerButton::Primary) {
        let click_pos = pointer.press_origin().unwrap_or_default();
        let mut clicked_on_node = None;

        // Trouver si on a cliqué sur un nœud (ordre inverse du dessin pour le z-index)
        for node in app_state.diagram.nodes.values() {
            if node.rect.contains(click_pos) {
                clicked_on_node = Some(node.id);
                break; // On prend le premier trouvé (le plus "haut")
            }
        }

        if let Some(node_id) = clicked_on_node {
            app_state.ui_state.selected_node = Some(node_id);
            app_state.ui_state.selected_arrow = None;
            // Marquer pour potentiel drag
            ctx.set_dragged_id(response.id);

        } else {
             // Seulement désélectionner si on n'a pas cliqué sur une flèche (vérifié dans handle_arrow_selection)
             // Clic dans le vide : désélectionner
             // Retiré d'ici, géré dans handle_arrow_selection ou si on clique vraiment dans le vide
             // app_state.ui_state.renaming_node = None; // Arrêter le renommage si on clique ailleurs -> Géré dans handle_rename
        }
    }

    // Gestion du drag en cours
    if pointer.button_down(PointerButton::Primary) && ctx.is_being_dragged(response.id) {
       if let Some(node_id) = app_state.ui_state.selected_node {
           if let Some(node) = app_state.diagram.get_node_mut(node_id) {
                node.rect = node.rect.translate(pointer.delta());
           }
           // S'assurer qu'on est bien en mode drag (évite de créer flèche en même temps)
           app_state.ui_state.arrow_creation_start = None;
       }
    }

    // Fin du drag
    if ctx.input(|i| i.pointer.any_released()) {
        ctx.stop_dragging();
    }
}
// Gérer la sélection des flèches
fn handle_arrow_selection(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());

    // Détection clic pour sélection de flèche (seulement si on n'a pas cliqué sur un nœud)
    if response.hovered()
        && pointer.button_pressed(PointerButton::Primary)
        && app_state.ui_state.selected_node.is_none() // Ne pas sélectionner flèche si on vient de sélectionner un nœud
        && !ctx.is_being_dragged(response.id) // Et qu'on ne drague pas un nœud
        && app_state.ui_state.arrow_creation_start.is_none() // Et qu'on ne crée pas une flèche
    {
        let click_pos = pointer.press_origin().unwrap_or_default();
        let mut clicked_on_arrow = None;

        for arrow in app_state.diagram.arrows.values() {
            if let (Some(src_node), Some(tgt_node)) = (
                app_state.diagram.get_node(arrow.source.node_id),
                app_state.diagram.get_node(arrow.target.node_id),
            ) {
                let start_pos = get_connection_pos(src_node, arrow.source.side);
                let end_pos = get_connection_pos(tgt_node, arrow.target.side);
                if distance_sq_to_segment(click_pos, start_pos, end_pos) < ARROW_SELECT_DISTANCE.powi(2) {
                    clicked_on_arrow = Some(arrow.id);
                    break;
                }
            }
        }

        if let Some(arrow_id) = clicked_on_arrow {
            app_state.ui_state.selected_arrow = Some(arrow_id);
            app_state.ui_state.selected_node = None; // Désélectionner nœud
            log::debug!("Flèche sélectionnée: {}", arrow_id);
        } else {
            // Clic dans le vide (ni nœud ni flèche) : tout désélectionner
            app_state.ui_state.selected_node = None;
            app_state.ui_state.selected_arrow = None;
            // Arrêter le renommage si on clique dans le vide est géré dans handle_rename
        }
    }
}

fn handle_arrow_creation(ctx: &Context, response: &egui::Response, app_state: &mut AppState) {
    let pointer = &ctx.input(|i| i.pointer.clone());
    let mouse_pos = app_state.ui_state.mouse_pos;

    // --- Démarrage ---
    // Condition ajoutée : && app_state.ui_state.selected_node.is_none()
    // Si on clique pour sélectionner un noeud, on ne veut PAS démarrer une flèche.
    // Mais si on clique sur un point de connexion d'un noeud NON sélectionné, ça devrait marcher.
    // Revoyons cette condition. Le problème est peut-être que response.hovered() est trop général.
    // On veut détecter le clic *sur* un point de connexion.
    if pointer.button_pressed(PointerButton::Primary) { // Démarrer sur clic primaire
        if let Some(start_point) = find_closest_connection_point(
            &app_state.diagram,
            pointer.press_origin().unwrap_or_default(),
            CONNECTION_POINT_RADIUS * 3.0, // <<< Augmenter un peu la tolérance initiale
        ) {
             // Ne démarrer QUE si on ne drague pas déjà un noeud
             // (ctx.dragged_id().is_none() vérifie ça globalement, mais
             // is_being_dragged(response.id) est plus spécifique au canvas)
             // Vérifions si on drague le canvas LUI-MEME (pas un ID spécifique de noeud/flèche)
            if !ctx.is_being_dragged(response.id) {
                 app_state.ui_state.arrow_creation_start = Some(start_point.clone());
                 // Marquer le contexte comme potentiellement en train de dragger (pour la ligne de prévisualisation)
                 // N'utilisons PAS set_dragged_id ici, car cela pourrait interférer avec le drag de noeuds.
                 // Le simple fait que arrow_creation_start soit Some(...) suffit pour dessiner la prévisualisation.
                 // ctx.set_dragged_id(response.id.with("arrow_drag")); // <<< Retirer cette ligne
                 log::debug!("Début création flèche depuis: {:?}", start_point);
            } else {
                 log::trace!("Ignoré début flèche: drag de noeud en cours");
            }
        }
    }

    // --- Fin ou Annulation ---
    // Utiliser l'emprunt (&) pour ne pas déplacer la valeur hors de l'option
    if let Some(start_point) = &app_state.ui_state.arrow_creation_start {
        // Vérifier si le bouton est relâché
        if pointer.any_released() {
             log::debug!("Tentative fin création flèche à: {:?}", mouse_pos);
             // Relâché: vérifier si c'est sur un point de connexion cible
             if let Some(end_point) = find_closest_connection_point(
                &app_state.diagram,
                mouse_pos, // Utiliser la position actuelle de la souris au relâchement
                CONNECTION_POINT_RADIUS * 4.0, // Tolérance plus grande au relâchement
            ) {
                 // Vérifier si la cible est différente de la source
                 // et que la cible existe bien (redondant mais sûr)
                if start_point.node_id != end_point.node_id && app_state.diagram.nodes.contains_key(&end_point.node_id) {
                    // Déterminer le type de flèche (logique simplifiée actuelle)
                     let arrow_type = match (start_point.side, end_point.side) {
                         (Side::Right, Side::Left) => ArrowType::Output,
                         (Side::Left, Side::Right) => ArrowType::Input,
                         (_, Side::Top) => ArrowType::Control,
                         (_, Side::Bottom) => ArrowType::Mechanism,
                         // Cas par défauts
                         (Side::Right, _) => ArrowType::Output,
                         (_, Side::Left) => ArrowType::Input,
                         _ => ArrowType::Input,
                    };

                    log::debug!("Fin création flèche vers: {:?}, type: {:?}", end_point, arrow_type);
                    // Ajouter la flèche (cloner start_point car il est emprunté)
                    app_state.diagram.add_arrow(start_point.clone(), end_point, arrow_type, None);
                } else {
                    log::debug!("Annulation flèche: connexion au même noeud ou noeud cible invalide.");
                }
            } else {
                 log::debug!("Annulation flèche: relâchement dans le vide.");
            }
            // --- Très Important: Réinitialiser DANS TOUS LES CAS après relâchement ---
            app_state.ui_state.arrow_creation_start = None;
            // ctx.stop_dragging(); // On ne fait plus de drag spécifique pour la flèche
        }
         // Si on n'a pas relâché, la boucle continue, et la ligne de prévisualisation est dessinée
         // par `draw_diagram` car `arrow_creation_start` est Some(...).
    }
}

fn handle_rename(ctx: &Context, app_state: &mut AppState) {
    // Renommage Nœud
    if let Some(node_id) = app_state.ui_state.renaming_node {
        if let Some(node) = app_state.diagram.get_node(node_id) {
             // Utiliser le texte stocké dans ui_state
             let node_rect = node.rect; // Copier rect avant d'emprunter mut app_state

            // Créer une fenêtre flottante pour l'édition
             egui::Window::new("Renommer Nœud")
                .collapsible(false)
                .resizable(false)
                // .title_bar(false) // Gardons la barre pour pouvoir la bouger au besoin
                .anchor(egui::Align2::CENTER_CENTER, node_rect.center_bottom() - node_rect.center()) // Positionner au-dessus/dessous
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                    // ... request_focus ...
                
                    // --- Logique de validation/annulation corrigée ---
                    let mut close_rename_window = false;
                    let mut rename_successful = false;
                
                    // Valider sur Entrée
                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        // Si on perd le focus ET appuie sur Entrée (cas où on clique sur un bouton hors de la fenêtre par ex)
                         rename_successful = true;
                         close_rename_window = true;
                    } else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         // Si on appuie sur Entrée SANS perdre le focus
                         rename_successful = true;
                         close_rename_window = true;
                         // Il faut "consommer" l'événement Entrée pour éviter qu'il ne soit traité ailleurs
                         // Cependant, egui ne fournit pas de moyen simple de le faire ici.
                         // Alternative: fermer la fenêtre retire le focus.
                    }
                     // Annuler sur Echap
                     else if ui.input(|i| i.key_pressed(Key::Escape)) {
                         close_rename_window = true;
                         rename_successful = false; // Annulation explicite
                     }
                     // Annuler si on perd le focus SANS avoir validé avec Entrée (clic extérieur)
                     else if text_edit_response.lost_focus() {
                          close_rename_window = true;
                          rename_successful = false; // Considéré comme une annulation
                          log::debug!("Renommage annulé par perte de focus.");
                     }
                
                
                    if close_rename_window {
                        if rename_successful {
                            if let Some(node_mut) = app_state.diagram.get_node_mut(node_id) {
                                 node_mut.name = app_state.ui_state.renaming_label_text.clone();
                                 log::info!("Nœud {} renommé en '{}'", node_id, node_mut.name);
                            }
                        } else {
                            log::info!("Renommage Nœud {} annulé.", node_id);
                            // Pas besoin de restaurer le texte, on efface juste le buffer temporaire
                        }
                        app_state.ui_state.renaming_node = None;
                        app_state.ui_state.renaming_label_text.clear(); // Vider le buffer
                        // ui.close_menu(); // Pas dans une fenêtre egui
                    }
                }); // Fin .show()

            // Si la fenêtre d'édition n'est plus ouverte (par ex. clic extérieur), arrêter le renommage
            // Note: egui gère cela implicitement si on utilise `id_source` mais ici on contrôle manuellement
             // On utilise le fait que si on clique ailleurs, le node_id est désélectionné/changé
        } else {
            // Le noeud a été supprimé pendant qu'on le renommait ?
            app_state.ui_state.renaming_node = None;
        }
    }
    // Renommage Flèche
    else if let Some(arrow_id) = app_state.ui_state.renaming_arrow {
         if let Some(arrow) = app_state.diagram.get_arrow(arrow_id) {
             // Calculer la position du label (milieu de la flèche)
             let mid_pos = if let (Some(src_node), Some(tgt_node)) = (
                 app_state.diagram.get_node(arrow.source.node_id),
                 app_state.diagram.get_node(arrow.target.node_id),
             ) {
                 let start_pos = get_connection_pos(src_node, arrow.source.side);
                 let end_pos = get_connection_pos(tgt_node, arrow.target.side);
                 start_pos.lerp(end_pos, 0.5)
             } else {
                 Pos2::ZERO // Fallback si nœuds non trouvés
             };

             // Créer une fenêtre flottante pour l'édition
             egui::Window::new("Editer Label Flèche")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, mid_pos - Pos2::ZERO) // Positionner près du milieu
                .show(ctx, |ui| {
                    let text_edit_response = ui.text_edit_singleline(&mut app_state.ui_state.renaming_label_text);
                     // ... request_focus ...
                
                    // --- Logique de validation/annulation corrigée (identique à celle pour les nœuds) ---
                    let mut close_rename_window = false;
                    let mut rename_successful = false;
                
                    if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if !text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                         rename_successful = true;
                         close_rename_window = true;
                    } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                         close_rename_window = true;
                         rename_successful = false;
                     } else if text_edit_response.lost_focus() {
                          close_rename_window = true;
                          rename_successful = false;
                          log::debug!("Edition label annulée par perte de focus.");
                     }
                
                
                    if close_rename_window {
                        if rename_successful {
                            if let Some(arrow_mut) = app_state.diagram.arrows.get_mut(&arrow_id) {
                                 let new_label = app_state.ui_state.renaming_label_text.clone();
                                 arrow_mut.label = if new_label.is_empty() { None } else { Some(new_label) };
                                 log::info!("Label flèche {} édité en '{:?}'", arrow_id, arrow_mut.label);
                            }
                        } else {
                             log::info!("Edition label flèche {} annulée.", arrow_id);
                        }
                        app_state.ui_state.renaming_arrow = None;
                        app_state.ui_state.renaming_label_text.clear();
                    }
                }); // Fin .show()
        } else {
            // Flèche supprimée ?
             app_state.ui_state.renaming_arrow = None;
        }
    }
    // Détection Double-clic pour démarrer renommage (NŒUD SEULEMENT pour l'instant)
    // Le double-clic sur flèche pourrait être ambigu. On utilise le menu contextuel.
    else {
        // Détecter double-clic pour démarrer le renommage
        // Attention: response n'est pas disponible ici, il faut le passer en argument si on garde cette détection.
        // Pour l'instant, on utilise SEULEMENT le menu contextuel pour démarrer le renommage.
        /*
            let click_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            let mut clicked_on_node = None;
             // Trouver si on a cliqué sur un nœud
            for node in app_state.diagram.nodes.values() {
                if node.rect.contains(click_pos) {
                    clicked_on_node = Some(node.id);
                    break;
                }
            }
             if let Some(node_id) = clicked_on_node {
                app_state.ui_state.renaming_node = Some(node_id);
                app_state.ui_state.selected_node = Some(node_id); // Sélectionner aussi
                // Pré-remplir texte
                 if let Some(node) = app_state.diagram.get_node(node_id) {
                    app_state.ui_state.renaming_label_text = node.name.clone();
                 } else {
                    app_state.ui_state.renaming_label_text = String::new();
                 }
             }

        */
    }
} // Fin handle_rename

fn handle_deletion(ctx: &Context, app_state: &mut AppState) {
    if ctx.input(|i| i.key_pressed(Key::Delete)) || ctx.input(|i| i.key_pressed(Key::Backspace)) {
        if let Some(node_id) = app_state.ui_state.selected_node.take() {
             log::info!("Suppression noeud: {}", node_id);
            app_state.diagram.remove_node(node_id);
            app_state.ui_state.renaming_node = None; // Assurer qu'on arrête de renommer si supprimé
            app_state.ui_state.renaming_arrow = None;
        } else if let Some(arrow_id) = app_state.ui_state.selected_arrow.take() {
             log::info!("Suppression flèche: {}", arrow_id);
             app_state.diagram.remove_arrow(arrow_id);
             app_state.ui_state.renaming_arrow = None; // Assurer qu'on arrête de renommer si supprimé
             app_state.ui_state.renaming_node = None;
        }
    }
}