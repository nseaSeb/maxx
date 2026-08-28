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
| `workspace.rs` | la fenêtre : l'état, l'ouverture et la fermeture d'un projet |
| `workspace/views.rs` | les onglets, la lecture et l'écriture d'une vue |
| `workspace/inspector.rs` | la sélection, les propriétés, l'état, l'insertion, l'annulation |
| `workspace/explorer.rs` | l'arbre de fichiers, sa sélection, ses suppressions |
| `workspace/code.rs` | le lecteur de code : n'importe quel fichier texte, en lecture seule |
| `workspace/menus.rs` | l'éditeur de barre de menus |
| `workspace/chrome.rs` | la coque : titre, écran d'accueil, barre d'état, `Render` |
| `workspace/process.rs` | `cargo run` et le panneau de sortie |
| `workspace/modules.rs` | les modules copiés dans le projet |
| `designer.rs` | le canvas, la structure, l'inspecteur, la palette |
| `preferences.rs` | l'écran de réglages |
| `about.rs` | la fenêtre À propos |
| `settings.rs` | ce que maxx retient d'un lancement à l'autre |
| `actions.rs` | les actions, leurs gestionnaires, le clavier |
| `menus.rs` | la barre de menus de maxx |
| `tools.rs` | le catalogue des éditeurs et des terminaux, leur détection |
| `run.rs` | tout ce qui suppose un système : `cargo`, terminal, éditeur, corbeille |
| `watch.rs` | la veille du projet sur le disque : ce qui réveille la fenêtre |
| `theme.rs` | la palette, en deux modes |
| `palette.rs` | la palette ⌘K : la barre de menus, aplatie |
| `locales/app.yml` | les traductions, une entrée par clé |

## Le cycle d'une vue

```
src/ui/home.rs
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
src/ui/home.rs
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

Deux workflows, deux rôles. `ci.yml` donne un signal rapide et fréquent ;
`release.yml` est le portail de publication : il part sur un tag `v*`, passe la
matrice entière, construit en release — ce que la CI ordinaire ne fait jamais,
et une optimisation révèle ce qu'un build de debug tolère —, vérifie ce qui
partirait dans un paquet crates.io, et attache les binaires des trois systèmes
à la version. Un tag reste le pire endroit pour *découvrir* une casse, le
commit étant déjà celui qu'on voulait publier : ce portail double le filet
hebdomadaire au moment où l'erreur coûte le plus cher, il ne le remplace pas.

Ce que le premier run Windows a appris, chiffres à l'appui : `cargo check`
18 min, `clippy` 16 s derrière lui, `cargo test` **37 min**. Les deux premiers
s'arrêtent aux métadonnées ; seul `cargo test` produit du code machine et lie
les binaires. Retirer le `check` séparé ne gagne donc presque rien — le coût
est ailleurs, et il se réduit autrement : un profil `ci` qui compile les
dépendances en O0 au lieu de O2, sans informations de débogage, et une
exclusion Defender sur les runners Windows, où l'antivirus inspecte chacun des
dizaines de milliers de fichiers que rustc écrit.

Il n'y a pas d'équivalent local pour Windows : Docker sur un Mac lance des
conteneurs Linux, un conteneur Windows exigeant un hôte Windows.
`scripts/verifier-linux.sh` rejoue la branche Linux, et c'est tout ce qu'on
peut rejouer.

Sur un dépôt public, les runners standard sont gratuits et illimités : les
trois systèmes tournent à chaque poussée. Tant que le dépôt était privé, ce
n'était pas tenable — les minutes y sont comptées avec des multiplicateurs, ×1
sur Linux, ×2 sur Windows, ×10 sur macOS, si bien qu'un seul run complet à
froid coûtait près de 400 minutes facturées sur un quota mensuel de 2 000. Si
le dépôt redevenait privé, il faudrait remettre le dosage que l'historique
garde.

## Ce que maxx ajoute à un projet

Six choses s'ajoutent à un projet existant, par insertion textuelle et jamais
par réécriture depuis le gabarit : une vue, la barre de menus, le module
système, les réglages, les images, la fenêtre. Les réglages et la fenêtre
tirent le module système avec eux et déclarent deux crates dans le
`Cargo.toml` du projet — insérées dans la section des dépendances, pas à la fin
du fichier, pour qu'un bloc `[profile]` reste après elles. Le projet peut être
antérieur à maxx et faire autre chose au démarrage — il doit le garder.

Les images et la fenêtre sont les deux premiers modules que `main.rs` **appelle**
et pas seulement déclare : `.with_assets(assets::Assets)` sur
`Application::new()`, `window::bounds(bounds)` et `window::remember(&window, cx)`
autour de l'ouverture. D'où la forme du câblage — une reliaison qui masque
plutôt qu'un argument, une instruction plutôt qu'un appel imbriqué : chaque
ligne écrite doit être une ligne dont le retrait laisse un fichier qui compile,
parce que supprimer le module retire la ligne. Le module des images porte en
plus un `build.rs`, qui n'est pas suivi dans `maxx.toml` : le contrat entre les
deux — `assets.rs` dans `OUT_DIR`, le symbole `ASSETS` — est écrit dans
l'en-tête de chacun.

Le module système mérite sa règle : il ne contient que ce qui diffère d'un
système à l'autre **et** que gpui ne fournit pas déjà. Le presse-papier,
`open_url`, `reveal_path`, `open_with_system`, les sélecteurs de fichiers sont
dans gpui ; les enrober ajouterait une couche à maintenir pour rien. Reste où
vont les fichiers d'une application et ce que « supprimer » veut dire — ce que
toute application de bureau finit par écrire, et que personne ne veut écrire
une troisième fois.

Symétrie nécessaire : supprimer `src/<module>.rs` depuis l'explorateur retire
sa ligne `mod` de `main.rs`, et avec elle le câblage que le module s'était
donné — la table `scaffold::WIRING` dit, par module, les instructions entières
à retirer et les fragments à ôter de la ligne qui les porte. Sans quoi
supprimer un fichier casse la compilation, ce qui est l'inverse du but.

**Une copie est une dette**, et il faut la nommer : le module système et les
réglages reprennent du code que maxx a écrit pour lui-même. Un défaut trouvé
d'un côté doit être porté de l'autre. C'est déjà arrivé — la fiche
`.trashinfo`, non conforme dans les deux à la fois.

`maxx.toml`, versionné à la racine du projet, rend cette dette rattrapable. Il
note quel module a été copié, dans quelle version, et l'empreinte qu'il avait
en sortant. maxx sait alors quels projets portent une version qu'il a depuis
corrigée, et l'empreinte lui dit si le développeur y a touché : **un fichier
modifié n'est jamais remplacé**, il est signalé. C'est une troisième voie entre
l'extraction d'un crate — qui casserait la promesse « un projet généré ne doit
rien à maxx » — et la génération du gabarit depuis le code de maxx.

Le même fichier porte **le projet lui-même**, et pas seulement ce qu'il a
emprunté : la vue sur laquelle sa fenêtre s'ouvre, et la ligne cargo qui le
lance — profil, features, arguments passés à l'application. Les deux étaient
écrits dans la pierre : la vue d'entrée dans `main.rs`, le lancement en `cargo
run` nu. Un projet qui voulait `--release`, une feature ou un autre premier
écran n'avait nulle part où le dire. Chaque clé est facultative, et un projet
qui n'en pose aucune se comporte exactement comme avant.

Déplacer la vue d'entrée écrit à deux endroits qui doivent s'accorder :
`src/main.rs`, qui ouvre réellement la fenêtre, et `maxx.toml`, où maxx la
relit. Le code d'abord — un `maxx.toml` qui annonce une entrée que le code
n'ouvre pas serait pire que pas d'enregistrement du tout. Et c'est le site de
construction qui fait foi, pas la ligne `use` : un `main.rs` peut importer
plusieurs vues, une seule est confiée à `Root`.

**Les commentaires de la zone gérée sont dans le modèle.** `syn` les jette —
ce ne sont pas des jetons —, et `codegen` réécrit la zone depuis le modèle :
tout ce que le modèle ne porte pas est effacé au premier enregistrement. Le
lecteur balaie donc le texte de la zone avant de le donner à `syn`, en sautant
chaînes, chaînes brutes et littéraux de caractère, puis attribue chaque
commentaire à **ce qui le suit** — le parcours de la chaîne se fait dans
l'ordre du fichier, et une file de commentaires se vide à mesure. D'où trois
places dans le modèle : au-dessus d'un appel (`Call::comments`), au-dessus d'un
nœud (`Node::comments`, écrites par le parent qui connaît la colonne), et après
le dernier appel (`Node::trailing`).

Deux pièges que le mécanisme doit éviter et que les tests tiennent : un
commentaire écrit **dans** un argument — une fermeture, un `match` gardé
verbatim — est déjà dans le texte de cet argument, donc il est retiré de la
file sans être gardé, sinon il serait écrit deux fois ; et une chaîne qui porte
un commentaire n'est jamais rendue sur une seule ligne, un commentaire n'ayant
nulle part où aller sur une ligne unique.

**Deux appels ne vivent pas sur un élément ordinaire** : le défilement et
l'infobulle. gpui ne les offre qu'à un élément *avec état*, c'est-à-dire une
fois `id` posé — écrits dans l'autre ordre, ils ne compilent pas, dans le
projet du développeur et sur une ligne que maxx a écrite. D'où leurs cibles à
eux, `Target::Scrollable` et `Target::Tooltip`, qui posent l'`id` avant. Et
d'où `Common::Element`, que seules la colonne, la ligne et l'espaceur portent :
aucun composant de `gpui-component` n'est un élément gpui, donc l'infobulle
posée sur tous serait un appel qui n'existe pas.

**Ce que le catalogue importe** est conditionnel depuis qu'il écrit des
variantes. Un appel comme `.primary()` ou `.disabled(…)` vient d'un trait, et
un trait doit être en portée — mais l'importer dès qu'on voit le composant
laisse un `use` inutilisé sur le bouton qui n'a pas de variante, donc un
avertissement dans un projet que maxx vient d'écrire. `Spec::extra_imports`
tient donc des paires « ces appels demandent cette ligne », et la condition est
par composant : `outline` est une variante de bouton *et* un drapeau de
pastille, si bien qu'une table d'appels commune importerait le trait du bouton
dans un fichier qui n'a que des pastilles.

**Les formes de projet** sont la troisième sorte de code que maxx écrit, après
les vues qu'il dessine et les modules qu'il copie. `src/ui/shell.rs` — une
barre latérale et la vue du moment — est du Rust ordinaire écrit une fois à la
création : ni marqueurs `maxx:`, ni version, ni rattrapage. C'est voulu, et
c'est la différence avec un module : une coquille est la forme du projet, que
le développeur va faire sienne dès la première page ajoutée.

Elles posaient un problème que les vues n'ont pas : rien ne les compilait.
`src/scaffold/templates.rs` ne dépend de rien, si bien que `build.rs`
l'inclut, appelle les mêmes fonctions que les projets reçoivent et écrit leur
sortie dans `OUT_DIR` ; `examples/shapes.rs` l'y compile, contre une vue et un
module de réglages réduits à leur surface. Un `clippy --all-targets` attrape
donc une méthode que `gpui-component` n'a pas. Ce qu'il n'attrape pas — le
câblage de `main.rs`, les crates déclarées, le module de réglages en entier —
est dans `tests/project.rs::every_shape_compiles`, ignoré par défaut parce
qu'il construit deux projets entiers.

**L'empreinte ne peut pas être les octets seuls.** Un projet formaté par son
développeur — `cargo fmt`, le geste le plus banal — n'est plus octet pour octet
celui que maxx a écrit, alors qu'aucune ligne de code n'a bougé : la mise en
page par défaut déplace dix lignes de `system.rs` et cinquante-six de
`theme.rs`, et va jusqu'à écrire `else { return; }` là où le gabarit dit
`else { return }`. maxx en concluait « modifié par le développeur » et
s'arrêtait là, sans un mot. `maxx.toml` porte donc **deux** empreintes : les
octets, et la forme — le même texte passé au `rustfmt` par défaut, la
configuration du projet ignorée, parce que ce qu'il faut ici est un étalon fixe
et non celui du moment. L'une ou l'autre suffit à reconnaître un fichier ; il
faut que les deux échouent pour qu'il soit tenu pour édité.

Le garde-fou qui fait tenir l'ensemble est dans `tests/modules.rs` : il retient
l'empreinte de chaque gabarit à sa version courante. Modifier un gabarit fait
échouer ce test, ce qui oblige à décider si la correction doit atteindre les
projets déjà écrits. Sans lui, une version ne monterait jamais et le mécanisme
serait décoratif.

## Mettre en forme ce que maxx écrit

Un réglage, éteint par défaut, passe `rustfmt` sur le fichier après chaque
écriture. `rustfmt` et non `cargo fmt` : le second formate une caisse entière,
le premier prend un fichier et trouve tout seul le `rustfmt.toml` du projet en
remontant depuis lui — les conventions du développeur l'emportent sur celles de
`codegen`.

C'est aussi le seul endroit où maxx lance un processus et l'attend, à rebours
de la règle posée ailleurs : il doit relire le fichier ensuite. Et cette
relecture n'est pas facultative — maxx tient une copie du fichier et la compare
au disque pour repérer ce qui a changé en dehors de lui. Laisser la copie
derrière ferait croire à l'enregistrement suivant que quelqu'un a touché au
fichier : maxx s'accusant lui-même.

Pourquoi allumé par défaut, et c'est le point de conception : un éditeur Rust
formate à l'enregistrement — c'est le défaut de Zed comme de rust-analyzer — et
`codegen` n'écrit pas ce que rustfmt écrirait. Vérifié plutôt que supposé :
rustfmt réécrit la zone gérée de la démo. Sans ce réglage, l'éditeur reformate
ce que maxx a écrit, maxx le réécrit à sa façon à l'enregistrement suivant, et
les deux se renvoient la balle avec un diff parasite à chaque tour. maxx
applique donc lui-même ce que l'éditeur appliquerait de toute façon.

Conséquence à énoncer honnêtement : **l'aller-retour de maxx est neutre à
rustfmt près**, et c'est cette composition-là que `tests/demo.rs` vérifie. Le
gabarit, lui, sort déjà au format de rustfmt — un projet fraîchement généré en
ressort inchangé, ce qu'un test constate.

Il reste que rustfmt met en forme le fichier entier, donc au-delà de la zone
gérée. Sur un projet déjà passé au formateur cela ne change rien ailleurs ; sur
un projet qui l'ignore, le réglage s'éteint.

## Le raccourci d'une entrée

Dans gpui, un raccourci n'est pas une propriété de l'entrée de menu : c'est une
liste séparée, `key_bindings`, que la barre lit pour afficher l'accélérateur.
Elle vit hors de la zone gérée, dans une fonction que le développeur édite aussi.

Première tentative, écartée : écrire le raccourci sur le disque au moment où on
le tape, puisque le modèle ne le portait pas. C'était faux sur quatre points à
la fois — une écriture par touche, donc une pour chaque état intermédiaire de la
frappe ; une écriture qui contournait la garde « fichier modifié en dehors de
maxx » ; un raccourci écrit pour une action que `actions!` ne déclarait pas
encore ; et un raccourci qui survivait à l'entrée renommée ou supprimée.

Ce qui les règle tous : **le raccourci est dans le modèle**, lu à l'ouverture et
posé sur l'entrée, écrit à `⌘S` avec le reste. Il voyage alors avec l'entrée
qu'on renomme ou qu'on déplace, il part avec elle quand on la supprime, et il
est écrit après `ensure_action`, jamais avant.

Deux règles de bordure. Toutes les lignes qui lient une action sont retirées
avant qu'une soit écrite, parce que gpui en accepte plusieurs pour une même
action : n'en réécrire qu'une laisserait l'ancienne vivante. Et une liaison
pour une action qui n'apparaît dans aucun menu appartient au développeur —
l'enregistrement n'y touche pas.

## La mise en forme du dépôt

maxx passe à `rustfmt`, avec une seule dérogation : `use_small_heuristics =
"Max"`, qui laisse une expression courte tenir sur sa ligne. C'est ce qui
préserve les tables de `registry.rs`, le fichier qu'on invite les autres à
étendre — y lire une liste de styles à raison d'un mot par ligne serait une
punition. Le reste est le rustfmt par défaut.

La raison de s'y plier est la même que pour les projets générés : un éditeur
Rust formate à l'enregistrement. Sans référence commune, le premier
contributeur qui ouvre un fichier dans Zed le reformate en entier et son vrai
changement se noie dedans. `cargo fmt --check` en CI clôt la question.

La démo a son propre `rustfmt.toml`, vide, et n'est donc pas concernée : elle
doit être mise en forme comme un projet généré ailleurs, au rustfmt par défaut.

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
  S'il a besoin d'une entité que la vue possède — un champ texte, une liste
  déroulante — l'entrée porte en plus un `StateSpec` : le type du champ, ses
  imports, et l'expression que `new` lui donne. C'est tout ce que `view::save`
  a besoin de savoir pour déclarer le champ et l'initialiser, et la liste des
  champs proposés dans l'inspecteur est filtrée sur ce type — proposer le champ
  d'un champ texte à une liste déroulante serait proposer ce qui ne compile
  pas.
- **Un réglage de plus** : un champ dans `settings::Preferences` avec son
  défaut, une ligne dans `documented_defaults`, puis un `SettingItem` dans
  `preferences.rs`. Le champ lit et écrit les réglages, il ne copie rien.
- **Une entrée de menu de plus** : une action dans `actions.rs`, son gestionnaire,
  et la ligne dans `menus.rs`. Une action qui porte une donnée ne peut pas venir
  de la macro `actions!` — voir `OpenRecent`.
- **Un appel système de plus** : `run.rs`, et nulle part ailleurs.
