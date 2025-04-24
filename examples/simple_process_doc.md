# Documentation SADT Générée

Ce document décrit le diagramme SADT généré par RustSADT.

## Activités (Nœuds)

### 1. Traiter Données (ID: `a1a1a1a1-b1b1-c1c1-d1d1-e1e1e1e1e1e1`)

- **Description:** (Ajouter une description ici, potentiellement depuis le modèle)
- **Position:** (100, 100)
- **Taille:** 120 x 60

- **Entrées (Inputs):** Données Brutes (depuis Externe/Placeholder)
- **Sorties (Outputs):** Données Traitées (vers Générer Rapport)
- **Contrôles (Controls):** (Vide)
- **Mécanismes (Mechanisms):** (Vide)

---

### 2. Générer Rapport (ID: `a2a2a2a2-b2b2-c2c2-d2d2-e2e2e2e2e2e2`)

- **Description:** (Ajouter une description ici, potentiellement depuis le modèle)
- **Position:** (300, 100)
- **Taille:** 120 x 60

- **Entrées (Inputs):** Données Traitées (depuis Traiter Données)
- **Sorties (Outputs):** (Vide dans l'exemple .ron, pourrait être 'Rapport Final')
- **Contrôles (Controls):** (Vide)
- **Mécanismes (Mechanisms):** (Vide)

---

## Flux (Flèches)

(Section à remplir manuellement ou par le générateur si amélioré)

- `f1f1f1f1...`: "Données Traitées" (Output) de [Traiter Données] vers [Générer Rapport]
- `f2f2f2f2...`: "Données Brutes" (Input) de [Externe] vers [Traiter Données]
