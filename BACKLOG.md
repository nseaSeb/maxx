# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

La sécurité des fichiers — détection d'une modification extérieure et adoption
d'une vue existante — est faite ; voir le README.

## Composants

- **Liste déroulante.** `Select` réclame un délégué et une entité d'état par
  nœud, comme `Input` : il ne rentre pas dans une entrée de catalogue statique.
  À faire avec la machinerie d'insertion de champs déjà écrite pour le champ
  texte (`view::ensure_input_field`).

## Confort

- **Lancer le projet depuis maxx** (`⌘R` → `cargo run`), avec la sortie et les
  erreurs de compilation dans un panneau.
- **Panneaux redimensionnables** via `gpui_component::dock`, et des onglets pour
  plusieurs vues ouvertes.
- **`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0** du
  fichier : un `use` placé après l'`impl` attirerait les imports insérés vers le
  bas du fichier. Cas tordu, mais réel.
