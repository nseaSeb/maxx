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

- **Des modèles de projet à la création.** Vide / barre latérale + contenu /
  application à réglages. `Sidebar` et `title_bar` de `gpui-component` arrivent
  par là, et pas par la palette de composants : ce ne sont pas des éléments à
  déposer, ce sont des formes de projet.
- **`maxx.toml` porte le projet, pas seulement les modules.** Aujourd'hui la
  vue d'entrée est écrite en dur dans `main_rs()` et la commande de lancement
  est toujours `cargo run`, sans profil ni features. C'est le même fichier qui
  les accueillera.
- ~~**Le composant image**~~ — fait. `gpui::img` accepte un `&str`, cherché
  dans l'`AssetSource` que le projet généré ne déclare pas, ou un chemin lu
  depuis le répertoire courant : c'est le chemin **relatif à la racine** qui
  fait tenir le canvas et le binaire ensemble, puisque `cargo run` démarre là.
  D'où `Kind::Path`, écrit `PathBuf::from("…")` — la seule propriété qui n'est
  ni un littéral ni une liaison `&self.` —, un chemin absolu refusé plutôt
  qu'écrit, et un bouton *Choisir* qui rend relatif ce que le sélecteur donne.

  Une image prise ailleurs est copiée dans `assets/images/` : le projet porte
  ses images, ou elles ne s'affichent que sur la machine qui les a choisies.

  Reste **`assets.rs`**, et ce n'est plus optionnel : le chemin est lu depuis
  le répertoire courant du processus, donc l'image s'affiche sous `cargo run`
  lancé à la racine et pas sur un binaire double-cliqué. Le module qui déclare
  un `AssetSource` est ce qui embarque l'image dedans.
- **`window.rs`.** maxx retient la géométrie de sa fenêtre ; toute application
  de bureau la veut, et personne n'a envie de la réécrire.

Critère de fin de chantier : *créer un projet, y déposer une image, le lancer,
le fermer, le rouvrir — et retrouver la fenêtre où on l'avait laissée.*

### 2. Le catalogue — ce qu'on peut déposer

Par coût croissant, pas par ordre alphabétique.

- **Gratuits, la machinerie existe.** `Slider` et `ColorPicker` prennent une
  `&Entity<…State>` : c'est exactement le `StateSpec` du champ texte et de la
  liste déroulante. `Skeleton` et `Spinner` sont un `new()` sans argument.
  Quatre lignes de table.
- **`Kind::Enum`, une pièce pour trois usages.** L'icône, les variantes de la
  pastille, celles du bouton. C'est la seule sorte d'argument qui manque au
  catalogue tel qu'il est.
- **Une entrée qui refuse `COMMON`.** `Badge` n'implémente pas `Styled` : les
  propriétés communes ne compilent pas dessus. Un drapeau sur `Spec`, et le
  problème est nommé une fois pour toutes.
- **Les emplacements multiples.** `Accordion`, `Collapsible`, `form`, `tab`,
  `description_list` sont des conteneurs à *deux* contenus — un titre et un
  corps — là où `Node` n'a qu'une liste d'enfants. C'est le seul manque
  structurel du modèle, et il vaut d'être décidé plutôt que contourné.
- **L'infobulle n'est pas un nœud** mais un décorateur (`.tooltip(…)`, qui
  prend une fermeture) : sa place est une propriété de `COMMON`.
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
