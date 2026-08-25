# Architecture

Ce document dit comment maxx est fait et pourquoi. Le `README.md` dit ce qu'il
fait ; le `BACKLOG.md` dit ce qui manque. Les trois se lisent dans cet ordre.

La documentation fine — le contrat d'une fonction, le piège d'un appel — vit
dans les en-têtes de module et les commentaires du code, où elle vieillit avec
lui. `cargo doc --open` la rend lisible.

## La règle qui commande tout le reste

**Le fichier `.rs` est la vérité.** maxx n'a pas de format d'écran, pas de base
de données de projet, aucune représentation qui survivrait à un `git clone`
autrement que sous forme de Rust. Tout ce qui suit découle de là.

Conséquence directe : maxx doit savoir *lire* ce qu'il n'a pas écrit, et
*réécrire* sans abîmer ce qu'il ne comprend pas. C'est ce que le modèle et le
parseur servent à garantir.

## Le tour des modules

| Module | Rôle |
|---|---|
| `model.rs` | l'arbre : une base, une liste ordonnée d'appels, des arguments, des nœuds opaques |
| `parser.rs` | texte Rust vers modèle ; repérage des marqueurs, découpe textuelle |
| `codegen.rs` | modèle vers texte Rust |
| `registry.rs` | le catalogue de composants — le seul endroit à étendre pour en ajouter un |
| `view.rs` | une vue ouverte : chargement, enregistrement, insertion de champs d'état |
| `menu_model.rs` | l'équivalent du modèle, pour une barre de menus |
| `menufile.rs` | l'équivalent de `view.rs`, pour `src/menus.rs` |
| `scaffold.rs` | les gabarits : projet, vue, barre de menus, et leur câblage |
| `project.rs` | l'arborescence de fichiers montrée dans l'explorateur |
| `workspace.rs` | la fenêtre : état, commandes, rendu de la coque |
| `designer.rs` | le canvas, la structure, l'inspecteur, la palette |
| `preferences.rs` | l'écran de réglages |
| `about.rs` | la fenêtre À propos |
| `settings.rs` | ce que maxx retient d'un lancement à l'autre |
| `actions.rs` | les actions, leurs gestionnaires, le clavier |
| `menus.rs` | la barre de menus de maxx |
| `run.rs` | tout ce qui suppose un système : `cargo`, terminal, éditeur, corbeille |
| `theme.rs` | la palette, en `const` |

## Le cycle d'une vue

```
src/ui/accueil.rs
   │  view::View::load
   ▼
repérage des marqueurs // maxx:begin / // maxx:end      (parser, balayage textuel)
   │
   ▼
syn parse la seule expression comprise entre eux
   │
   ▼
model::Node  ── base + appels ordonnés + enfants
   │                        ▲
   │  édition dans le designer
   ▼                        │
codegen rend l'expression   │
   │                        │
   ▼                        │
parser::splice réécrit uniquement la plage d'octets entre les marqueurs
   │
   ▼
src/ui/accueil.rs
```

Trois choses à retenir de ce cycle.

**`syn` ne voit jamais le fichier entier**, parce qu'il perd les commentaires.
La zone gérée est trouvée par balayage textuel, et l'enregistrement ne touche
que cette plage d'octets. Imports, `impl`, méthodes, mise en forme, commentaires
hors zone : intacts par construction, pas par précaution.

**Ce que maxx ne comprend pas est porté, pas perdu.** Une méthode inconnue
devient une donnée réécrite telle quelle ; une expression qui n'est pas une
chaîne de builders — un `if`, un `match`, un composant maison — devient un nœud
opaque, affiché mais jamais réécrit.

**Le texte opaque est stocké désindenté.** `parser::splice` réindente chaque
ligne du bloc qu'il écrit : stocker une tranche avec son indentation de fichier
lui ferait gagner un niveau à chaque enregistrement, sans borne.

La barre de menus suit exactement le même cycle, avec `menu_model` et
`menufile` à la place de `model` et `view`. Elle a son propre modèle parce que
`vec![Menu { name, items }]` est un littéral de structure : le parseur de nœuds
le dégraderait en un unique blob opaque.

## L'état d'une fenêtre

`Workspace` est la vue racine logique de chaque fenêtre. Il tient le projet,
les vues ouvertes, la sélection de l'explorateur, et le *mode* du panneau
central — designer, éditeur de menus, ou préférences.

Deux contraintes de gpui qui expliquent des choses qui auraient l'air tordues
sans elles.

**La racine réelle de la fenêtre est `gpui_component::Root`**, pas `Workspace` :
plusieurs composants remontent jusqu'à elle et interrompent le processus si
elle manque. Le workspace n'est donc pas atteignable en dégradant la poignée de
fenêtre ; il est inscrit dans une table globale `WindowId -> WeakEntity<Workspace>`.

**Un gestionnaire d'action tourne à l'intérieur de la mise à jour d'une
fenêtre.** Ouvrir, activer ou mettre à jour une fenêtre depuis là échoue en
silence — sans erreur, sans panique. Tout ce qui touche une fenêtre depuis
`cx.on_action` passe par `cx.defer` : c'est pourquoi `about::open` est scindé en
deux fonctions, et pourquoi les actions du workspace passent par
`workspace::defer_active`.

Une troisième, propre à l'inspecteur : **taper du texte n'incrémente pas
`revision` et ne pose pas de point d'annulation**, parce que `revision` est ce
qui déclenche la reconstruction des `InputState` — la bumper à chaque frappe
recréerait le champ sous le curseur. Le prix assumé est que les éditions de
texte échappent à `⌘Z`.

## Les réglages

Deux fichiers, comme Zed les sépare, parce que ce ne sont pas deux fois la même
chose.

`settings.json` est à l'utilisateur. Il s'édite à la main autant que par maxx,
donc **maxx n'y réécrit que la clé qu'il change** : `walk` parcourt les membres
de l'objet et `splice_key` remplace la seule tranche d'octets de la valeur,
exactement comme `parser::splice` le fait dans un `.rs`. Commentaires et mise en
forme survivent.

Ce parcours doit connaître les commentaires, pas seulement les chaînes et
l'imbrication, et ce n'est pas un raffinement : une recherche textuelle de la
clé la trouve dans un commentaire qui la cite, et un guillemet impair dans un
commentaire laisse un balayage naïf « dans une chaîne » jusqu'à la fin du
fichier — accolade fermante comprise. Une clé absente est ajoutée juste après
l'accolade ouvrante et non avant la fermante : la dernière chose d'un objet est
souvent un commentaire, et une virgule ajoutée là se retrouve commentée. Un fichier
absent est écrit avec tous ses défauts et une ligne d'explication par clé —
c'est cette partie-là des réglages de Zed qui vaut d'être copiée, avant toute
question de format.

`state.json` est à la machine : projets récents, géométrie de la fenêtre.
Personne ne l'édite, il est réécrit en entier.

Le format est du JSON à commentaires, lu par `serde_json_lenient` — le crate
avec lequel Zed lit les siens, déjà dans l'arbre via gpui. Le JSON strict ne
sait pas porter un commentaire, et un fichier de réglages qu'on ne peut pas
annoter est un fichier dont il faut tenir la documentation ailleurs. Un schéma
JSON est écrit à côté, dérivé de la structure par `schemars`, pour que l'éditeur
complète et signale les fautes de frappe.

Les réglages sont chargés une fois au démarrage dans un `Global`, et c'est la
**seule** source : le workspace ne garde pas de copie de l'état des panneaux, il
lit au rendu. C'est ce qui empêche l'écran de préférences, la barre de menus et
la fenêtre de diverger — et c'est nécessaire, le `SettingField` de
gpui-component lisant et écrivant l'application sans passer par la vue.

Trois voies d'écriture, volontairement distinctes :

- `update_prefs` change une préférence et rustine le fichier de l'utilisateur.
- `update_state` change l'état machine et le réécrit.
- `stage_state` change en mémoire seulement, `flush` écrit à l'extinction. Pour
  ce qui bouge en continu : la géométrie de la fenêtre, où un fichier par image
  serait absurde. Corollaire assumé : un `kill -9` perd la géométrie.

Le principe de lecture : un fichier absent, partiel ou abîmé n'est jamais pire
que pas de fichier. `serde(default)` fait retomber une clé manquante sur son
défaut plutôt que d'échouer la lecture entière, et un fichier illisible est
signalé puis laissé intact — l'écraser perdrait ce que l'utilisateur était en
train d'y écrire.

Le `settings.toml` de la version précédente est repris une fois au démarrage,
scindé en deux, puis renommé `settings.toml.repris` — pas supprimé : une
migration qui mange des données est une migration que personne ne croit.

## Ouvrir un projet : deux chemins, pas un

`workspace::open_folder` ne rejoint `set_project` que lorsqu'il réutilise une
fenêtre déjà ouverte et vide. Sinon — et c'est aussi le cas de `maxx <chemin>`
en ligne de commande — il passe par `open_workspace_window`, qui construit
`Workspace::new` sans jamais toucher `set_project`.

Tout effet de bord attaché à « ouvrir un projet » doit donc être câblé aux deux
endroits. C'est exactement ce qui a fait que les projets récents ne
s'enregistraient pas au premier essai.

## Le système, et rien d'autre

Tout ce qui suppose une plateforme est dans `run.rs` : `cargo`, le terminal,
l'éditeur, la corbeille, le cache partagé, la façon de tuer un groupe de
processus. `settings.rs` connaît en plus les trois conventions de répertoire de
configuration.

C'est délibéré, et c'est ce qui rend le portage abordable : gpui livre déjà les
trois dorsales. Le détail est dans `BACKLOG.md`, section Portabilité.

## Où brancher quoi

- **Un composant de plus** : `registry.rs`, une entrée. Rien d'autre à toucher.
- **Un réglage de plus** : un champ dans `settings::Preferences` avec son
  défaut, une ligne dans `documented_defaults`, puis un `SettingItem` dans
  `preferences.rs`. Le champ lit et écrit les réglages, il ne copie rien.
- **Une entrée de menu de plus** : une action dans `actions.rs`, son gestionnaire,
  et la ligne dans `menus.rs`. Une action qui porte une donnée ne peut pas venir
  de la macro `actions!` — voir `OpenRecent`.
- **Un appel système de plus** : `run.rs`, et nulle part ailleurs.
