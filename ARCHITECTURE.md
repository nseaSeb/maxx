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
| `tools.rs` | le catalogue des éditeurs et des terminaux, leur détection |
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

## Éditeur et terminal

`tools.rs` tient une table, pas une heuristique, et c'est le point : ouvrir un
fichier *à une ligne* s'écrit différemment chez chacun — `zed fichier:12`,
`code -g fichier:12`, `nvim +12 fichier`, `idea --line 12 fichier` — et il n'y a
pas de majorité à suivre. Confondre deux de ces formes n'échoue pas franchement :
`code fichier:12` ouvre un fichier nommé « fichier:12 ».

Le piège qui ne se voit pas au moment de choisir : `hx`, `nvim` et `vim` ne sont
pas des applications mais des programmes qui ont besoin d'un terminal autour
d'eux. Les deux réglages ne sont donc pas indépendants, et tous les terminaux ne
savent pas recevoir une commande — celui de macOS n'y donne accès que par
AppleScript, qui réclame une permission d'automatisation au milieu d'un clic.

`auto`, le défaut, prend le premier installé du catalogue ; pour l'éditeur,
`$VISUAL` et `$EDITOR` passent avant, parce que qui les a réglés a déjà dit ce
qu'il voulait. La détection cherche la commande sur le `PATH` et, sur macOS, le
paquet dans `/Applications`.

## Le système, et rien d'autre

Tout ce qui suppose une plateforme est dans `run.rs`, et nulle part ailleurs :
`cargo`, le terminal, l'éditeur, la corbeille, le cache, la façon de tuer
l'arbre de processus lancé. `settings.rs` connaît en plus les conventions de
répertoire de configuration, et `tools.rs` celles de détection.

Ce qui diffère vraiment, système par système :

- **Le cache et la configuration.** `XDG_*` quand l'utilisateur les a réglés,
  `LOCALAPPDATA` et `APPDATA` sur Windows, `Library/Caches` et
  `Library/Application Support` sur macOS, `.cache` et `.config` ailleurs.
- **La corbeille.** `~/.Trash` sur macOS ; sur Linux la spécification
  freedesktop, `$XDG_DATA_HOME/Trash/files` plus un `.trashinfo` sans lequel le
  bureau ne sait pas d'où le fichier venait et ne peut pas le restaurer ; sur
  Windows une corbeille propre à maxx, parce que la vraie ne s'atteint que par
  l'API du shell — ce qui coûterait une dépendance et un bloc `unsafe` pour un
  geste qui doit rester simple. maxx le dit plutôt que de faire semblant.
- **Tuer ce qui a été lancé.** `cargo` lance lui-même l'application : signaler
  `cargo` seul laisserait la fenêtre ouverte. Sur unix l'enfant reçoit son
  propre groupe de processus, que `kill -TERM -pid` atteint en entier ; Windows
  n'a pas cette notion et `taskkill /T` fait le geste équivalent.
- **Le repli par paquet d'application.** `open -a` est un outil macOS et un
  `.app` une notion macOS : ailleurs, la commande sur le `PATH` est la seule
  voie, et son absence est la raison pour laquelle rien ne s'est produit.

Ce que la CI en matrice prouve : que ça compile et que la suite passe sur les
trois. C'est ce qui empêche une ligne comme `std::os::unix::process::CommandExt`
de se réintroduire sans qu'on le voie. Ce qu'elle ne prouve pas : que
l'interface est utilisable, aucun test n'ouvrant de fenêtre. maxx n'a été
essayé à la main que sur macOS.

## Ce que maxx ajoute à un projet

Quatre choses s'ajoutent à un projet existant, par insertion textuelle et
jamais par réécriture depuis le gabarit : une vue, la barre de menus, le module
système, les réglages. Ce dernier tire le module système avec lui et déclare
deux crates dans le `Cargo.toml` du projet — insérées dans la section des
dépendances, pas à la fin du fichier, pour qu'un bloc `[profile]` reste après
elles. Le projet peut être antérieur à maxx et faire autre chose au
démarrage — il doit le garder.

Le module système mérite sa règle : il ne contient que ce qui diffère d'un
système à l'autre **et** que gpui ne fournit pas déjà. Le presse-papier,
`open_url`, `reveal_path`, `open_with_system`, les sélecteurs de fichiers sont
dans gpui ; les enrober ajouterait une couche à maintenir pour rien. Reste où
vont les fichiers d'une application et ce que « supprimer » veut dire — ce que
toute application de bureau finit par écrire, et que personne ne veut écrire
une troisième fois.

Symétrie nécessaire : supprimer `src/<module>.rs` depuis l'explorateur retire
sa ligne `mod` de `main.rs`. Sans quoi supprimer un fichier casse la
compilation, ce qui est l'inverse du but.

**Une copie est une dette**, et il faut la nommer : le module système et les
réglages reprennent du code que maxx a écrit pour lui-même. Un défaut trouvé
d'un côté doit être porté de l'autre. C'est déjà arrivé — la fiche
`.trashinfo`, non conforme dans les deux à la fois. Le prix est assumé, un
projet généré ne devant rien à maxx, mais il se paie à chaque correction.

## La démo comme référence

`demo/` est un projet complet, versionné, avec sa propre racine d'espace de
travail — `cargo check` à la racine du dépôt ne le compile donc pas. Il est
écrit dans la forme exacte que `codegen` produit, ce qui rend une propriété
vérifiable : `tests/demo.rs` relit chaque vue, la réécrit, et exige le fichier
identique à l'octet près. Tout écart est une perte.

C'est aussi la seule référence de ce que maxx doit comprendre. Elle a remplacé
un chemin absolu vers un dossier personnel, dans un test qui s'arrêtait sans
échouer quand il manquait : chez quelqu'un d'autre, la couverture était nulle et
silencieuse.

## Où brancher quoi

- **Un composant de plus** : `registry.rs`, une entrée. Rien d'autre à toucher.
- **Un réglage de plus** : un champ dans `settings::Preferences` avec son
  défaut, une ligne dans `documented_defaults`, puis un `SettingItem` dans
  `preferences.rs`. Le champ lit et écrit les réglages, il ne copie rien.
- **Une entrée de menu de plus** : une action dans `actions.rs`, son gestionnaire,
  et la ligne dans `menus.rs`. Une action qui porte une donnée ne peut pas venir
  de la macro `actions!` — voir `OpenRecent`.
- **Un appel système de plus** : `run.rs`, et nulle part ailleurs.
