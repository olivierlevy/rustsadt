use crate::error::Result;
use crate::sadt_model::SadtDiagram;
use rfd::FileDialog;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

const FILE_EXTENSION: &str = "ron";

// Sauvegarde le diagramme dans un fichier RON
pub fn save_diagram(diagram: &SadtDiagram, path: &PathBuf) -> Result<()> {
    let ron_string = ron::ser::to_string_pretty(diagram, ron::ser::PrettyConfig::default())?;
    let mut file = File::create(path)?;
    file.write_all(ron_string.as_bytes())?;
    log::info!("Diagramme sauvegardé dans: {}", path.display());
    Ok(())
}

// Charge un diagramme depuis un fichier RON
pub fn load_diagram(path: &PathBuf) -> Result<SadtDiagram> {
    let mut file = File::open(path)?;
    let mut ron_string = String::new();
    file.read_to_string(&mut ron_string)?;
    let diagram: SadtDiagram = ron::from_str(&ron_string)?;
     log::info!("Diagramme chargé depuis: {}", path.display());
    Ok(diagram)
}

// Ouvre une boîte de dialogue pour choisir où sauvegarder
pub fn save_diagram_dialog(diagram: &SadtDiagram) -> Result<Option<PathBuf>> {
    let path = FileDialog::new()
        .add_filter("SADT Diagram", &[FILE_EXTENSION])
        .set_file_name("diagram.ron")
        .save_file();

    match path {
        Some(p) => {
            save_diagram(diagram, &p)?;
            Ok(Some(p))
        }
        None => Ok(None), // L'utilisateur a annulé
    }
}

// Ouvre une boîte de dialogue pour choisir quel fichier charger
pub fn load_diagram_dialog() -> Result<Option<(SadtDiagram, PathBuf)>> {
     let path = FileDialog::new()
        .add_filter("SADT Diagram", &[FILE_EXTENSION])
        .pick_file();

     match path {
        Some(p) => {
            let diagram = load_diagram(&p)?;
            Ok(Some((diagram, p)))
        }
        None => Ok(None), // L'utilisateur a annulé
     }
}