use crate::error::RustSadtError;
use crate::sadt_model::{ProcessNode, SadtDiagram};
use crate::sadt_elements::ArrowType;
use serde::Serialize;
use tera::{Context, Tera};

// Structure pour passer les données au template Tera
#[derive(Serialize)]
struct FunctionContext<'a> {
    name: &'a str,
    // Simplifié: on utilise des types génériques ou placeholders
    inputs: Vec<(&'a str, &'a str)>,  // (nom_flèche/placeholder, type_placeholder)
    outputs: Vec<(&'a str, &'a str)>, // (nom_flèche/placeholder, type_placeholder)
    controls: Vec<(&'a str, &'a str)>,
    mechanisms: Vec<(&'a str, &'a str)>,
    // On pourrait ajouter plus d'infos ici (documentation, etc.)
}

#[derive(Serialize)]
struct ModuleContext<'a> {
    module_name: &'a str, // Nom du module (dérivé du diagramme?)
    functions: Vec<FunctionContext<'a>>,
    // Potentiellement: imports nécessaires, structs globales, etc.
}

#[derive(Serialize)]
struct MarkdownContext<'a> {
     nodes: Vec<&'a ProcessNode>,
     // On pourrait ajouter les flèches ici pour une description plus complète
}

pub struct CodeGenerator {
    tera: Tera,
}

impl CodeGenerator {
    pub fn new() -> crate::error::Result<Self> {
        let mut tera = Tera::new("templates/**/*")?; // Charger tous les templates
        tera.autoescape_on(vec![]); // Important pour générer du code brut
        Ok(CodeGenerator { tera })
    }

    pub fn generate_rust_module(&self, diagram: &SadtDiagram, module_name: &str) -> crate::error::Result<String> {
        let mut functions_context = Vec::new();

        for node in diagram.nodes.values() {
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();
            let mut controls = Vec::new();
            let mut mechanisms = Vec::new();

            // Trouver les flèches connectées à ce noeud
            for arrow in diagram.arrows.values() {
                let label = arrow.label.as_deref().unwrap_or("data"); // Nom par défaut
                 // Très simplifié: Type placeholder basé sur le type de flèche
                let type_placeholder = match arrow.arrow_type {
                    ArrowType::Input => "InputData",
                    ArrowType::Output => "OutputData",
                    ArrowType::Control => "ControlParam",
                    ArrowType::Mechanism => "MechanismResource",
                };

                if arrow.target.node_id == node.id { // Flèche entrante
                    match arrow.arrow_type {
                        ArrowType::Input => inputs.push((label, type_placeholder)),
                        ArrowType::Control => controls.push((label, type_placeholder)),
                        ArrowType::Mechanism => mechanisms.push((label, type_placeholder)),
                        ArrowType::Output => {} // Une sortie ne devrait pas *entrer* ici
                    }
                } else if arrow.source.node_id == node.id { // Flèche sortante
                     if arrow.arrow_type == ArrowType::Output {
                        outputs.push((label, type_placeholder));
                     }
                }
            }

            functions_context.push(FunctionContext {
                name: &node.name,
                inputs,
                outputs,
                controls,
                mechanisms,
            });
        }

        let context = ModuleContext {
            module_name,
            functions: functions_context,
        };

        // La sérialisation peut échouer, ajoutons un contexte d'erreur si besoin
        let tera_context = Context::from_serialize(context).map_err(|e| RustSadtError::Tera(e.into()))?; // Assurez-vous que l'erreur Tera est gérée
        let rendered = self.tera.render("rust_module.tera", &tera_context)?;
        Ok(rendered)
    }

    pub fn generate_markdown_doc(&self, diagram: &SadtDiagram) -> crate::error::Result<String> {
        let nodes: Vec<&ProcessNode> = diagram.nodes.values().collect();
        let context = MarkdownContext { nodes };
        let tera_context = Context::from_serialize(context).map_err(|e| RustSadtError::Tera(e.into()))?;
        let rendered = self.tera.render("markdown_doc.tera", &tera_context)?;
        Ok(rendered)
     }
}