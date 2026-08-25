# Démonstration de maxx

Un projet `gpui` + `gpui-component` ordinaire, écrit dans la forme que maxx
produit et relit. Il ne dépend pas de maxx : `cargo run` suffit.

```sh
cd demo
cargo run
```

## Ce qu'il montre

**Les composants du catalogue**, tous dans `src/ui/accueil.rs` : cadre, libellé,
champ texte lié à un champ de la vue, case à cocher, interrupteur, séparateur,
bouton avec infobulle et gestionnaire.

**Une fenêtre ouverte depuis la barre de menus** — `Fenêtre > Ouvrir
l'inspecteur`, ou `⌘I`, ou le bouton de l'accueil. C'est le geste qui réunit
deux pièges que rien ne signale au moment où on les commet :

- un gestionnaire d'action tourne à l'intérieur de la mise à jour de la fenêtre
  qui l'a émis, et gpui refuse d'en entrer une seconde. Ouvrir une fenêtre
  directement depuis `cx.on_action` ne fait **rien du tout** — sans erreur, sans
  panique. D'où le `cx.defer` dans `src/menus.rs` ;
- une fenêtre qui dessine le moindre composant de `gpui-component` doit être
  enracinée dans `Root`. Plusieurs composants remontent jusqu'à lui et
  interrompent le processus s'il manque. La fenêtre « À propos », qui n'utilise
  que du gpui nu, est la seule à s'en passer.

**Une barre de menus éditable.** Ouvrez `demo/` dans maxx, cliquez
`src/menus.rs` : l'éditeur de menus s'affiche.

## Pourquoi elle est dans le dépôt

Elle sert de référence aux tests : `tests/demo.rs` vérifie que maxx relit chaque
vue, que la réécriture est neutre à l'octet près, et que la barre de menus se
relit avec son entrée d'ouverture de fenêtre. Une démo qui se casse fait échouer
la suite.
