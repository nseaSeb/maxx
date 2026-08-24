# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

## Composants

- **Liste déroulante.** `Select` réclame un délégué et une entité d'état par
  nœud, comme `Input` : il ne rentre pas dans une entrée de catalogue statique.
  À faire avec la machinerie d'insertion de champs déjà écrite pour le champ
  texte (`view::ensure_input_field`).

## Menus

- **Sous-menus et raccourcis.** L'éditeur gère les menus, les entrées et les
  séparateurs ; un `MenuItem::submenu(..)` est conservé mais pas modifiable, et
  le raccourci d'une entrée se déclare encore à la main dans `key_bindings`.
- **Ni réordonnancement ni glisser-déposer** dans l'éditeur de menus : une
  entrée s'ajoute après la sélection et se supprime, c'est tout.

## Confort

- **Panneaux redimensionnables** via `gpui_component::dock`. Les onglets et le
  défilement sont faits ; la largeur des colonnes est encore figée.
- **`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0** du
  fichier : un `use` placé après l'`impl` attirerait les imports insérés vers le
  bas du fichier. Cas tordu, mais réel.
