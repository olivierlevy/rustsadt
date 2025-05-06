use crate::error::Result; // Utilise l'alias Result<T> = std::result::Result<T, RustSadtError>
use crate::sadt_model::{ProcessNode, SadtDiagram};
use crate::sadt_elements::ArrowType;
use serde::Serialize;
use tera::{Context, Tera};


// Structure pour passer les données au template Tera (Fonction)
#[derive(Serialize)]
struct FunctionContext<'a> {
    name: &'a str,
    inputs: Vec<(&'a str, &'a str)>,
    outputs: Vec<(&'a str, &'a str)>,
    controls: Vec<(&'a str, &'a str)>,
    mechanisms: Vec<(&'a str, &'a str)>,
}

// Structure pour passer les données au template Tera (Module)
#[derive(Serialize)]
struct ModuleContext<'a> {
    module_name: &'a str,
    functions: Vec<FunctionContext<'a>>,
}

// Structure spécifique pour le contexte Markdown
#[derive(Serialize)]
struct MarkdownNodeContext {
    id: String,
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

// Structure spécifique pour le contexte Markdown (Document)
#[derive(Serialize)]
struct MarkdownDocContext { // Retrait du lifetime 'a car on possède les données
     nodes: Vec<MarkdownNodeContext>,
}


pub struct CodeGenerator {
    tera: Tera,
}

impl CodeGenerator {
    pub fn new() -> Result<Self> { // Retourne Result<CodeGenerator, RustSadtError>
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                 log::error!("Erreur lors du chargement des templates Tera depuis 'templates/': {}", e);
                 // Essayer de donner plus de contexte
                 if let Ok(cwd) = std::env::current_dir() {
                    log::error!("Répertoire courant: {}", cwd.display());
                 } else {
                     log::error!("Impossible de déterminer le répertoire courant.");
                 }
                 log::error!("Assurez-vous que le dossier 'templates' existe à la racine du projet et contient les fichiers .tera, et que vous lancez l'exécutable depuis la racine.");
                return Err(e.into()); // Convertit tera::Error en RustSadtError::Tera
            }
        };
        tera.autoescape_on(vec![]);
        Ok(CodeGenerator { tera })
    }

    pub fn generate_rust_module(&self, diagram: &SadtDiagram, module_name: &str) -> Result<String> {
        let mut functions_context = Vec::new();

        for node in diagram.nodes.values() {
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();
            let mut controls = Vec::new();
            let mut mechanisms = Vec::new();

            for arrow in diagram.arrows.values() {
                let label = arrow.label.as_deref().unwrap_or("data");
                let type_placeholder = match arrow.arrow_type {
                    ArrowType::Input => "InputData",
                    ArrowType::Output => "OutputData",
                    ArrowType::Control => "ControlParam",
                    ArrowType::Mechanism => "MechanismResource",
                };

                if arrow.target.node_id == node.id {
                    match arrow.arrow_type {
                        ArrowType::Input => inputs.push((label, type_placeholder)),
                        ArrowType::Control => controls.push((label, type_placeholder)),
                        ArrowType::Mechanism => mechanisms.push((label, type_placeholder)),
                        ArrowType::Output => {}
                    }
                } else if arrow.source.node_id == node.id {
                     if arrow.arrow_type == ArrowType::Output {
                        outputs.push((label, type_placeholder));
                     }
                }
            }

            functions_context.push(FunctionContext {
                name: &node.name, // Référence ok ici car ModuleContext a un lifetime
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

        let tera_context = Context::from_serialize(context).map_err(|e| tera::Error::from(e))?;
        let rendered = self.tera.render("rust_module.tera", &tera_context)?; // Utilise le bon template
        Ok(rendered)
    }

     pub fn generate_markdown_doc(&self, diagram: &SadtDiagram) -> Result<String> {
        let md_nodes: Vec<MarkdownNodeContext> = diagram.nodes.values().map(|node| {
            MarkdownNodeContext {
                id: node.id.to_string(),
                name: node.name.clone(),
                x: node.rect.min.x,
                y: node.rect.min.y,
                width: node.rect.width(),
                height: node.rect.height(),
            }
        }).collect();

        let context = MarkdownDocContext { nodes: md_nodes };
        let tera_context = Context::from_serialize(context).map_err(|e| tera::Error::from(e))?;
        let rendered = self.tera.render("markdown_doc.tera", &tera_context)?;
        Ok(rendered)
     }
}