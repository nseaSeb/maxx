# Roadmap

Ce que maxx cherche à devenir, dans quel ordre, et à quoi on dit non.

`BACKLOG.md` dit ce qui est connu et remis à plus tard, décision par décision.
Ce fichier-ci dit *pourquoi* on prend ces décisions dans cet ordre.

## Le pari

maxx n'est pas un générateur d'écrans. C'est **le chemin le plus court entre
« je veux une application de bureau en Rust » et un projet GPUI qui compile,
qui a déjà pris les bonnes décisions, et qui ne doit rien à maxx**.

La différence n'est pas de vocabulaire. Un RAD produit un écran et laisse le
reste — où vont les fichiers, ce que « supprimer » veut dire, comment la fenêtre
retrouve sa place, comment on met à jour le code qu'il a copié. C'est ce reste
qui coûte les trois premiers jours d'un projet de bureau, et c'est là que maxx a
quelque chose à donner que personne d'autre ne donne.

Trois principes tiennent déjà, et rien de ce qui suit ne doit les casser :

1. **Le `.rs` est la vérité.** Aucun format à maxx, aucune source parallèle.
2. **Ce que maxx écrit est du Rust ordinaire.** Le projet compile, tourne et
   s'édite sans maxx, et s'ouvre dans Zed comme n'importe quel projet.
3. **Zed est la référence d'interaction.** Un développeur qui vient de Zed doit
   s'y retrouver sans rien apprendre.

## Ce que « venir de Zed » veut dire

C'est une grille de décision, pas une déclaration d'intention. Devant un choix
d'interface, la question est : *comment Zed le fait ?*

Déjà aligné :

| Geste | Zed | maxx |
| --- | --- | --- |
| Réglages | un onglet, JSON à commentaires | fait, et seule la clé changée est réécrite |
| Palette de commandes | `⌘K` / `⌘⇧P` | `⌘K`, qui **est** la barre de menus aplatie |
| Panneau de projet | `⌘B` | fait, avec largeur retenue |
| Thème | clair / sombre / système | fait, des rôles et non des couleurs |
| Format à l'enregistrement | par défaut | fait, `rustfmt` sur le fichier |
| Fichier lu, pas édité | — | le lecteur de code, `⌘E` pour retourner la vue |
| Ouverture rapide | `⌘P` | `⌘P`, la même boîte que la palette, sur les fichiers |
| Onglet suivant, fichier précédent | `⌘⌥→`, `⌃⇥` | les mêmes touches |

Reste à aligner, par ordre de manque ressenti :

- ~~**`⌘P`, l'ouverture rapide de fichier.**~~ — fait, et c'était bien la même
  boîte : `⌘K` la remplit de la barre de menus aplatie, `⌘P` des fichiers du
  projet. Le clavier, la surbrillance, le clic, la sortie sont écrits une fois
  et servent les deux — c'est ce qui les fait répondre pareil. La recherche
  porte sur le **chemin entier**, donc `ui home` trouve `src/ui/home.rs`, et la
  liste se construit à l'ouverture : bornée, elle ne peut pas figer la fenêtre
  sur le geste qui doit être instantané.
- **Le glisser-déposer d'une entrée entre deux menus** — fait —, et le
  glisser-déposer dans l'arbre de structure — fait aussi. Il reste à décider
  si le canvas doit accepter un dépôt venant de l'arbre, et l'inverse.
- ~~**La surveillance du fichier.**~~ — fait : maxx recharge un tampon non
  modifié sans rien demander, et sans attendre le retour dans la fenêtre.
- ~~**Plusieurs vues ouvertes en même temps.**~~ — fait : `⌘⌥→` et `⌘⌥←`
  parcourent la bande comme un anneau — s'arrêter au dernier onglet ne ferait
  rien précisément quand il y a où aller —, et `⌃⇥` revient au fichier
  précédent, deux fois de suite pour repartir. Ce qui est retenu est le
  **chemin**, pas l'indice : fermer un onglet décale tous les suivants, et un
  indice gardé au travers d'une fermeture nomme ce qui a pris sa place.

## Ce qui passe avant le reste

**Un commentaire écrit dans la zone gérée y reste** — fait. Il disparaissait au
`⌘S` suivant, silencieusement, sur les mots du développeur : le seul défaut qui
contredisait la promesse « un projet prêt à être codé », puisqu'il punissait
exactement le geste que maxx cherche à rendre possible. Aucun composant de plus
au catalogue ne rattrapait ça.

## Les quatre chantiers, dans l'ordre

### 1. Le démarrage — de zéro à la première fenêtre — clos

C'est là que maxx est irremplaçable, et c'est là qu'il était le plus maigre :
`Fichier ▸ Nouveau projet…` écrivait toujours le même squelette. Le chantier
est terminé — le détail des points est plus bas, et le critère de fin est
atteint.

- ~~**Des modèles de projet à la création.**~~ — fait pour deux des trois
  formes. *Vide*, *Barre latérale et contenu*, *Avec des réglages*, sous
  `Fichier ▸ Nouveau projet`. `Sidebar` arrive par là et non par la palette :
  ce n'est pas un élément à déposer, c'est ce à quoi le canvas s'accroche. La
  coquille (`src/ui/shell.rs`) est du Rust écrit une fois, pas une vue que maxx
  dessine ; les pages qu'elle tient, elles, restent des vues que maxx dessine.
  `title_bar` n'est pas encore de la partie.

  Ce que maxx écrit là ne passait par aucun compilateur : `build.rs` écrit
  désormais les mêmes gabarits dans `OUT_DIR`, où `examples/shapes.rs` les
  compile à chaque `cargo clippy --all-targets`. Et `tests/project.rs` porte,
  ignoré par défaut, le test qui construit les deux projets pour de bon.
- ~~**`maxx.toml` porte le projet, pas seulement les modules.**~~ — fait. Deux
  sections facultatives : `[project] entry` dit la vue sur laquelle la fenêtre
  s'ouvre, `[run]` dit le profil, les features et les arguments de
  l'application. Un projet qui n'en pose aucune est lancé comme avant.

  Déplacer l'entrée écrit à deux endroits qui doivent s'accorder — `main.rs`
  d'abord, le fichier ensuite —, et c'est le site de construction
  (`::new(window, cx)`) qui dit quelle vue est l'entrée, pas la ligne `use` :
  un `main.rs` peut en importer plusieurs, une seule est confiée à `Root`. Le
  type est lu dans le fichier de la vue et non déduit de son nom, sans quoi une
  vue adoptée d'ailleurs serait importée sous un type qui n'existe pas.
- ~~**Le composant image**~~ — fait. `gpui::img` accepte un `&str`, cherché
  dans l'`AssetSource` que le projet généré ne déclare pas, ou un chemin lu
  depuis le répertoire courant : c'est le chemin **relatif à la racine** qui
  fait tenir le canvas et le binaire ensemble, puisque `cargo run` démarre là.
  D'où `Kind::Path`, écrit `PathBuf::from("…")` — la seule propriété qui n'est
  ni un littéral ni une liaison `&self.` —, un chemin absolu refusé plutôt
  qu'écrit, et un bouton *Choisir* qui rend relatif ce que le sélecteur donne.

  Une image prise ailleurs est copiée dans `assets/images/` : le projet porte
  ses images, ou elles ne s'affichent que sur la machine qui les a choisies.

- ~~**`assets.rs`**~~ — fait, et la source d'une image est devenue une chaîne
  nue : `img("assets/images/logo.png")` est la seule graphie que gpui cherche
  dans l'`AssetSource`, là où un `PathBuf` est lu depuis le répertoire courant
  du processus. Le module le déclare, un `build.rs` embarque `assets/` et
  `icons/` dans le binaire, et le repli disque sert ce qui a été ajouté depuis
  la dernière compilation. Il s'ajoute tout seul au premier enregistrement
  d'une vue qui dessine une image : une chaîne sans source derrière ne dessine
  rien et ne le dit qu'au journal.

  Les projets écrits avant gardent leur `PathBuf::from(…)`, que `write`
  préserve plutôt que de le convertir — les deux graphies ne veulent pas dire
  la même chose à l'exécution.
- ~~**`window.rs`**~~ — fait. Son propre `state.json` à côté de
  `settings.json` : les réglages sont ceux de l'utilisateur, l'état est celui
  de la machine. Deux crochets, `on_window_should_close` et `on_app_quit`, pour
  les deux sorties, et rien par image.

~~Critère de fin de chantier~~ — atteint : *créer un projet, y déposer une
image, le lancer, le fermer, le rouvrir — et retrouver la fenêtre où on l'avait
laissée.*

Les deux points qui restaient sont faits, et le chantier est clos :

- ~~**`title_bar` dans une forme de projet.**~~ — fait. Les deux formes qui ont une coquille
  — *Barre latérale et contenu*, *Avec des réglages* — dessinent leur propre
  barre de titre ; la forme *Vide* garde celle du système, n'ayant rien à mettre
  dans une barre à elle. La décision se prend à deux endroits qui doivent
  s'accorder : `TitleBar::title_bar_options()` dans `main.rs`, qui rend la barre
  du système transparente, et `gpui_component::TitleBar` dans `shell.rs`, qui
  est ce qu'on dessine à sa place. L'un sans l'autre se voit au premier coup
  d'œil — une barre doublée, ou une bande nue là où sont les feux tricolores —,
  d'où le drapeau porté par le gabarit de `main.rs` plutôt que déduit ailleurs.

  Le nom d'une page est désormais écrit une fois, dans `Page::label` : la barre
  latérale et la barre de titre le lisent au même endroit, et ne peuvent plus
  finir par se contredire.

- ~~**Éditer `[run]`.**~~ — fait. Une page *Exécution* dans l'écran des réglages, et non un
  écran de plus : quatre champs ne valent pas une deuxième barre latérale. Ce
  qu'elle porte est dit plutôt que dessiné — ce sont les réglages du **projet**,
  pas ceux de l'utilisateur, et ils partent dans son `maxx.toml`. La commande
  qui sera lancée s'affiche dessous, telle que le panneau de sortie l'écrira.

  Un champ est écrit quand on le quitte, pas à chaque frappe : un `maxx.toml`
  réécrit par caractère serait un fichier passant par tous les états d'un nom à
  moitié tapé. Et `⌘R` écrit d'abord ce qui traîne dans une boîte encore
  focalisée — sans quoi changer le profil puis lancer sans quitter le champ
  lancerait le précédent.

  La traduction entre ce qu'une personne écrit et ce que la section porte —
  un profil vide qui veut dire « aucun », une liste à séparateurs — vit dans
  `projectfile`, avec la section, où elle se teste sans fenêtre.

### 2. Le catalogue — ce qu'on peut déposer

Par coût croissant, pas par ordre alphabétique.

- ~~**Gratuits, la machinerie existe.**~~ — faits, et pas tout à fait gratuits :
  `Slider` et `ColorPicker` ont bien demandé le `StateSpec` du champ texte,
  mais ils ont révélé que `Select` se posait sans liaison — `Select::new()` ne
  compile pas — parce que la règle était écrite `id == "input"` et non « ce
  composant a un état ». Un `SliderState` n'a pas non plus besoin de `window`,
  d'où un `_window` renommé pour rien et un avertissement dans le projet.
- ~~**`Kind::Enum`, une pièce pour trois usages.**~~ — fait, mais ce n'était pas
  une pièce, c'en était deux : les variantes du bouton étaient déjà une famille
  de méthodes, celles de la pastille passent par `with_variant` — une méthode,
  et non les constructeurs `Tag::primary()` qui changeraient la base du nœud —
  et l'icône a demandé la vraie nouveauté, `Target::VariantArg` : une variante
  en **argument du constructeur**, `Icon::new(IconName::Check)`, qui n'a pas
  d'état vide puisque le composant ne compile pas sans elle. Vingt-deux icônes
  offertes alors sur les quatre-vingt-six — le compte disait quatre-vingt-huit
  et il était faux —, chacune dessinée sur le canvas ; les quatre-vingt-six
  sont arrivées plus bas, engendrées.
- ~~**Une entrée qui refuse `COMMON`.**~~ — faite, et elles sont deux : `Badge`
  comme prévu, et `Spinner`, qui n'implémente pas `Styled` non plus. Le
  `Common::None` existait déjà ; ce sont ses deux premiers usages.

  Au passage, ce lot a corrigé ce que maxx laissait dans les projets : les
  traits n'étaient importés que sur la vue du composant, donc un bouton sans
  variante emportait un `ButtonVariants` inutilisé. Les `use` d'un trait sont
  désormais conditionnés à l'appel qui les demande (`Spec::extra_imports`), et
  `tests/project.rs` exige que le projet compile **sans un avertissement**.
- ~~**Les emplacements multiples.**~~ — décidé : **non**, et la mesure est la
  raison. `Accordion`, `Collapsible`, `form`, `description_list` prennent bien
  un élément en argument (`.title(…)`, `.content(…)`, `.label(…)`) là où `Node`
  n'a qu'une liste d'enfants. On pouvait le modéliser : le marqueur
  `CHILD_SLOT` existe déjà, un emplacement nommé se serait écrit `"\0title"`,
  et `children` serait resté plat — donc `Path` intact, donc rien à réécrire
  ailleurs. Coût estimé honnête, autour de quatre cents lignes réparties sur le
  modèle, le parseur, le codegen, le canvas et les zones de dépôt.

  Ce que ça aurait acheté, mesuré et non supposé : le **glisser-déposer** de
  quatre composants de queue. Le code, lui, on l'a déjà — un `Accordion` écrit
  à la main, fermeture comprise, revient du parseur à l'octet près :

  ```rust
  Accordion::new("a").item(|item| item.title(Label::new("T")).child(Label::new("B")))
  ```

  Payer la première vraie complication du modèle — deux sortes d'enfants, qui
  fuiraient dans la suppression, le copier-coller, l'annulation — pour un
  confort de dépôt sur quatre entrées rares, non. « Le `.rs` est la vérité » va
  jusque-là : ce qui ne se dépose pas s'écrit, et maxx ne le casse pas. À
  rouvrir si l'usage le réclame.

  Deux faits qui ont fermé la question au passage : `Accordion` (le groupe)
  prend ses items par `.item(|item| …)`, **une fermeture**, donc restait hors
  de portée dans tous les cas ; et `form` s'atteint par `v_form()`, le module
  `form` étant privé.
- ~~**La barre de défilement visible.**~~ — faite, et sans le mécanisme qu'on
  croyait nécessaire. Ce n'était pas « deux éléments pour un nœud » : c'est
  **deux nœuds**, ce que l'arbre sait déjà porter. Une propriété *Barre
  visible* sur la colonne et la ligne écrit d'un coup ce que la composition
  demande — `relative()`, `track_scroll(&self.…)`, et par-dessus un `div`
  positionné qui tient le `Scrollbar` lié au **même** `ScrollHandle`, la forme
  avec laquelle `gpui-component` monte les siennes.

  La forme n'est pas un choix : dans gpui, `Div::prepaint` décale **tous** ses
  enfants du décalage de défilement, y compris un enfant positionné en absolu.
  Une barre écrite *dans* la boîte défile donc avec le contenu et sort de
  l'écran — c'est la première version, prise en revue. La barre est un **frère**
  de la boîte, sous une enveloppe `relative`, et rien de la boîte ne bouge :
  elle garde son style, son `gap`, ses enfants et son identifiant. C'est
  précisément ce que l'enveloppe achète, là où déplacer la mise en page sur un
  élément interne est ce qui fait perdre le `gap` à `overflow_y_scrollbar()`.

  Un seul interrupteur pour tout cela, à rebours des autres propriétés, et
  c'est le but : assembler ça à la main demanderait trois entrées du catalogue
  et un nom de champ tapé deux fois — c'est-à-dire de savoir ce que maxx sait.
  Ce qui est écrit reste du Rust ordinaire : des nœuds de l'arbre, qu'on édite
  et qu'on supprime comme les autres.

  `ScrollableElement::overflow_y_scrollbar()` reste écarté pour la raison
  vérifiée à l'écran : `Scrollable::render` déplace le style vers son
  enveloppe, l'élément qui porte les enfants retombe sur `Display::Block`, et
  le `gap` comme l'alignement du conteneur sont perdus.
- ~~**L'infobulle n'est pas un nœud**~~ — faite, et sa place n'était pas
  `COMMON` : `.tooltip(…)` vit sur un élément **avec état**, donc gpui ne
  l'offre qu'après `id`, exactement comme le défilement. Vérifié au
  compilateur : ni `v_flex().tooltip(…)` ni `Label::new("x").tooltip(…)`
  n'existent. Elle est donc une propriété de `Common::Element`, que seuls la
  colonne, la ligne et l'espaceur portent — les trois entrées du catalogue qui
  sont des éléments gpui.

  Pour les autres, il faudrait envelopper le composant dans un `div` : un nœud
  dessiné en deux éléments, soit le même manque que les emplacements multiples
  ci-dessous. C'est là que ça se décidera.
- ~~**Les boîtes de dialogue non plus**~~ — faites, pour trois des quatre, et
  la place annoncée était la bonne : le gestionnaire. Le champ *Action* d'un
  bouton porte maintenant trois boutons — *ouvre dialog*, *ouvre sheet*,
  *ouvre notification* — qui écrivent dans la méthode le corps qui présente la
  boîte, avec ses imports, et qui délient les deux paramètres que le talon
  laissait inutilisés.

  Un corps **vide** seulement. La règle que suit `ensure_handler` tient ici pour
  une raison plus forte encore : ce n'est pas un talon qu'on pose, c'est une
  méthode déjà sur le disque, et ce que le développeur y a écrit *est* le
  fichier.

  Écrit droit au fichier et non dans l'arbre : un gestionnaire n'est pas dans la
  zone gérée, il est à côté — il n'y a donc rien que `⌘S` puisse porter. La vue
  est relue après coup, pour la raison que donne `format_after_save`.

  **`popover` reste dehors, et le point de la feuille de route était mal
  classé** : ce n'est pas une boîte impérative. `Popover::new(id).trigger(…)
  .content(…)` est un élément déclaratif, enfant de la vue, avec **deux**
  emplacements — un déclencheur et un contenu. Sa place serait donc le
  catalogue, et il bute exactement sur « les emplacements multiples », auxquels
  ce document a déjà dit non. Le sortir demanderait de revenir sur cette
  décision-là, pas d'écrire un gestionnaire de plus.

  Les trois corps sont compilés là où ils sont écrits : `build.rs` les enveloppe
  dans les deux paramètres d'un gestionnaire et `examples/shapes.rs` les
  compile, comme il le fait déjà des formes de projet.

Hors de portée, et c'est un choix : `Table`, `tree`, `list`, `virtual_list`
sont pilotés par un `Delegate` que l'utilisateur écrit ; `chart` et `plot` sont
des données ; `webview` est une dépendance lourde.

### 3. Le confort — la parité Zed

- ~~**Renommer une vue.**~~ — fait, et la décision a été celle annoncée : les
  occurrences que maxx ne possède pas sont **dites et laissées**. maxx connaît
  trois endroits par construction — le fichier, sa ligne dans `src/ui/mod.rs`,
  et le site d'entrée de `main.rs` quand c'était la vue d'entrée. Partout
  ailleurs, l'ancien nom peut être un champ, un commentaire ou une chaîne, et un
  outil qui réécrit ça est un outil qu'on ne laisse pas approcher deux fois.

  Ce n'est pas un confort : toute vue créée par maxx s'appelle `view_1`,
  `view_2`… La renommer est le pas entre une vue faite et une vue nommée. La
  boîte est dans un panneau *Vue*, au-dessus de l'état.

  Le fichier est écrit avant que sa déclaration ne bouge — un `pub mod` qui
  pointe sur un fichier absent est un projet qui ne compile plus, et la fenêtre
  peut se fermer entre les deux écritures. La ligne de `mod.rs` est réécrite sur
  place plutôt que retirée et redéclarée : elle garde la position que le
  développeur lui a donnée. Et le type est remplacé en tant qu'**identifiant
  entier** : renommer `Home` en `Start` ne doit pas faire de `HomePage` un
  `StartPage`.
- ~~**`⌘P`.**~~ — fait, et ce point était périmé : voir plus haut, c'est la
  même boîte que `⌘K`, remplie d'une seconde source de lignes.
- ~~**La surveillance du fichier (`notify`).**~~ — fait, et c'était bien le
  seul déclencheur qui manquait : `check_disk` faisait déjà le travail sûr —
  recharger ce que le designer n'a pas touché, mettre le reste en conflit — mais
  n'était appelé qu'au retour dans la fenêtre. maxx et Zed côte à côte, on tape
  dans Zed, le canvas suit sans qu'on clique dans maxx. Le principe « le `.rs`
  est la vérité » rendu visible.

  Ce qui est surveillé est `src/` en entier, plus les fichiers de la racine, et
  **pas** la racine en entier : sur Linux une veille récursive pose une veille
  par répertoire, et `target/` seul peut épuiser `max_user_watches` et emporter
  la veille entière. Le prix, et il est réel : une image déposée dans `assets/`
  depuis l'extérieur n'est vue qu'au retour dans la fenêtre — qui reste, comme
  filet, pour tout ce que la veille ne voit pas.

  Le drain **attend** son canal au lieu de le sonder : une fenêtre ouverte tout
  l'après-midi ne doit rien coûter, et une minuterie à dix coups par seconde est
  ce qui empêche une machine de se mettre au repos. Ce que le fil de `cargo run`
  peut se permettre — il finit —, la veille du projet ne le peut pas.

  Deux défauts latents sont sortis avec le second déclencheur, et c'est la vraie
  leçon : `check_disk` bumpait `revision` sur un conflit, et vidait
  `edit_snapshot` sur un conflit. Inoffensif tant qu'il ne tournait qu'au retour
  du focus — personne ne tape à ce moment-là —, destructeur dès qu'il tourne
  pendant la frappe. Un conflit ne déplace rien du côté de maxx : il ne doit ni
  reconstruire les champs de l'inspecteur, ni emporter le pas d'annulation.
- ~~**Les modèles de sous-arbre.**~~ — faits, et la prédiction tenait : aucune
  machinerie neuve. Une carte, une barre d'outils, une section, sous un titre à
  eux dans la palette, qui répondent à la même recherche. Le chemin est celui du
  presse-papier — `parser::parse_expr` lit les deux —, la source venant d'une
  table au lieu du système.

  Les trois sont **sans état**, exprès : un modèle portant `&self.champ`
  nommerait un champ que la vue n'a peut-être pas, et le collage ne relie qu'à
  des champs déjà là. Un formulaire avec de vraies zones de saisie est un modèle
  qui doit d'abord déclarer son état — c'est une autre fonctionnalité. (Elle est
  arrivée depuis, avec les cinq modèles suivants : la règle est tombée.)

  La table est écrite dans la graphie de `codegen`, et un test l'y tient : ce
  qu'un dépôt met dans le fichier est alors ce texte, caractère pour caractère,
  et non un cousin reformaté. Les expressions sont compilées là où elles sont
  écrites, comme les corps de boîtes et les formes de projet.

### 4. Sortir

Rien de technique, et c'est ce qui décide de tout le reste.

- ~~**Un GIF dans le README.**~~ — fait : `docs/maxx-demo.gif`, la démo de
  `demo/` filmée, en tête du README.
- ~~**Des binaires attachés aux versions.**~~ — décidé autrement, et ce
  point disait le contraire de ce que le dépôt fait :
  `.github/workflows/release.yml` part bien sur une étiquette `v*`,
  vérifie les trois systèmes (`fmt`, clippy, tests, puis une construction
  en release que la CI ordinaire ne fait jamais), contrôle ce que `cargo
  package` emporterait — `build.rs` et `Cargo.lock` présents, `demo/`
  absent — et ouvre la version par `gh` avec la section du CHANGELOG pour
  corps. Mais **aucun binaire n'est attaché, et c'est voulu** : un
  exécutable est une distribution au sens des licences — Apache-2.0 pour
  gpui, MPL-2.0 pour `option-ext` —, avec les mentions qui doivent
  voyager avec ; publier la source n'en demande aucune, et `cargo install`
  suffit à qui a une chaîne de compilation. `scripts/bundle-macos.sh`
  existe et n'est appelé par aucun workflow. Le README et `ARCHITECTURE.md`
  disent encore l'inverse ; c'est noté dans `BACKLOG.md`.

  La matrice et les noms de sortie sont **déclarés**, et non déduits de la
  machine : ce qui est détecté se limite à ce qui ne peut pas être écrit, le
  système du runner pour choisir un nom et l'étiquette pour la version. Le jour
  où `macos-latest` change d'architecture, une sortie « automatique » changerait
  de cible sans que personne l'ait demandé ; une matrice déclarée casse
  bruyamment, ce qu'on veut d'un portail de publication.
- ~~**crates.io**, ensuite, surtout pour réserver le nom.~~ — fait : `maxx` y
  est publié, en 0.1.0 puis 0.2.0.
- **Un essai humain sur Linux et sur Windows.** La CI prouve que ça compile ;
  aucun test n'ouvre de fenêtre.

## Le prochain cycle

Les quatre chantiers ont fait ce qu'ils promettaient : un projet se crée, se
lance, se rouvre où on l'avait laissé, et ce que maxx y écrit compile sans
avertissement. Ce qui manque maintenant ne se voit pas dans un test — ça se
voit à la dixième minute d'usage. Trois chantiers, dans l'ordre où le manque
se fait sentir, et la même règle d'arbitrage qu'avant : chacun doit
raccourcir le chemin vers le premier écran qui tourne, ou reprendre une
décision de bureau que personne ne veut reprendre.

**Où en est ce cycle.** La 0.3.0 en emporte dix-neuf points sur vingt-quatre :
tout le geste sauf la sélection multiple et le dépôt entre l'arbre et le
canvas, tout GPUI, et du côté des formes les six applications types, les
modèles de sous-arbre et `maxx new`. Restent cinq points, barrés nulle part
plus bas : la **sélection multiple**, le **dépôt arbre ⇄ canvas**, la
**galerie de formes à la création**, les **templates pris dans un dossier**,
et la **mise à jour d'une forme déjà posée**. Les trois derniers vont
ensemble — c'est le même chantier vu par trois portes — et c'est par là que
le cycle suivant commence.

### 5. Le geste — l'usage au quotidien

Tout ce que maxx sait faire s'atteint par la barre de menus ou par `⌘K`.
C'est exact, et c'est le problème : un développeur qui construit une vue a la
souris sur le canvas, et la barre de menus est à l'autre bout de l'écran. Le
seul menu contextuel est celui de l'explorateur ; le canvas, l'arbre de
structure, les onglets et la palette n'en ont pas, et aucune touche ne
déplace un nœud. Le détail est dans `BACKLOG.md`, section *Le geste*.

- ~~**Le clic droit sur un nœud**, sur le canvas comme dans l'arbre.~~ —
  fait : un menu par panneau, agissant sur la sélection que le clic droit
  vient de déplacer, comme l'explorateur le fait déjà et pour la même
  raison — `ContextMenuExt::context_menu` code en dur l'identifiant de ce
  qu'il ouvre. Les deux panneaux partagent un seul constructeur de menu,
  puisqu'ils parlent du même nœud. *Monter* et *Descendre* ont demandé la
  seule commande qui manquait, `MoveNodeUp` / `MoveNodeDown` ; *Aller au
  gestionnaire* ouvre la vue à la ligne de la méthode, ce que
  `View::method_line` savait déjà dire.
- ~~**Envelopper et désenvelopper.**~~ — fait : `⌘⌥G` enveloppe le nœud
  sélectionné dans une colonne, `⌘⌥⇧G` dans une ligne, `⌘⌥U` remplace un
  conteneur par son unique enfant. Un seul point de reprise par geste, et le
  nœud garde ses appels de style — l'enveloppe est une `v_flex()` neuve avec
  ses valeurs par défaut, pas un déménagement du style vers le haut.
  Désenvelopper refuse la racine et un conteneur à zéro ou plusieurs
  enfants, plutôt que d'en promouvoir un au hasard.
- ~~**Le clavier dans l'arbre.**~~ — fait, dans un contexte « Tree » que le
  panneau ne porte que lorsqu'il a le focus, comme « Palette » : `↑` `↓`
  parcourent les rangées dans l'ordre où elles sont peintes, `⌥↑` `⌥↓`
  déplacent le nœud parmi ses frères, `⏎` donne le curseur au champ texte
  de l'inspecteur et `⌫` supprime. `←` `→` ne plient rien : l'arbre n'a pas
  de pliage — chaque nœud est toujours une rangée —, donc les deux touches
  parcourent la profondeur, parent et premier enfant.
- ~~**Le clic droit ailleurs** : sur un onglet, sur la palette, dans
  l'éditeur de menus.~~ — fait : trois menus de plus, chacun posé sur son
  conteneur et agissant sur la sélection que le clic droit vient d'y
  déplacer. L'onglet ferme, ferme les autres, ferme celles de droite, révèle
  dans le panneau de projet, copie le chemin et ouvre dans l'éditeur ; la
  palette insère avant, après ou dans la sélection, par la route du glisser
  pour que le champ d'un composant à état soit nommé au même endroit ;
  l'éditeur de menus reprend ses propres boutons. Le menu appartient chaque
  fois à une liste et non à une rangée, donc ce qui n'en fait pas partie
  reste en dehors du conteneur : l'onglet du lecteur, les composants du
  projet, les gabarits — un menu ouvert au-dessus d'eux aurait parlé d'un
  voisin.
- ~~**Éditer le texte sur place**~~ — fait : le double-clic ouvre la saisie là
  où le texte est, prérempli et sélectionné. Le champ est une session
  d'annulation de plus, pas un mécanisme parallèle, et ce qu'il ouvre est la
  propriété que le composant dit à voix haute — la table des titres range déjà
  l'identifiant ailleurs, donc rien ici ne teste un nom.
- ~~**Les poignées**~~ — faites : bord droit et bord bas, `w(px(…))` et
  `h(px(…))`, un seul pas d'annulation par glissement. Par le même travail,
  l'aperçu montre enfin la taille posée — sur une image, cadre manquant
  compris, et sur la douzaine de composants qui étaient dessinés sans leurs
  appels de style et répondaient donc à un glissement en ne bougeant pas.
- **La sélection multiple**, `⇧`-clic et `⌘`-clic, pour supprimer,
  déplacer ou envelopper en bloc. Elle vient après le reste : chaque
  commande ci-dessus doit d'abord savoir agir sur plusieurs nœuds.
- ~~**L'annulation des saisies texte.**~~ — faite : le pas se prend à la
  sortie du champ, comme `[run]` s'écrit à la sortie du champ. Une visite
  dans un champ est un pas, `⌘S` en est une frontière, et la session porte
  le nom du champ qui l'a ouverte — gpui livre un seul événement de focus à
  tous ses auditeurs, alors remonter l'inspecteur fait parler le nouveau
  champ avant l'ancien.
- **Le dépôt entre l'arbre et le canvas**, resté ouvert plus haut :
  tranché dans ce cycle, dans un sens ou dans l'autre.
- ~~**Les deux panneaux figés**~~ — fait : la sortie et l'inspecteur de
  l'éditeur de menus ont chacun leur poignée, comme les quatre autres
  joints, et leur taille se retrouve à la réouverture.
- ~~**L'écran d'accueil**~~ — fait : chaque projet récent est une carte
  qui montre sa vue d'entrée, dessinée par le canvas depuis le fichier, et
  le bouton « Ouvrir la démo » paraît quand le dépôt est là pour la donner.

Critère de fin : *construire une vue de dix nœuds sans ouvrir la barre de
menus ni `⌘K`, et sans lâcher la souris pour aller à l'inspecteur.*

### 6. GPUI — ce que le canvas sait dessiner et écrire

Deux moitiés, qui ne se ressemblent pas. La première est le style de gpui,
ce que `Styled` offre sur n'importe quel élément : le catalogue n'en écrit
que quatre propriétés communes — largeur, hauteur, fond, arrondi —, trois de
texte, et six sur les colonnes et les lignes. Tout le reste — une marge, une
bordure, une largeur maximale — se tape dans Zed. La seconde est ce que
`gpui-component` sait construire et que la palette n'offre pas encore.

- ~~**Le style, par ordre de manque.**~~ — fait : quatre propriétés
  communes sont devenues dix-sept, et trois de texte huit. Les marges
  s'arrêtent à `m`, `mx`, `my` — les quatre côtés seuls doublaient les
  rangées pour dire ce que deux disent déjà —, et l'inspecteur a gagné un
  sixième titre, « Boîte », faute de quoi les deux propriétés d'un bouton
  disparaissaient sous vingt qu'il partage avec tout le monde.
- ~~**Les composants gratuits — la moitié sans état.**~~ — fait :
  `avatar`, `breadcrumb`, `kbd`, `clipboard` et la barre d'onglets sont
  dans la palette. Deux choses ont été tranchées en chemin. Le texte
  Markdown est écarté : `TextView::markdown` réclame une fenêtre et un
  contexte, or le `render` engendré laisse la sienne sous un tiret bas et
  le canvas dessine un nœud depuis une fonction qui n'en a aucune. Et les
  enfants d'un fil d'Ariane comme d'une barre d'onglets sont des types, pas
  des éléments : ils sont devenus une propriété « Éléments » — un tableau
  de littéraux, la seule écriture qu'ils acceptent d'une chaîne — plutôt
  que des nœuds, ce qui aurait demandé à l'arbre de savoir quel parent
  accepte quel enfant.
- ~~**Les composants gratuits — la moitié à état.**~~ — fait : le sélecteur
  de date, le calendrier, le champ numérique et le champ de code sont au
  catalogue avec leur `StateSpec`. Le multi-lignes n'est pas une entrée mais
  une propriété du champ texte, parce que `InputState::multi_line` construit
  l'état et non l'élément — et c'est ce qui a ouvert la seule mécanique
  nouvelle du lot : une propriété qui s'écrit dans l'initialiseur que `new`
  pose, hors de la région gérée.
- ~~**Le survol.**~~ — fait : un titre « Au survol » qui reprend six
  propriétés de style — les couleurs, l'ombre, le coin, l'opacité —, écrites
  dans la fermeture que gpui prend, lues et réécrites appel par appel, et
  montrées par le canvas puisqu'il reçoit déjà le survol. Deux choses
  tranchées : `hover` vient de `InteractiveElement`, donc il n'est offert
  qu'aux conteneurs, les seuls nœuds qui en sont un ; et six propriétés et
  non vingt-cinq, parce qu'un survol qui déplace une marge fait sauter la
  mise en page sous le curseur.
- ~~**Les quatre-vingt-six icônes.**~~ — fait, et elles sont quatre-vingt-six
  et non quatre-vingt-huit : `IconName` n'a ni `FromStr`, ni `Display`, ni
  moyen de s'énumérer en 0.5.1, donc `build.rs` lit l'énumération dans les
  sources que cargo a dépliées et écrit les deux tables — celle que
  l'inspecteur offre et celle que le canvas dessine. Elles ne peuvent plus
  diverger, et `tests/components.rs` relit l'énumération pour le prouver.
- ~~**La propriété « Éléments »**~~ — fait : les entrées de la liste
  déroulante s'écrivent dans l'initialiseur, avec la règle des gestionnaires
  — maxx réécrit ce qu'il a posé, à l'espacement près, et laisse ce que le
  développeur a changé. Pas de forme vide : une liste sans entrée irait avec
  un index sélectionné qui ne pointe sur rien.
- ~~**`svg`.**~~ — fait : `Kind::Path` sur `.path(…)`, la même copie dans
  `assets/images/`, le même refus d'un chemin absolu. Deux choses tranchées :
  `TEXT_COMMON` ne s'applique pas — la couleur seule le fait, comme pour
  `icon`, parce qu'une graisse sur un dessin est une rangée vide —, et le
  canvas le dessine avec `img`, puisque `Svg::path` passe par l'`AssetSource`
  de l'application et que celui de l'atelier répondrait avec ses propres
  fichiers.
- ~~**Suivre `gpui-component`.**~~ — fait : `tests/components.rs` lit les
  sources de la crate dans le registre de cargo, à la version que
  `Cargo.lock` épingle, et range ses soixante-deux modules en trois listes
  — offerts, écartés, à regarder. Les offerts sont *déduits* des lignes
  `use` que le catalogue écrit, donc rien à tenir à jour de ce côté ;
  0.5 → 0.6 échouera sur le module que personne n'a classé.
- **Les emplacements multiples restent fermés.** La mesure du chantier 2
  tient toujours, et les composants gratuits ci-dessus passent avant. À
  rouvrir si une forme de projet du chantier 7 en a besoin — ce qui
  serait, cette fois, une raison mesurée.

Critère de fin : *toute vue de la galerie de `gpui-component` qui n'est ni
un délégué ni un emplacement multiple se dépose depuis la palette, et une
marge ou une bordure se pose sans ouvrir Zed.*

### 7. Les templates — de zéro à une application qui ressemble à quelque chose

Les trois formes de projet répondent à « qu'est-ce qui tient quoi » ; aucune
ne répond à « qu'est-ce que ça fait ». Un projet créé aujourd'hui est une
coquille avec une page vide dedans, et les trois premiers jours qu'on
voulait épargner recommencent là : une liste et son détail, un formulaire
qui enregistre, un tableau de bord. Ce sont des décisions de bureau, et
c'est exactement ce que maxx est censé reprendre.

- ~~**Des formes qui font quelque chose**~~ — faites : six de plus, et
  `Template` en porte neuf. *Liste et détail*, *Formulaire*, *Tableau de
  bord*, *Assistant*, *Utilitaire*, *Éditeur*, chacune une coquille autour
  de pages, et chaque page une vue que maxx continue de dessiner. Ce qui a
  été tranché : la barre de titre du projet n'est plus une décision à part,
  c'est `has_shell()` — `shell.rs` est le seul fichier qui dessine un
  `TitleBar`, donc une fenêtre ouverte sans lui n'aurait qu'une bande nue.
  L'*Utilitaire* n'a pas de barre latérale, donc pas de coquille, donc la
  barre du système et une fenêtre de 480 × 360.
- ~~**Une forme déclare son état**~~ — faite : une page écrite sur le disque
  ne passe par aucun enregistrement, donc elle porte son champ et son
  initialisation elle-même, dans le même fichier que l'arbre qui le lie.
  C'est ce qui donne au *Formulaire* des zones de saisie qui saisissent, et
  à l'*Éditeur* sa zone multi-lignes.
- ~~**Des modèles de sous-arbre de plus**~~ — faits : cinq de plus, un champ
  de formulaire (étiquette, saisie, texte d'aide), un en-tête de page, une
  barre d'état, une liste vide et une rangée Annuler / OK. Les modèles ont
  gagné l'**état** au passage, et pour une ligne : le dépôt renomme comme le
  collage, et la déclaration du champ était déjà écrite — chaque
  enregistrement parcourt l'arbre pour ses liaisons. Deux dépôts du champ de
  formulaire donnent donc deux champs distincts, et non deux zones qui se
  recopient.
- **Une galerie à la création** : une vignette par forme, dessinée par le
  canvas lui-même, au lieu d'une liste de noms qu'il faut essayer pour
  comprendre.
- **Un template est un projet.** Un dossier de templates de l'utilisateur,
  ou un chemin donné : un projet maxx ordinaire, avec son `maxx.toml`,
  copié et renommé. Pas de format — le `.rs` reste la vérité, et
  « enregistrer le projet courant comme forme » en est le geste inverse.
- ~~**`maxx new <chemin> --shape …`**~~ — fait : `src/cli.rs`, sur
  `std::env::args` seul et sans dépendance. `new`, `--help` et `--version`
  répondent sur le terminal et sortent avant que gpui ne démarre, donc ni
  un script, ni une CI, ni une machine sans écran n'ouvrent de fenêtre pour
  écrire un projet. Le nom du crate vient du dernier segment du chemin,
  comme la boîte de dialogue — la même fonction sert aux deux — et
  `Template::ALL` tient la liste des formes que l'usage et le refus
  affichent. Reste l'essai humain sur Linux et Windows.
- **Une forme se met à jour.** `shell.rs` et les pages qu'une forme écrit
  doivent entrer dans `maxx.toml` avec les deux empreintes, comme les
  modules ; sinon une forme corrigée dans maxx n'atteint jamais les projets
  déjà écrits.

Critère de fin : *choisir « Liste et détail », lancer, et voir une
application qu'on n'a pas honte de montrer — sans avoir ouvert Zed.*

### Ce que ce cycle ne fait pas

Les non de plus bas tiennent, et trois choses restent où elles étaient,
parce qu'aucun des trois chantiers ne les fait avancer : **l'essai humain
sur Linux et sur Windows**, qui ne se règle pas au clavier ; **la palette
par projet** et **le diff avant de remplacer un module**, qui attendent
qu'un projet les demande. Et deux dettes qui ne changent rien à l'usage
mais qu'on note pour ne pas les redécouvrir : `fold()` est écrit deux fois
(`palette.rs`, `designer/palette.rs`), et `view.rs` est le dernier fichier
au-dessus de mille lignes.

## Ce qu'on ne fera pas

Le dire est aussi utile que le reste, parce que ce sont les demandes qui
reviendront.

- **Éditer et enregistrer du code depuis maxx.** Un `.rs` géré a déjà une
  source de vérité — le canvas — et deux écrivains sur un fichier demandent une
  politique de conflit que `Workspace::conflicts` n'esquisse qu'à moitié. Le
  lecteur lit ; `⌘⌥Z` est le chemin vers l'écriture.
- **Un format à maxx**, sous quelque forme que ce soit : ni fichier de projet
  d'écrans, ni presse-papier privé, ni base de données.
- **Remplacer Zed.** maxx est un compagnon : il pose un projet, écrit ce qu'il
  sait écrire, et s'efface.
- **Prétendre gérer les données.** Le contenu d'une liste, les colonnes d'une
  table, la source d'un graphe sont du code que l'utilisateur écrit.

## La règle d'arbitrage

Devant une idée, une seule question :

> Est-ce que ça raccourcit le chemin vers le premier écran qui tourne, ou
> est-ce que c'est une décision de bureau que personne ne veut reprendre ?

Si c'est ni l'un ni l'autre, c'est du RAD, et ça peut attendre.
