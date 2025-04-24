// src/main.rs
use eframe::egui;

// Déclaration des modules pour qu'ils soient accessibles depuis la racine du crate
mod app;
mod error;
mod generator;
mod persistence;
mod sadt_elements;
mod sadt_model;
// Déclaration du module ui et de ses sous-modules
mod ui {
    pub mod drawing;
    pub mod interaction;
    // Le fichier `mod.rs` est souvent implicite, pas besoin de 'mod_impl'
}

fn main() -> Result<(), eframe::Error> {
    // Setup logging
    env_logger::init(); // Ou un autre logger comme tracing

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([600.0, 400.0]),
            // .with_icon(...) // TODO: Ajouter une icône
        ..Default::default()
    };

    log::info!("Lancement de RustSADT...");

    eframe::run_native(
        "RustSADT", // Titre de la fenêtre
        options,
        Box::new(|cc| {
            // Configurer explicitement les polices egui lors de la création
            // Cela garantit qu'elles sont prêtes avant le premier rendu.
            let fonts = egui::FontDefinitions::default(); // Pas besoin de 'mut'
            // Ici, vous pourriez charger/configurer des polices personnalisées si nécessaire
            cc.egui_ctx.set_fonts(fonts);

            // Retourner l'état initial de l'application
            // Utilise le module 'app' déclaré plus haut
            Box::<app::RustSadtApp>::default()
        }),
    )
}