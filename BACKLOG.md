# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

## À vérifier

- **`⌘⌥T` ouvre-t-il vraiment le terminal dans le dossier du projet ?**
  L'action lance bien Ghostty, c'est mesuré, mais rien ne confirme que le shell
  démarre dans le bon répertoire — un processus dont on n'est pas propriétaire
  ne laisse pas lire son répertoire courant. Si Ghostty atterrit dans `$HOME`,
  changer l'invocation dans `run::open_terminal`.

## Éditeur

- **Le champ lié d'un champ texte se tape à la main.** La section « État »
  connaît maintenant les champs déclarés ; la propriété « Champ lié » devrait
  proposer cette liste au lieu d'une saisie libre, comme le fait déjà une
  propriété texte liée.
- **Une longueur ou une couleur invalide est ignorée en silence.** `registry`
  refuse d'écrire ce qui n'est pas un nombre ou six chiffres hexadécimaux, mais
  rien ne le dit à l'écran.
- **La frappe dans l'inspecteur n'est pas annulable.** Un point d'annulation par
  lettre serait pire, mais le résultat est qu'une saisie de texte échappe
  entièrement à `⌘Z`. Il faudrait un point de reprise à la perte du focus.
- **Après une annulation, la sélection retombe sur la racine** au lieu de rester
  sur le nœud concerné.
- **Ni renommage ni suppression d'une vue depuis maxx** — il faut passer par le
  Finder ou le terminal, et corriger `src/ui/mod.rs` à la main.

## Composants

- **Liste déroulante.** `Select` réclame un délégué et une entité d'état par
  nœud, comme `Input` : il ne rentre pas dans une entrée de catalogue statique.
  À faire avec la machinerie d'insertion de champs déjà écrite pour le champ
  texte (`view::ensure_input_field`).

## Confort

- **Panneaux redimensionnables** via `gpui_component::dock`. Les onglets et le
  défilement sont faits ; la largeur des colonnes est encore figée.
- **`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0** du
  fichier : un `use` placé après l'`impl` attirerait les imports insérés vers le
  bas du fichier. Cas tordu, mais réel.
