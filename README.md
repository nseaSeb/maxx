# maxx

Un atelier visuel qui construit des vues [GPUI](https://gpui.rs) et les écrit
sous forme de vrai code Rust.

On crée ou on ouvre un projet, on pose des composants, on règle leurs
propriétés, on câble une action — et ce qui sort est un projet
`gpui` + `gpui-component` ordinaire, qui compile et tourne sans maxx, et qui
s'ouvre dans Zed comme n'importe quel projet Rust.

## Le principe

**Le fichier `.rs` est la vérité.** maxx n'a pas de format d'écran ; il écrit
dans `src/ui/<vue>.rs` et le relit avec `syn`.

Une vue est modélisée comme *une expression de base plus une liste ordonnée
d'appels de méthodes* — exactement la forme du code GPUI :

```rust
// maxx:begin
v_flex()
    .gap_2()
    .p_4()
    .child(Label::new("Nom"))
    .child(Input::new(&self.champ))
    .child(Button::new("valider").label("Valider").on_click(cx.listener(Self::on_valider)))
// maxx:end
```

C'est ce qui rend l'aller-retour sûr :

- une méthode que maxx ne connaît pas est portée comme donnée et réécrite
  telle quelle ;
- une expression qui n'est pas une chaîne de builders — un `if`, un `match`,
  un composant maison — devient un nœud opaque, affiché mais jamais réécrit ;
- `syn` ne reçoit jamais le fichier entier, car il perd les commentaires : la
  zone gérée est repérée par balayage textuel entre `// maxx:begin` et
  `// maxx:end`, et l'enregistrement n'y touche que cette plage d'octets. Le
  reste du fichier — imports, `impl`, méthodes, commentaires, mise en forme —
  est intact par construction.

Corollaire : ce que vous écrivez à la main dans Zed est relu par maxx, et ce
que maxx écrit est du Rust que vous auriez pu écrire.

## Utilisation

```sh
cargo run              # écran d'accueil
cargo run -- <chemin>  # ouvre un projet directement
```

`Fichier > Nouveau projet…` crée un projet complet, `Fichier > Nouvelle vue…`
ajoute une vue et l'inscrit dans `src/ui/mod.rs`.

Dans le canvas : clic pour sélectionner, glisser pour déplacer, double-clic sur
un bouton pour lui donner une action. `⌘S` écrit le fichier, `⌘Z` / `⌘⇧Z`
annulent, `⌘⇧⌫` supprime le nœud sélectionné, `⌘B` masque le panneau du projet.

## Prérequis

macOS avec Xcode. La dépendance `gpui` active la feature `runtime_shaders`,
qui compile les shaders Metal au lancement plutôt qu'à la compilation : Xcode 26
livre le toolchain Metal en composant séparé, et sans cette feature le build
échoue sur un outil `metal` introuvable. Les projets générés portent la même
feature, pour la même raison.

## État de la vue

Une propriété texte est un littéral par défaut. Le bouton `abc` de l'inspecteur
la fait lire un champ de la vue à la place :

```rust
Label::new("Titre")                 →   Label::new(self.titre.clone())
```

Les champs se déclarent dans la section « État », qui les insère dans la struct
et dans `new`. Un `usize` ou un `f32` est rendu par `.to_string()`, un
`SharedString` par `.clone()`.

Une propriété liée n'est plus éditable en texte libre : l'écraser par un
littéral changerait silencieusement ce que le code veut dire.

Ce qui reste à écrire à la main, et c'est voulu : le corps des méthodes. Le
bouton `→ Zed` à côté de la propriété Action ouvre l'éditeur sur la ligne de la
méthode. Et il ne faut pas oublier `cx.notify()` — sans lui le champ change et
l'écran ne bouge pas.

## Barre de menus

Une application GPUI n'a aucune barre de menus tant qu'elle n'appelle pas
`set_menus` — pas même un « Quitter ». Le gabarit en pose donc une, dans
`src/menus.rs`, avec les gestes que macOS attend : À propos, Masquer, Quitter,
un menu Édition câblé sur les actions système, et Réduire.

Ce fichier a sa propre zone marquée : ouvrez-le depuis l'explorateur et maxx
affiche un éditeur de menus. Ajouter une entrée avec une action inconnue
déclare cette action dans `actions!` et lui écrit un gestionnaire vide, comme le
double-clic sur un bouton le fait pour une vue. Une entrée que maxx ne
reconnaît pas — un sous-menu, un appel maison — est conservée telle quelle.

## Fichiers modifiés en dehors de maxx

maxx tient une copie du fichier en mémoire, donc écrire sans regarder le disque
écraserait ce qui a été tapé dans Zed entre-temps. Au retour du focus, et de
nouveau avant chaque enregistrement, maxx compare :

* le disque a changé et l'arbre n'a pas été touché ici — rechargement
  automatique, comme un éditeur le fait pour un tampon non modifié ;
* les deux côtés ont changé — refus d'écrire, la barre d'état le signale, et
  `Fichier > Recharger la vue` (⌘⇧R) ou `Fichier > Écraser le fichier`
  tranchent.

## Ouvrir une vue que maxx n'a pas écrite

`Fichier > Adopter cette vue` pose les marqueurs autour de l'expression que
retourne un `fn render` écrit à la main. Rien d'autre n'est touché, et les
instructions qui précèdent l'expression finale sont laissées telles quelles.
Si le corps ne se termine pas par une expression, l'adoption échoue et le dit :
maxx ne saurait pas où couper.

## Compilation des projets générés

`gpui` et `gpui-component` représentent environ 750 crates. Un projet qui a son
propre `target/` les recompile intégralement, ce qui coûte plusieurs minutes à
chaque nouveau projet.

Chaque projet généré reçoit donc un `.cargo/config.toml` qui pointe vers un
cache commun, `~/Library/Caches/maxx/target` : le premier projet paie le prix,
les suivants sont quasi instantanés. Comme le fichier porte un chemin absolu, il
est propre à la machine et se trouve dans le `.gitignore` du projet — le perdre
ne coûte qu'une recompilation. Un `cargo run` tapé dans un terminal lit le même
fichier, donc terminal et maxx partagent le cache.

À la création d'un projet, `cargo build` démarre en arrière-plan pour payer ce
prix pendant que vous dessinez. `Exécution > Préparer les dépendances` le
relance à la demande.

## Organisation

| Fichier | Rôle |
|---|---|
| `src/model.rs` | l'arbre : base, appels, arguments, nœuds opaques |
| `src/codegen.rs` | modèle → texte Rust |
| `src/parser.rs` | texte Rust → modèle, marqueurs et découpe textuelle |
| `src/registry.rs` | le catalogue de composants — le seul endroit à étendre |
| `src/view.rs` | une vue ouverte : chargement, enregistrement, insertions |
| `src/scaffold.rs` | gabarits de projet et de vue |
| `src/designer.rs` | canvas, structure, inspecteur, palette |
| `src/workspace.rs` | la fenêtre, l'état, les commandes |
| `src/about.rs` | la fenêtre « À propos » |

Ce qui est connu et reporté est dans [`BACKLOG.md`](BACKLOG.md).

## Réglages

maxx écrit ce qu'il retient dans un TOML — `~/Library/Application Support/maxx/`
sur macOS, `$XDG_CONFIG_HOME` ou `~/.config` ailleurs. Il s'édite à la main :
projets récents, panneaux affichés, position de la fenêtre. Un fichier absent,
partiel ou abîmé n'empêche jamais maxx de démarrer — chaque valeur a un défaut,
et un fichier illisible est signalé puis laissé tel quel, pour ne pas écraser ce
que vous étiez en train d'y écrire.

## Licence

maxx est sous licence MIT — voir [`LICENSE`](LICENSE).

GPUI et gpui-component sont sous Apache-2.0, ce qui n'impose rien de plus que
de joindre leur licence et de conserver leurs mentions de copyright. Un crate
transitif, `option-ext`, est sous MPL-2.0. Le détail et ce que la distribution
d'un binaire demande sont dans [`THIRD-PARTY.md`](THIRD-PARTY.md).

Les projets générés n'héritent d'aucune licence : maxx écrit du Rust ordinaire,
qui vous appartient.
