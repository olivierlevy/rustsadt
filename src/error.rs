use thiserror::Error;
use ron::error::SpannedError;

#[derive(Error, Debug)]
pub enum RustSadtError {
    #[error("Erreur d'entrée/sortie: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de sérialisation/désérialisation RON: {0}")]
    Ron(#[from] ron::Error),
    
    #[error("Erreur de désérialisation RON (avec position): {0}")]
    RonSpanned(#[from] SpannedError), // Ajouter celui-ci pour ron::de::Error

    #[error("Erreur de rendu du template Tera: {0}")]
    Tera(#[from] tera::Error),

    #[error("Impossible de trouver le répertoire home")]
    HomeDir,

    #[error("Action annulée par l'utilisateur")]
    UserCancelled,

    #[error("Élément non trouvé avec l'ID: {0}")]
    NotFound(String),

    #[error("Erreur de génération: {0}")]
    Generation(String),

    #[error("Erreur d'interface utilisateur: {0}")]
    Ui(String),
}

pub type Result<T> = std::result::Result<T, RustSadtError>;