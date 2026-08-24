# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

## Sécurité des fichiers — reporté (usage personnel pour l'instant)

- **Détection d'une modification extérieure.** `⌘S` réécrit le fichier sans
  regarder s'il a changé sur disque depuis son ouverture. Si la vue est éditée
  dans Zed pendant que maxx la tient ouverte, l'enregistrement écrase ce
  travail. Correction prévue : comparer le contenu du disque avec `View::source`
  avant d'écrire, refuser et proposer « Recharger » (`⌘R`).
- **Adopter une vue existante.** maxx n'ouvre que les vues qu'il a écrites,
  puisqu'il exige les marqueurs `// maxx:begin` / `// maxx:end`. Une action qui
  insère ces marqueurs autour du corps d'un `render` déjà écrit à la main
  permettrait d'ouvrir un projet GPUI que maxx n'a pas généré.

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
