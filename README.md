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

Ce qui est connu et reporté est dans [`BACKLOG.md`](BACKLOG.md).
