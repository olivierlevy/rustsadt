use crate::error::Result;
use crate::error::RustSadtError;
use crate::sadt_model::SadtDiagram;
use crate::sadt_elements::ArrowType;
use rfd::FileDialog;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
// Ajouts pour SVG
use svg::node::element::{Line, Polygon, Rectangle, Text as SvgText}; // Renommer Text pour éviter conflit
use svg::Document;

const FILE_EXTENSION: &str = "ron";
const SVG_FILE_EXTENSION: &str = "svg";

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

// Exporte le diagramme en SVG
pub fn export_svg(diagram: &SadtDiagram, path: &PathBuf) -> Result<()> {
    let mut document = Document::new().set("viewBox", (0, 0, 1024, 768)); // Vue initiale (peut être ajustée)

    // Dessiner les flèches d'abord (pour qu'elles soient en dessous)
    for arrow in diagram.arrows.values() {
        if let (Some(src_node), Some(tgt_node)) = (
            diagram.nodes.get(&arrow.source.node_id),
            diagram.nodes.get(&arrow.target.node_id),
        ) {
            let start_pos = crate::ui::drawing::get_connection_pos(src_node, arrow.source.side);
            let end_pos = crate::ui::drawing::get_connection_pos(tgt_node, arrow.target.side);

            let color_str = match arrow.arrow_type {
                ArrowType::Input => "lightgreen",
                ArrowType::Output => "lightblue",
                ArrowType::Control => "lightcoral", // lightred n'est pas standard SVG
                ArrowType::Mechanism => "yellow",   // lightyellow n'est pas standard
            };

            let line = Line::new()
                .set("x1", start_pos.x)
                .set("y1", start_pos.y)
                .set("x2", end_pos.x)
                .set("y2", end_pos.y)
                .set("stroke", color_str)
                .set("stroke-width", 1.5)
                .set("marker-end", "url(#arrowhead)"); // Référence à la définition de la pointe
            document = document.add(line);

            // Ajouter le label de la flèche
            if let Some(label) = &arrow.label {
                let mid_x = (start_pos.x + end_pos.x) / 2.0;
                let mid_y = (start_pos.y + end_pos.y) / 2.0;
                let text = SvgText::new(label)
                    .set("x", mid_x)
                    .set("y", mid_y - 5.0) // Un peu au-dessus de la ligne
                    .set("fill", color_str)
                    .set("font-size", "10px")
                    .set("text-anchor", "middle"); // Centrer le texte
                 document = document.add(text);
            }
        }
    }

    // Dessiner les nœuds
    for node in diagram.nodes.values() {
        let rect = Rectangle::new()
            .set("x", node.rect.min.x)
            .set("y", node.rect.min.y)
            .set("width", node.rect.width())
            .set("height", node.rect.height())
            .set("rx", 5) // coins arrondis
            .set("ry", 5)
            .set("fill", "rgb(50, 50, 50)") // Gris foncé
            .set("stroke", "gray")
            .set("stroke-width", 1);
        document = document.add(rect);

        // Ajouter le nom du nœud
        let text = SvgText::new(&node.name)
            .set("x", node.rect.center().x)
            .set("y", node.rect.center().y)
            .set("fill", "white")
            .set("font-size", "14px")
            .set("dy", ".3em") // Ajustement vertical pour centrer
            .set("text-anchor", "middle"); // Centrer horizontalement
        document = document.add(text);
    }

    // Définir une pointe de flèche réutilisable (marker)
    // Note: Les couleurs des pointes ne peuvent pas être facilement héritées ici
    // Pour simplifier, on fait une pointe grise.
    let arrowhead_marker = svg::node::element::Marker::new()
        .set("id", "arrowhead")
        .set("viewBox", "0 0 10 10")
        .set("refX", 9) // Position de la pointe sur le chemin
        .set("refY", 5)
        .set("markerWidth", 6)
        .set("markerHeight", 6)
        .set("orient", "auto-start-reverse")
        .add(
            Polygon::new()
                .set("points", "0,0 10,5 0,10")
                .set("fill", "gray"), // Couleur fixe pour la pointe
        );

    // Ajouter la définition du marker dans un bloc <defs>
    let defs = svg::node::element::Definitions::new().add(arrowhead_marker);
    document = document.add(defs);

    // Sauvegarder le document SVG
    svg::save(path, &document).map_err(|e| RustSadtError::Io(e))?;
    log::info!("Diagramme exporté en SVG dans: {}", path.display());
    Ok(())
}

// Ouvre une boîte de dialogue pour choisir où exporter en SVG
pub fn export_svg_dialog(diagram: &SadtDiagram) -> Result<Option<PathBuf>> {
    let path = FileDialog::new()
        .add_filter("Scalable Vector Graphics", &[SVG_FILE_EXTENSION])
        .set_file_name("diagram.svg")
        .save_file();

    match path {
        Some(p) => {
            export_svg(diagram, &p)?;
            Ok(Some(p))
        }
        None => Ok(None), // L'utilisateur a annulé
    }
}
