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

Reste à aligner, par ordre de manque ressenti :

- **`⌘P`, l'ouverture rapide de fichier.** C'est le geste le plus utilisé de
  Zed, et maxx n'a que l'arborescence. La palette existe déjà et sait chercher
  par mots dans le désordre : c'est la même boîte, sur une autre liste.
- **Le glisser-déposer d'une entrée entre deux menus** — fait —, et le
  glisser-déposer dans l'arbre de structure — fait aussi. Il reste à décider
  si le canvas doit accepter un dépôt venant de l'arbre, et l'inverse.
- **La surveillance du fichier.** Zed recharge un tampon non modifié sans rien
  demander ; maxx le fait aussi, mais seulement au retour dans la fenêtre.
- **Plusieurs vues ouvertes en même temps.** Les onglets existent, la
  navigation entre eux est encore pauvre : pas de `⌘⌥→`, pas de « fichier
  précédent ».

## Les quatre chantiers, dans l'ordre

### 1. Le démarrage — de zéro à la première fenêtre

C'est là que maxx est irremplaçable, et c'est ce sur quoi il est le plus
maigre : `Fichier ▸ Nouveau projet…` écrit toujours le même squelette.

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
laissée.* Reste, du chantier : `title_bar` dans une forme de projet, et de
quoi éditer `[run]` autrement qu'à la main.

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
  offertes sur les quatre-vingt-huit, chacune dessinée sur le canvas.
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
- **Les boîtes de dialogue non plus** : `dialog`, `sheet`, `popover`,
  `notification` sont présentés impérativement, jamais enfants de la vue. Leur
  place est du côté des gestionnaires — « ce bouton ouvre une boîte » — et donc
  après le chantier 1, pas pendant.

Hors de portée, et c'est un choix : `Table`, `tree`, `list`, `virtual_list`
sont pilotés par un `Delegate` que l'utilisateur écrit ; `chart` et `plot` sont
des données ; `webview` est une dépendance lourde.

### 3. Le confort — la parité Zed

- **Renommer une vue.** Toutes les pièces existent (`scaffold::create_view`,
  `workspace::unregister_view`, `view_module`, `explorer::entry_view`) ; il
  manque la commande qui les enchaîne, et la décision sur les occurrences que
  maxx ne possède pas — les dire et s'arrêter, comme partout ailleurs.
- **`⌘P`.** La palette sait déjà chercher par mots dans le désordre et sans
  accents ; il lui faut une seconde source de lignes.
- **La surveillance du fichier (`notify`).** `reload_untouched` fait déjà le
  travail sûr, il ne manque que le déclencheur. Effet : maxx et Zed côte à
  côte, on tape dans Zed, le canvas suit. C'est le principe « le `.rs` est la
  vérité » rendu visible, et le meilleur rapport effet/effort de la liste.
- **Les modèles de sous-arbre.** Un formulaire, une barre latérale, une carte,
  déposés d'un geste au lieu d'un `v_flex` vide. Aucune machinerie neuve : le
  presse-papier prouve que `parser::parse_expr` suffit — un modèle est un bout
  de Rust dans une table, comme le catalogue.

### 4. Sortir

Rien de technique, et c'est ce qui décide de tout le reste.

- **Un GIF dans le README.** Pour un outil visuel, l'élément à plus fort
  rendement de tout ce document. `demo/` est fait pour ça.
- **Des binaires attachés aux versions.** La CI est déjà en matrice sur les
  trois systèmes ; il manque un travail déclenché par une étiquette. C'est ce
  qui évite à l'utilisateur Linux d'avoir à installer Vulkan, Wayland et
  fontconfig pour un `cargo install`.
- **crates.io**, ensuite, surtout pour réserver le nom.
- **Un essai humain sur Linux et sur Windows.** La CI prouve que ça compile ;
  aucun test n'ouvre de fenêtre.

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
