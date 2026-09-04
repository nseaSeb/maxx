# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

## Ce que la revue a trouvé

Une relecture du cycle entier, le 3 septembre, sur les 3 400 lignes qu'il
avait posées. Sept constats, tous réels, tous corrigés — plus trois trouvés
à la main avant elle, plus bas. Ce qu'il faut en retenir : la suite était
verte du premier au dernier, et aucun de ces défauts n'était visible
autrement qu'en lisant.

- ~~**Désenvelopper perdait les phrases écrites sur les appels de la boîte.**~~
  — corrigé. Les commentaires du conteneur et sa traîne étaient préservés,
  mais ses `calls` étaient jetés en bloc — et chaque `Call` porte ses propres
  `comments`. Un `// disposé en colonne pour que les deux s'empilent` écrit
  au-dessus du `.gap_2()` disparaissait au premier `⌘⌥U`, sans un mot. Les
  appels ne suivent toujours pas — un `gap` sur la boîte ne dit rien du nœud
  qu'elle tenait — mais les phrases, si, dans l'ordre de lecture.

- ~~**Les commandes d'arbre cassaient l'assemblage de la barre de
  défilement.**~~ — corrigé par un refus. La barre visible n'est pas un nœud
  mais trois, dans une forme que gpui rend obligatoire, et
  `is_scrollbar_wrapper` la reconnaît à cette forme exacte. Monter la
  surcouche la mettait en première position : l'interrupteur de l'inspecteur
  basculait alors sur *éteint* pendant que le fichier tenait toujours la
  barre, et le rallumer en écrivait une seconde. La désenvelopper sortait le
  `Scrollbar` de son `div` absolu, et le projet généré peignait la barre dans
  le flux. Les deux compilent, d'où leur survie. Refuser plutôt que réparer :
  l'assemblage appartient à un interrupteur, et une commande qui le
  reconstruirait en douce serait un second auteur.

- ~~**Un raccourci vidé disait oui et ne changeait rien.**~~ — corrigé.
  `validate` rendait `None` sur une valeur vide, donc aucune erreur ; `write`
  sortait tôt parce que `is_keystroke("")` est faux. Le champ revenait à son
  ancienne valeur à la reconstruction suivante. C'est le cas de la propriété
  d'initialiseur juste à côté : pas de forme vide, donc on le dit.

- ~~**Un `border_color` illisible supprimait la teinte *et* la bordure.**~~ —
  corrigé. La teinte d'aperçu était retenue dès qu'un appel `border_color`
  existait, y compris un `.border_color(cx.theme().border)` écrit à la main
  que l'aperçu ne sait pas évaluer — et dont le bras rend l'élément
  inchangé. Le canvas ne montrait alors aucune bordure là où l'application en
  dessine une, ce qui se lit comme une propriété cassée. La question est
  devenue « une couleur que l'aperçu peut *utiliser* ».

- ~~**La démo était cherchée jusque dans le dossier personnel.**~~ — corrigé.
  Le compte de niveaux est calibré sur `target/debug/maxx` ; depuis
  `~/.cargo/bin/maxx` les trois mêmes pas atteignent le répertoire de
  l'utilisateur, et un `~/demo` sans rapport était offert sur l'écran
  d'accueil comme étant *la* démo — le défaut exact que la borne prétendait
  empêcher. Le répertoire doit désormais porter le manifeste de maxx.

- ~~**`maxx new .` nommait le crate `mon_app`.**~~ — corrigé.
  `Path::new(".").file_name()` est `None`. Le chemin est canonicalisé avant
  d'en tirer le nom. La boîte de dialogue ne pouvait pas produire le cas, donc
  seule la ligne de commande le rencontrait, et la promesse « deux voies, une
  réponse » était fausse sans que rien ne le dise.

- ~~**Cinq fichiers de test écrivaient sous des noms fixes.**~~ — corrigés.
  `tests/{code,tools,menus,settings,cli}.rs` et deux endroits de
  `tests/scaffold.rs` ignoraient `MAXX_SCRATCH` et se piétinaient dès que deux
  `cargo test` se chevauchaient — l'échec tombant sur un test au hasard. C'est
  la cause unique de tous les faux échecs rencontrés pendant ce cycle, et ils
  ont été nombreux. Chacun a maintenant un répertoire portant son pid, ce qui
  sépare deux exécutions même sans la variable. Vérifié : deux suites
  simultanées sans `MAXX_SCRATCH` passent, ce qui était impossible avant.

## Trois défauts trouvés en chemin

Sortis de l'implémentation du cycle, pas de son inventaire. Corrigés, dits ici
parce que chacun avait une raison de survivre longtemps.

- ~~**Un composant à état déposé partageait l'entité du précédent.**~~ — corrigé.
  Deux voies mènent un composant dans l'arbre, et elles ne posaient pas la même
  question : la commande de la palette demandait au catalogue « ce composant
  a-t-il un état », le glisser-déposer demandait « son nom est-il `input` ».
  Tout autre composant à état — une liste déroulante, un curseur, un sélecteur
  de couleur — arrivait donc avec le `&self.field` sans numéro que laisse
  `instantiate`, et un second se liait à la même entité. Ça compile ; les deux
  boîtes se recopient l'une l'autre une fois le projet lancé, c'est-à-dire loin
  d'ici. Les deux voies appellent maintenant `bind_own_state_field`, écrite une
  fois, et le test porte sur la liste déroulante — pas sur le champ texte, qui
  était précisément le seul cas que l'ancien code servait bien.
- ~~**L'aperçu ne comptait les pixels que d'une entrée nommée `image`.**~~ —
  corrigé : la question se pose de la propriété (`Kind::Path`) et non du nom de
  l'entrée. L'avatar porte une image lui aussi, et serait resté sans dimensions
  pour une raison qu'aucun lecteur du code n'aurait trouvée.
- ~~**`tests/code.rs` écrivait sous des noms fixes de `temp_dir()`.**~~ —
  corrigé. Deux exécutions de la suite qui se chevauchent — un second dépôt, une
  tâche de CI à côté d'une exécution locale — et l'une supprimait le répertoire
  que l'autre lisait ; l'échec tombait sur un test au hasard. Le reste de la
  suite honorait déjà `MAXX_SCRATCH` ; ce fichier a désormais un répertoire par
  processus, ce qui sépare aussi deux exécutions quand la variable est absente.

## Le geste

Ce que `ROADMAP.md` appelle le chantier 5. Les commandes existent ; c'est la
manière de les atteindre qui manque. Chaque point ci-dessous nomme la pièce
qui existe déjà, parce que presque aucun ne demande de machinerie neuve.

- ~~**Le clic droit sur le canvas et dans l'arbre.**~~ — fait. Un menu par
  panneau, sur le modèle de l'explorateur : `ContextMenuExt::context_menu` code
  en dur l'identifiant de ce qu'il ouvre, donc jamais un menu par rangée. Le
  clic droit déplace la sélection avant que le menu ne se construise, et il le
  peut pour une raison qu'il valait la peine de vérifier dans gpui :
  l'écouteur du menu est enregistré **après** ceux qu'il enveloppe, donc servi
  le premier en remontée, tandis que la construction du menu, elle, est
  différée à l'image suivante.

  Le canvas (`designer/canvas.rs`) et l'arbre (`designer/tree.rs`) partagent
  **un seul** constructeur, `designer::node_menu`, puisqu'ils parlent du même
  nœud : Dupliquer, Copier, Coller, Envelopper dans une colonne ou une ligne,
  Désenvelopper, Monter, Descendre, Supprimer, Voir le code, Aller au
  gestionnaire. La position annoncée a tenu — rien n'est grisé d'après la
  sélection, et chaque entrée qui refuse le dit après coup dans la barre
  d'état, comme `DeleteFile`. Sur le canvas, le clic droit arrête sa remontée
  pour la raison que le clic gauche l'arrête : chaque ancêtre porte le même
  écouteur, et la racine gagnerait toujours.
- ~~**Envelopper, désenvelopper.**~~ — fait. `WrapInColumn` (`⌘⌥G`),
  `WrapInRow` (`⌘⌥⇧G`) et `Unwrap` (`⌘⌥U`), dans le menu Édition à côté de
  Dupliquer, donc dans `⌘K`. L'enveloppe est un nœud du catalogue ordinaire,
  `registry::instantiate("column")`, et le geste tient en un seul
  `checkpoint()` : le retrait et l'insertion se font après lui, donc `⌘Z`
  défait l'envelopper entier et non sa moitié. La position annoncée a tenu —
  le nœud garde ses appels, l'enveloppe naît avec ses seules valeurs par
  défaut. Désenvelopper refuse la racine et tout conteneur qui ne tient pas
  exactement un enfant, en le disant ; les commentaires écrits au-dessus de
  l'enveloppe et après son dernier appel descendent sur l'enfant promu
  plutôt que de disparaître avec la boîte. La sélection suit le nœud, pas la
  boîte : après envelopper elle est sur le nœud enveloppé, après
  désenvelopper sur l'enfant promu — donc désenvelopper juste après
  envelopper demande de remonter d'un cran, ce que le test dit tel quel.
- ~~**Le clavier dans l'arbre.**~~ — fait. Un contexte clavier « Tree », posé
  sur le panneau de structure et porté par un `FocusHandle` que le clic sur une
  rangée vient prendre : un contexte ne lie rien tout seul, gpui distribuant la
  frappe le long du chemin de l'élément **focalisé**, si bien que `↑` `↓` `←`
  `→` `⏎` `⌫` ne sont prises à personne d'autre. `↑` et `↓` parcourent les
  rangées dans l'ordre où elles sont peintes et s'arrêtent aux deux bouts ;
  `⌥↑` et `⌥↓` déplacent le nœud parmi ses frères, ce que `edits.rs` sait
  faire depuis `MoveNodeUp` / `MoveNodeDown` ; `⏎` donne le curseur au champ
  texte de l'inspecteur, et ne fait rien pour un nœud qui n'en a pas.

  Ce qui a été tranché autrement : `←` et `→` ne plient rien. L'arbre n'a
  aucun pliage — chaque nœud est toujours une rangée —, donc les deux touches
  parcourent la profondeur, le parent et le premier enfant, qui est l'autre
  chose qu'une main leur demande.
- ~~**Le clic droit sur un onglet.**~~ — fait : Fermer, Fermer les autres,
  Fermer celles de droite, Révéler dans le panneau de projet, Copier le
  chemin, Ouvrir dans l'éditeur. Le clic droit active l'onglet visé avant
  que le menu ne se construise, comme dans l'arbre — et activer, c'est aussi
  ce qui allume le fichier dans l'explorateur, donc « Ouvrir dans » n'a rien
  demandé de plus.

  Ce qui a été tranché : le menu appartient aux onglets de vue, pas à la
  bande. L'onglet du lecteur est un frère **en dehors** du conteneur qui
  porte le menu, parce qu'aucune des six entrées ne veut dire quelque chose
  d'un document qui n'est pas une vue — on ne le révèle pas parmi les vues,
  et « fermer les autres » n'a pas d'autres à nommer ; un menu ouvert
  au-dessus de lui aurait parlé d'un onglet voisin, ce qui est exactement le
  piège que `ContextMenuExt` tend en codant en dur l'identifiant de ce qu'il
  ouvre. Fermer les autres et fermer à droite passent par `close_view` un
  indice à la fois, de la droite vers la gauche : fermer décale tout ce qui
  suit, et une vue non enregistrée refuse de se fermer en le disant — ce
  refus est la raison pour laquelle la bande ne se vide pas en une ligne.
  Le chemin copié est absolu : ce qui sort de maxx va dans un terminal ou
  dans un autre éditeur, qui ne savent rien de la racine du projet.
  `tabs.rs` a reçu les trois fonctions pures que `tests/tabs.rs` tient —
  l'ordre de fermeture est tout le contenu de deux d'entre elles.

  Deux bords restent. Révéler déplie les dossiers au-dessus du fichier et
  allume sa rangée, mais ne fait pas défiler jusqu'à elle : la liste de
  l'explorateur n'a pas de poignée de défilement, et lui en donner une est un
  geste à elle. Et les six commandes parlent de la vue en avant, donc
  atteintes par `⌘K` pendant que le lecteur couvre l'écran, elles parlent de
  la vue qu'il couvre — ce qui est la position tenue plus haut, écrite ici
  parce qu'elle se voit de là.
- ~~**Le clic droit sur la palette.**~~ — fait : Insérer avant la sélection,
  après, dedans, qui passent par `drop_at` — la route que le glisser prend
  déjà, donc un composant à état reçoit ici son champ à lui exactement comme
  au dépôt, `bind_own_state_field` restant nommé à un seul endroit.

  Deux choses ont été tranchées. Le menu ne couvre que les rangées du
  catalogue : les composants du projet et les gabarits restent en dehors du
  conteneur qui le porte, parce que les trois entrées insèrent ce que l'arbre
  accepte d'un glisser et que ni une brique ni un gabarit ne se glisse. Et la
  palette a gagné une sélection à elle, `palette_target`, allumée par le clic
  droit et peinte comme telle : le menu agit longtemps après le clic qui l'a
  choisie, et un choix que personne ne voit est un menu qui agit ailleurs.
  Quand la sélection est la racine — une vue où l'on n'a pas encore cliqué —
  « avant » et « après » se lisent comme les deux bouts de ce qu'elle tient,
  plutôt que deux entrées sur trois qui refusent ; « dans » sur un nœud qui
  n'accepte pas d'enfant refuse en le disant, parce qu'une entrée qui annonce
  « dans » et dépose à côté ment sur l'endroit où le nœud est parti.

  L'éditeur de menus a reçu le sien dans le même lot, faute de point à lui :
  Monter, Descendre, Nouvelle entrée, Nouveau sous-menu, Supprimer — les
  boutons du panneau, amenés là où sont les rangées. Ajouter un sous-menu et
  supprimer n'étaient atteignables que par ces boutons ; ils ont maintenant
  `AddSubmenu` et `DeleteMenuEntry` dans le menu Édition, donc dans `⌘K`.
- ~~**Éditer le texte sur place.**~~ — fait : le double-clic ouvre un `Input`
  juste au-dessus du nœud, prérempli et sélectionné, et ce champ n'est pas un
  mécanisme de plus. Il ouvre une session de `begin_text_edit` comme celles de
  l'inspecteur, clé comprise : gpui livre un seul événement de focus à tous ses
  auditeurs dans l'ordre d'inscription, donc une session anonyme serait fermée
  par la mauvaise moitié. Sorties : perte de focus, `⏎`, changement de
  sélection — `Escape` n'en est pas une, gpui-component le laisse remonter sans
  rien émettre. Construit à l'ouverture et non par une passe `sync_…`, parce
  qu'un champ reconstruit à chaque image est un curseur perdu à chaque image.
  La position sur le rechargement a tenu : le champ se ferme et perd ce qui
  n'était pas écrit, comme `check_disk` reprend un tampon non modifié.

  Ce qui a été tranché en plus, et qui n'était pas dans l'énoncé : *quelle*
  propriété le double-clic ouvre. Ce ne peut pas être la première `Kind::Text`
  — celle d'un bouton est `prop.id` —, et ce ne sera pas un test sur un nom :
  la table `GROUPS` range déjà `prop.id` sous `Group::Action` et tout ce qu'un
  composant dit à voix haute sous `Group::Text`, donc `registry::spoken_text`
  pose la question à cette table.

  Ce que ça coûte, dit ici plutôt que découvert : le double-clic servait à
  écrire un gestionnaire, et les quatre composants du catalogue qui en portent
  un — bouton, case, interrupteur, radio — portent tous une étiquette. La règle
  « les mots d'abord, l'action à défaut » est donc écrite, mais sa seconde
  moitié ne se déclenche sur rien aujourd'hui : le raccourci est retiré, et le
  champ Action de l'inspecteur reste la route, comme il l'a toujours été.
  Reste aussi que `focus_prop_text` — le `⏎` de l'arbre — prend encore la
  première propriété `Kind::Text`, c'est-à-dire l'identifiant d'un bouton :
  `spoken_text` est la réponse, en une ligne, le jour où quelqu'un touche à
  ce fichier.
- ~~**Les poignées sur le canvas.**~~ — faites : deux, au bord droit et au bord
  bas du nœud sélectionné, et pas de coin — un coin écrirait une largeur et une
  hauteur d'un seul geste, alors que ce sont deux appels qui disent deux
  choses. Le glissement passe par le glisser de gpui, qui le garde vivant
  au-delà du bord de l'élément, et la taille se lit comme
  `gpui_component::resizable` la lit : la position du curseur moins le bord
  mesuré, ce bord étant celui de l'élément qui porte l'écouteur. Un seul pas
  d'annulation par glissement, et le checkpoint est pris au premier mouvement
  plutôt qu'à la prise : c'est le même arbre — rien ne peut arriver entre les
  deux —, et une poignée simplement cliquée ne laisse pas derrière elle un
  `⌘Z` qui ne défait rien. Une poignée n'apparaît que là où le catalogue donne
  une largeur : un composant qui n'est pas `Styled` recevrait un appel que le
  projet engendré ne compile pas.

  L'aperçu montre désormais la taille — et l'énoncé était déjà dépassé sur ce
  point : `apply` est générique depuis longtemps et s'applique bien à `img`.
  Ce qui manquait était ailleurs, et les poignées le rendaient visible à chaque
  geste : le cadre qui remplace une image absente ignorait la taille du nœud,
  et une douzaine de composants — l'étiquette, le bouton, la case, l'alerte —
  étaient dessinés sans passer par `apply` du tout, si bien qu'ils répondaient
  à un glissement en ne bougeant pas. Tous y passent maintenant, et le
  catalogue reste ce qui décide : `Common::None` n'a pas de largeur à écrire.
- **La sélection multiple.** `View::selected` est un `Path` unique, et
  chaque commande le lit. Passer à plusieurs demande de reprendre chaque
  commande une par une — supprimer, déplacer, envelopper, copier —, et de
  décider ce que fait l'inspecteur devant deux nœuds. Position :
  l'inspecteur montre le premier et le dit ; la sélection multiple sert
  aux gestes, pas aux propriétés. En dernier dans le chantier, parce que
  chaque commande ci-dessus doit d'abord exister pour un nœud.
- ~~**L'annulation des saisies texte.**~~ — faite : le pas se prend **à la
  sortie du champ**, quand la valeur est déjà dans l'arbre et que rien n'est
  sous le curseur — le moment où `[run]` est écrit dans `maxx.toml`. Une
  visite dans un champ est un pas ; changer de champ, changer de sélection
  ou `⌘S` la ferme. Deux choses ont été tranchées en chemin. La session
  porte l'identité du champ qui l'a ouverte : gpui livre un seul événement
  de focus à tous ses auditeurs, dans l'ordre où ils se sont inscrits, donc
  remonter l'inspecteur fait parler le `Focus` du nouveau champ avant le
  `Blur` de l'ancien, et une session anonyme était fermée par la mauvaise
  moitié — la saisie ne laissait aucun pas. Et la clé de `sync_prop_inputs`
  est adoptée quand le pas est posé : `revision` avance pour le panneau de
  code, sans reconstruire le champ où le curseur vient d'arriver. Reste
  qu'`Escape` n'est une sortie nulle part : gpui-component le laisse
  remonter sans rien émettre, et maxx ne l'attrape que dans la palette.
- **Le dépôt entre l'arbre et le canvas.** Resté ouvert dans la feuille de
  route. Le glisser porte déjà un `Dragged` commun aux trois sources
  (palette, arbre, canvas) ; ce qui manque est que les zones de dépôt de
  l'un acceptent l'origine de l'autre. Position : oui dans les deux sens —
  c'est le même arbre, et une zone qui refuse un nœud parce qu'il vient de
  l'autre panneau est une règle que personne ne devine.
- ~~**Les deux panneaux figés.**~~ — fait : la même pièce deux fois de plus.
  La sortie est devenue la seconde moitié d'un `v_resizable` qui tient tout
  ce qui est sous la barre de titre (120–600 px, 200 par défaut), et
  l'inspecteur de l'éditeur de menus la seconde moitié d'un `h_resizable`
  (220–560 px comme l'autre inspecteur, 280 par défaut) ; les deux panneaux
  ont perdu la taille qu'ils se donnaient — un `h(px(200.))`, un
  `w(px(280.))` — parce qu'une taille posée dedans se bat avec la poignée.
  `output_height` et `menu_inspector_width` rejoignent `panel_width` et
  `inspector_width` dans `state.json` : tenues en mémoire par `stage_state`
  pendant le glissement, écrites par `flush` à l'extinction, et absentes
  d'un `state.json` ancien elles retombent sur la valeur d'avant. Le schéma
  JSON ne couvre que `settings.json`, donc il n'avait rien à suivre.
  Ce qui a été tranché : pas de groupe redimensionnable quand la sortie est
  fermée — un groupe d'un seul volet coûte un état pour rien, exactement la
  raison déjà écrite pour le panneau de projet.
- ~~**L'écran d'accueil.**~~ — fait : chaque projet récent est une carte de
  200 px qui montre sa vue d'entrée, dessinée par le canvas lui-même depuis
  le fichier et peinte avec les couleurs du projet, comme le plan de travail
  la peint. La position a tenu : rien n'est gardé dans `state.json`, la
  vignette se lit à l'affichage et se borne à la vue d'entrée. « À
  l'affichage » a quand même demandé d'être précisé, parce que dix projets
  récents relus à chaque image seraient dix fichiers ouverts soixante fois
  par seconde : les arbres sont lus **une fois**, quand la liste change, et
  jetés dès qu'un projet s'ouvre devant eux — la liste ne bouge pas quand on
  modifie la vue qu'une carte montre, donc le retour à l'accueil relit.
  Une vue qui ne se lit pas, un `maxx.toml` qui ne nomme aucune vue et un
  projet qui a déménagé donnent la même chose : un cadre vide, jamais une
  erreur.

  Ce qui a été tranché en plus : le canvas dessine désormais un arbre sur
  deux surfaces, et la différence tient en un seul paramètre. `node_preview`
  reçoit ses enfants par une fermeture au lieu de les lire sur le nœud — le
  plan de travail y intercale ses zones de dépôt, la vignette non — si bien
  que la vignette ne porte aucun écouteur : elle montre un projet qui n'est
  pas ouvert, et le moindre clic agirait sur l'arbre d'un autre. Et « petite
  échelle » veut dire une petite boîte, pas une boîte réduite : gpui n'a pas
  de transformation d'échelle, donc c'est un cadre de 120 px au plus petit
  corps de texte, qui coupe ce qui dépasse. Le bouton « Ouvrir la démo »
  cherche `demo/maxx.toml` à côté de l'exécutable puis deux crans au-dessus
  — soit `target/debug/maxx` jusqu'à la racine du dépôt, et pas un de plus,
  sinon un dépôt posé à côté du `demo/` de quelqu'un d'autre ouvrirait le
  sien — et ne se dessine pas quand il ne trouve rien ; rien n'est copié,
  rien n'est embarqué.

## GPUI

Le chantier 6. Deux moitiés : le style que gpui offre sur tout élément, et
les composants de `gpui-component` que la palette n'offre pas encore.

- ~~**Le style commun est maigre.**~~ — fait : `COMMON` porte dix-sept
  propriétés au lieu de quatre, `TEXT_COMMON` huit au lieu de trois, et la
  colonne et la ligne deux de plus (`justify_*`, `flex_wrap`) parce que ce
  sont les deux qui ne veulent rien dire ailleurs. Ce qui a été tranché :
  les marges s'arrêtent à `m`, `mx`, `my` — `mt`/`mb`/`ml`/`mr` existent
  bien dans gpui, mais quatre familles de plus doublaient les rangées du
  titre pour dire ce que deux disaient déjà, et une marge d'un seul côté
  est la moitié rare d'une propriété déjà rare ; `border_3` est écarté
  pour la même raison, un cran entre deux qui se ressemblent. Et
  l'inspecteur a bien ouvert son sixième titre, `Group::Box`, où va tout ce
  que `Styled` pose autour de n'importe quoi — marges, bordure, minimums et
  maximums, ombre, opacité, rognage, curseur — pour que les deux propriétés
  propres d'un bouton restent lisibles. L'aperçu prête sa couleur de
  séparateur à une bordure que personne n'a colorée : gpui la dessine
  transparente, et une largeur qui ne montre rien se lit comme une panne.
- ~~**Le survol.**~~ — fait : `Target::Hover` enveloppe la cible qu'utilise
  déjà la propriété ordinaire, si bien qu'un fond survolé *est* un `bg` et
  ne peut pas s'écrire autrement. La seconde chaîne ne va pas dans `Node` :
  elle reste où le parseur la met déjà, dans le texte de la fermeture, lue
  en y plantant un `div()` à la place du paramètre et réécrite appel par
  appel — le nom que le développeur a donné au paramètre ressort intact, et
  une fermeture qu'il a écrite lui-même est montrée et laissée. Trois
  choses tranchées. `hover` vient de `InteractiveElement`, que seul un
  élément gpui est : il n'est offert qu'aux conteneurs, comme l'infobulle,
  parce que posé sur un `Label` c'est un appel qui ne compile pas. Six
  propriétés et non vingt-cinq — les couleurs, l'ombre, le coin, l'opacité
  — parce qu'un survol qui déplace une marge ou change une graisse fait
  sauter la mise en page sous le curseur, ce qui se fait à la main et
  exprès. Et l'aperçu montre bien l'état : `StyleRefinement` implémente
  `Styled`, donc c'est la fonction qui applique les appels ordinaires qui
  applique ceux-là.
- ~~**Les composants gratuits — les sans état.**~~ — fait : `avatar`,
  `breadcrumb`, `kbd`, `clipboard` et `tab_bar` sont au catalogue, chacun
  avec sa branche du canvas, son appel dans `examples/catalogue.rs` et ses
  `extra_imports`. Trois choses ont été tranchées. `text` n'entre pas :
  `TextView::markdown` prend un `&mut Window` et un `&mut App`, et les deux
  bouts le refusent — le `render` engendré laisse sa fenêtre sous un tiret
  bas, et `canvas::node_preview` n'en reçoit aucune ; il est passé dans les
  écartés de `tests/components.rs`, avec cette raison. `Tab` n'est pas une
  entrée : `TabBar::child` prend un `impl Into<Tab>` et `Breadcrumb::child`
  un `impl Into<BreadcrumbItem>`, donc un libellé déposé dedans écrirait un
  appel qui ne compile pas, et refuser ce dépôt demanderait à l'arbre une
  notion de parent qu'il n'a pas ; les deux portent leurs libellés comme
  une propriété « Éléments », un `.children(["Home", "Files"])`, la seule
  écriture que ces types acceptent d'une chaîne. Enfin les deux formes
  nouvelles — la frappe et la liste — sont des `Target`, pas des `Kind`,
  pour la raison qui a fait `Target::Tooltip` : le champ montre du texte et
  le fichier reçoit une expression autour, et un `Kind` neuf aurait dû être
  ajouté à deux listes non exhaustives dans les inspecteurs, où l'oubli
  donne une propriété qui n'apparaît jamais. `menufile::is_keystroke` est
  ce qui refuse un raccourci que gpui ne lit pas ; l'avatar a rouvert
  `Kind::Path` sur une méthode, `uses_an_asset` compris, sans quoi son
  image ne se serait affichée que sur le canvas.
- ~~**Les composants gratuits — ceux à état.**~~ — fait : `date_picker`,
  `calendar`, `number_input` et `otp_input` sont au catalogue avec leur
  `StateSpec`, leur branche du canvas et leur appel dans
  `examples/catalogue.rs` ; la liaison passe par `bind_own_state_field`,
  qui demande `spec.state.is_some()` et rien d'autre. Trois choses
  tranchées. Les deux modules du temps s'importent
  `gpui_component::date_picker` et non `time::date_picker` : la crate garde
  `time` privé et réexporte les deux depuis sa racine, si bien que le chemin
  long ne résout pas — `tests/components.rs` a gagné la table qui ramène un
  réexport à son module, sans quoi il aurait signalé trois fautes pour un
  composant bel et bien offert. `OtpInput` n'implémente pas `Styled`, donc
  `Common::None`, et `Calendar` compose sa propre typographie, donc
  `Common::Box`. Enfin le multi-lignes n'est pas une entrée mais une
  propriété du champ texte, et c'est ce qui a demandé la mécanique
  nouvelle : `Target::Initializer`, une propriété qui n'écrit rien dans la
  région et tout dans l'initialiseur que `ensure_state_field` pose — avec la
  règle des gestionnaires, l'espacement écrasé pour qu'un `cargo fmt` sur
  `new` ne passe pas pour une réécriture à la main. Elle est hors de la pile
  d'annulation, qui ne tient que des arbres, exactement là où
  `add_state_field` était déjà.
- ~~**Les icônes.**~~ — fait, et elles sont quatre-vingt-six : `IconName`
  n'a gagné ni `FromStr`, ni `Display`, ni `IntoIterator` en 0.5.1, donc la
  table ne disparaît pas, elle est engendrée. `build.rs` va lire l'énumération
  là où le compilateur la lit — les sources dépliées par cargo, à la version
  que `Cargo.lock` épingle, la même recherche que `tests/components.rs` fait,
  recopiée parce qu'un script de compilation ne s'importe pas — et écrit deux
  fichiers dans `OUT_DIR` : la liste que l'inspecteur offre, incluse par
  `catalogue.rs`, et le `match` que le canvas dessine, inclus par `canvas.rs`.
  Les deux sortant de la même lecture, elles ne peuvent plus diverger ; ce qui
  restait à tenir, c'est la lecture elle-même, et `tests/components.rs` relit
  l'énumération pour l'exiger entière. Une chose tranchée : les
  quatre-vingt-six noms ne sont plus recopiés dans `examples/catalogue.rs` —
  le `match` engendré les nomme tous, et c'est le même compilateur qui répond
  à la même question.
- ~~**La propriété « Éléments » de la liste déroulante.**~~ — fait, sur la
  mécanique que le multi-lignes a ouverte : `Target::Initializer` et non
  `Target::Labels`, parce que ce qui s'écrit là n'est pas un tableau de
  littéraux mais une suite de `SharedString::from(…)`, et parce que la valeur
  ne vit pas sur le nœud — l'inspecteur la lit dans `new` et l'y réécrit.
  Une chose tranchée : il n'y a pas de forme vide. Une liste sans entrée
  s'écrirait `vec![]` à côté d'un `Some(IndexPath::new(0))` qui pointe sur
  rien, et laisserait un `use` d'`IndexPath` inutilisé dans un projet que
  maxx vient d'écrire ; vider le champ laisse donc la ligne et le dit.
- ~~**`svg`.**~~ — fait : `Kind::Path` sur `.path(…)` — `svg()` ne prend
  aucun argument —, la copie dans `assets/images/` que `import_asset` faisait
  déjà, puisque `Img::extensions` nomme `svg` et que la destination est un
  seul endroit du code. Trois choses tranchées. `TEXT_COMMON` ne s'applique
  **pas** : gpui ne peint un SVG que si `style.text.color` dit de quoi le
  teindre, donc la couleur est une propriété de l'entrée — mais une taille de
  police, une graisse, un soulignement sur un dessin sont sept rangées vides.
  C'est la forme qu'`icon` avait déjà prise, et pour la même raison, une icône
  *étant* un `svg` teinté. Ensuite `instantiate` pose la couleur et le chemin,
  parce qu'un dessin sans couleur est un élément qui prend sa place et ne
  montre rien — ce qui se lit comme un fichier cassé. Enfin le canvas le
  dessine avec `img` : `Svg::path` passe par l'`AssetSource` de l'application,
  et sur ce canvas c'est celui de l'atelier ; le chargeur d'images de gpui lit
  les `.svg` sur le disque, donc le dessin est le bon, seule la teinte manque.
  Vérifié aussi : `sync_image_size` ne compte rien sur un `.svg`, et il le
  demande maintenant avant de lire le fichier plutôt que de compter sur
  l'erreur du décodeur.
- ~~**Suivre `gpui-component`.**~~ — fait : `tests/components.rs` localise
  les sources de la crate dans `$CARGO_HOME/registry/src`, à la version que
  `Cargo.lock` épingle — la même lecture du verrou que `build.rs`, recopiée
  parce qu'un script de compilation ne s'importe pas —, énumère les
  soixante-deux modules de premier niveau et exige que chacun tombe dans
  l'une de trois listes : offert, écarté avec sa raison, à regarder. Ce qui
  a été tranché : les offerts ne sont pas listés mais **déduits** des lignes
  `use gpui_component::<module>::…` que `CATALOGUE` écrit, si bien qu'un
  composant ajouté à la table sort tout seul de « à regarder » ; deux noms
  seulement sont écrits à la main, `icon` (réexporté depuis la racine de la
  crate, donc son module n'apparaît dans aucun import) et `tooltip` (une
  propriété, pas une entrée). Un registre non peuplé fait échouer le test
  avec la commande à lancer, jamais passer en silence.
- **Les emplacements multiples**, encore non. La mesure du chantier 2 tient,
  et tout ce qui précède passe avant. Ce qui les rouvrirait, cette fois
  pour une raison mesurée : une forme de projet du chantier 7 qui en ait
  besoin — un *Assistant* voudra peut-être `Collapsible`, un *Tableau de
  bord* `DescriptionList`. C'est là que la question se reposera, avec un
  usage devant elle.

## Les templates

Le chantier 7. Trois formes répondent à « qu'est-ce qui tient quoi » ;
aucune ne répond à « qu'est-ce que ça fait ».

- ~~**Des formes qui font quelque chose.**~~ — fait : `Template` en porte
  neuf. *Liste et détail* (une liste, un panneau qui suit la sélection),
  *Formulaire* (deux champs liés, un bouton qui les lit), *Tableau de bord*
  (un en-tête, six cartes, des chiffres), *Assistant* (trois étapes, une
  barre, Précédent et Suivant), *Utilitaire* (une fenêtre compacte),
  *Éditeur* (une bande d'onglets, une zone multi-lignes, une barre d'état).
  La règle du chantier 1 tient sans exception : la coquille reste
  `shell.rs`, du Rust écrit une fois, et chaque page est une vue avec ses
  marqueurs — donc `page_rs` à côté de `shell_rs`, et une seule table
  `SHAPE_PAGES` que `create_project` écrit dans le projet et que `build.rs`
  écrit dans `OUT_DIR`, si bien qu'une page posée est une page déjà
  compilée. Ce qui a été tranché : la barre de titre n'est plus un drapeau
  mais `Template::has_shell()`, parce que `shell.rs` est le seul fichier qui
  dessine un `TitleBar` et qu'ouvrir la fenêtre sans lui ne laisse qu'une
  bande nue là où sont les feux ; l'*Utilitaire*, n'ayant pas de barre
  latérale, n'a donc pas de coquille, garde la barre du système comme la
  forme *Vide* et ouvre en 480 × 360 ; et une forme nommée d'après une page
  ouvre dessus, `home` restant derrière comme place pour l'écran suivant.
- ~~**Une forme déclare son état.**~~ — fait, et par l'autre chemin que
  celui prévu : `ensure_state_field` écrit un champ **à l'enregistrement**,
  et une page écrite droit sur le disque ne passe par aucun enregistrement.
  Elle porte donc son champ, son type et son initialisation dans le fichier
  même qui porte l'arbre qui le lie — `page_rs` les écrit ensemble, et
  nomme `window` et `cx` d'après ce qui s'en sert, faute de quoi maxx serait
  l'auteur du premier avertissement du projet. C'est ce qui rend les zones
  de saisie du *Formulaire* et de l'*Utilitaire* réelles, et la zone
  multi-lignes de l'*Éditeur* aussi.
- ~~**Des modèles de sous-arbre de plus.**~~ — fait : `SUBTREES` en tient
  huit. Un champ de formulaire, un en-tête de page, une barre d'état, une
  liste vide — faite des entrées du catalogue et non du composant
  `EmptyState`, parce qu'un modèle dépose un arbre que maxx peut défaire, et
  une brique est un nœud dont l'intérieur est un fichier — et une rangée
  Annuler / OK. La règle « sans état » est levée, et elle coûtait **une
  ligne** : `insert_subtree` appelle `registry::rebind_state_fields` comme le
  collage, et la déclaration du champ n'était à écrire nulle part —
  `view::render_source` parcourt déjà l'arbre à chaque enregistrement et
  appelle `ensure_state_field` pour toute liaison qu'il y trouve, ce que le
  point ci-dessus n'avait pas vu. Le champ de formulaire porte donc une vraie
  zone de saisie, et déposé deux fois il donne deux champs distincts.

  La rangée Annuler / OK porte ses gestionnaires, `ensure_handler` écrivant
  les deux méthodes au même enregistrement. Ce qu'il a fallu trancher : un
  modèle qui lie un état ou nomme `Self::…` ne compile pas dans une fonction
  libre, donc `build.rs` l'enveloppe dans le seul endroit où il atterrit —
  une vue, avec ses champs et un talon par gestionnaire. D'où une colonne de
  plus dans la table, les champs écrits en chemins complets pour que la
  preuve de compilation ne doive aucun `use`. Ce qui reste : deux dépôts de
  la rangée nomment le même `Self::on_ok`, exactement comme deux collages
  d'un même bouton — les identifiants sont renumérotés, les gestionnaires
  non.
- **Une galerie à la création.** `Fichier ▸ Nouveau projet` est un
  sous-menu de noms. Une boîte avec une vignette par forme, dessinée par le
  canvas depuis les pages de la forme — `canvas::render_node` dessine un
  arbre, et `parser` lit un arbre dans le texte d'une page comme il lit une
  vue ouverte ; aucun projet n'a besoin d'exister pour ça. Ce qu'il faudra
  trancher : la vignette est-elle rendue à
  chaque ouverture ou une image versionnée ? Rendue : une image versionnée
  ment dès que la forme change.
- **Un template est un projet.** Aucun format : un template est un dossier
  qui contient un `maxx.toml`, copié sous un autre nom. Deux sources — un
  dossier de l'utilisateur à côté de `settings.json`, et un chemin donné
  dans la boîte — et une seule opération, qui reprend `create_project`
  jusque dans son refus d'écrire sur un `Cargo.toml` existant, et renomme
  le crate comme `cargo_toml` et `crate_name` le font. Le geste inverse,
  « enregistrer le projet courant comme forme », est une copie sans
  `target/` ni `.cargo/`. Ce qu'il faudra trancher : un template copié
  garde-t-il ses empreintes dans `maxx.toml` ? Oui — ce sont celles des
  modules, qui ne changent pas en copiant, et c'est ce qui permet de les
  mettre à jour ensuite.
- ~~**`maxx new`.**~~ — fait : `src/cli.rs` lit `std::env::args` et rien
  d'autre, sans dépendance. `parse` est pure — elle ne touche ni le disque
  ni le processus, donc elle se lit — et `dispatch` agit : `new`, `--help`
  et `--version` répondent sur le terminal et **sortent avant**
  `Application::new()`, si bien qu'aucune de ces trois commandes ne
  réclame un écran. Ce qui a été tranché : le nom du crate vient du dernier
  segment du chemin, comme la boîte de dialogue le fait — c'est maintenant
  `scaffold::project_name`, appelée des deux côtés, et non plus quatre
  lignes recopiées ; `--shape` accepte `--shape x` comme `--shape=x` ;
  `Template::ALL` tient la liste des formes, que le message de refus et
  l'usage lisent tous deux, donc une variante de plus s'y ajoute en un
  endroit ; et un premier argument inconnu reste un chemin à ouvrir, parce
  que refuser une fenêtre pour un drapeau que personne ne connaît serait
  une régression. `tests/cli.rs` lance le binaire pour de vrai.
- **Une forme se met à jour.** `shell.rs` et les pages qu'une forme écrit
  entière — `settings_screen.rs` — ne sont pas dans `[modules]` de
  `maxx.toml`, donc *Mettre à jour les modules* ne les voit pas, et une
  forme corrigée dans maxx n'atteint jamais un projet déjà écrit. La
  pièce existe (`scaffold/modules.rs`, les deux empreintes, le refus de
  remplacer un fichier modifié) ; il manque d'y inscrire ce que
  `add_shell` écrit. Ce qu'il faudra trancher : `shell.rs` porte la liste
  des pages, que le développeur modifie dès qu'il en ajoute une — il sera
  donc presque toujours « modifié », et la mise à jour presque toujours
  refusée. C'est le point *Montrer ce qui change* de plus bas qui le
  débloquerait, et c'est pour ça qu'il est cité dans ce cycle.

## Le thème

Fait, dans les deux sens.

**Le projet généré** reçoit `src/theme.rs` par `Fichier ▸ Ajouter au projet ▸ La
palette` : des rôles plutôt que des couleurs, deux valeurs chacun, et le mode lu
dans le thème de `gpui-component` plutôt que tenu à côté — deux modes qui
peuvent diverger, c'est la fenêtre dans l'un et ses boutons dans l'autre. La
démo le porte et le montre, donc la CI prouve qu'il compile.

**maxx lui-même** a les deux palettes, et `Affichage ▸ Clair ou sombre` bascule
à la fois ce qu'il dessine et ce que le canvas montre des composants. Les
constantes sont devenues des accesseurs sans argument : la mode est une valeur
unique pour tout le processus, comme la langue, et une couleur se demande depuis
des fermetures et des fonctions libres qui n'ont pas de `cx` à donner.

Ce que ce lot a demandé au passage : **le gestionnaire écrit doit suivre la
forme du composant**. `Button::on_click` tend un `&ClickEvent`, `Switch::on_click`
tend l'état vers lequel il vient de basculer. Un seul gabarit laissait un projet
qui ne compile pas, sur une ligne écrite par maxx — d'où `HandlerSpec`, et une
propriété Action sur l'interrupteur, la case à cocher et le bouton radio.

Ce qui reste de ce côté :

- ~~**Éditer la palette dans maxx.**~~ — fait, et sans marqueurs. La solution
  retenue n'est pas une zone gérée mais une **rustine** : `src/themefile.rs`
  lit les rôles du fichier et n'en réécrit que les huit caractères du littéral
  changé, comme `parser::splice` dans une vue et comme la rustine de réglages
  dans un JSON. Le module reste celui du développeur — commentaires, rôles
  ajoutés, ordre — et ce que le lecteur ne reconnaît pas n'est ni montré ni
  touché. La page vit dans l'écran des réglages, à côté de celle d'exécution.

  Ce qu'il a fallu trancher : écrire dedans fait diverger le fichier de toutes
  les versions du gabarit, donc maxx cesse d'y proposer ses mises à jour. C'est
  la règle qui vaut déjà pour un module édité à la main dans Zed, et choisir une
  couleur est exactement autant une prise de possession.
- **Une palette par projet.** Aujourd'hui les valeurs du gabarit sont les mêmes
  pour tout le monde ; c'est un point de départ, pas une identité.

## Les modules copiés

- ~~**Le `rustfmt` du projet défait l'empreinte.**~~ — réparé. maxx
  reconnaissait un module qu'il avait écrit aux octets qu'il avait laissés, et
  `cargo fmt` — le geste le plus banal qui soit — change ces octets sans
  toucher une ligne de code. Mesuré sur les gabarits : la mise en page par
  défaut déplace dix lignes de `system.rs`, cinquante-six de `theme.rs`, et
  transforme `else { return }` en `else { return; }` — donc aucune comparaison
  « aux espaces près » ne rattrape ça. maxx tenait alors le fichier pour édité
  par le développeur et cessait, **en silence**, de lui proposer les
  corrections.

  `maxx.toml` porte désormais deux empreintes : celle des octets posés, et
  celle du même texte passé au `rustfmt` **par défaut**, configuration du
  projet ignorée — un étalon fixe, que les deux côtés d'une comparaison faite à
  des années d'écart peuvent viser. Un fichier reconnu par l'une ou par l'autre
  n'a pas été touché. Un commentaire ajouté par le développeur reste une
  modification, `rustfmt` conservant les commentaires. Sans `rustfmt` sur la
  machine, ou pour un `maxx.toml` écrit avant cette clé, on retombe sur les
  octets, c'est-à-dire le comportement précédent.

## Ce que la zone gérée perdait

~~**Un commentaire écrit entre les marqueurs disparaît au `⌘S` suivant.**~~ —
réparé. `syn` jette toujours les commentaires ; c'est le modèle qui les porte
désormais, et le lecteur les récupère du texte avant de le lui donner. Un
commentaire appartient à ce qui le suit : au-dessus d'un appel, au-dessus d'un
enfant, en tête de chaîne ou après le dernier appel.

Ce qui reste de ce côté, et qui est mineur : un commentaire **en fin de ligne**
(`.gap_2() // pourquoi`) remonte au-dessus de l'appel plutôt que de rester
derrière lui, et un commentaire écrit **entre les parenthèses** d'un appel
(`.gap(/* huit */ 8)`) remonte de même au-dessus de lui. Le rendu de maxx est
d'un appel par ligne, donc la place existe ; c'est le modèle qui ne distingue
pas encore ces positions. Ce qui est garanti est que les mots ne disparaissent
pas, et qu'un deuxième enregistrement ne bouge plus rien.

Reste aussi vrai, et c'est un autre sujet : la **mise en page** de la zone est
celle de maxx, pas celle du développeur — un enfant écrit sur une ligne peut
ressortir sur trois. Ce qui est garanti est le contenu, pas la colonne.

## Composants

- ~~Élargir le catalogue~~ — fait, cinq entrées de plus : bouton radio, lien,
  alerte, pastille et barre de progression. Une seule a demandé de la
  machinerie, la barre : sa valeur est un `f32` nu, là où `Kind::Number`
  écrivait toujours `px(…)`. D'où `Kind::Ratio`, et `pixel_literal` qui n'est
  plus qu'un enrobage de `float_literal`.

  Ce qui était hors de portée l'est six entrées de moins : **l'icône** a
  demandé `Target::VariantArg`, une variante d'énumération en argument du
  constructeur ; **le curseur** et **le sélecteur de couleur** ont réutilisé le
  `StateSpec` du champ texte ; **les variantes de la pastille** passent par
  `with_variant`, une méthode, et non par les constructeurs `Tag::primary()`
  qui changeraient la base du nœud ; **le badge** et **le rouet**
  n'implémentent pas `Styled`, ce qui est précisément ce que `Common::None`
  disait déjà ; et **l'ossature** est un `new()` nu.

  Ce qui restait de ce côté — vingt-deux icônes offertes sur les
  quatre-vingt-six que porte `IconName` — est réglé plus bas : la table est
  engendrée depuis l'énumération, et les deux côtés sortent de la même
  lecture.

- ~~Liste déroulante~~ — faite. Elle a demandé de généraliser la machinerie du
  champ texte plutôt que de la copier : une entrée du catalogue porte
  maintenant un `StateSpec` optionnel, et `view::ensure_state_field` sert les
  deux. Ce qui reste de ce côté : le contenu de la liste est écrit dans
  l'initialiseur, donc dans le code que vous éditez à la main. maxx pose deux
  entrées pour que quelque chose s'affiche et ne prétend pas gérer la source
  des données — un jour, peut-être, une propriété « Éléments ».

- ~~L'image~~ — faite, et c'est elle qui a apporté `Kind::Path`. La question
  n'était pas le composant mais la **source** : `img("logo.png")` compile et
  n'affiche rien, parce qu'un `&str` est cherché dans l'`AssetSource` qu'un
  projet généré ne déclare pas. Un chemin, lui, est lu depuis le répertoire
  courant — celui où `cargo run` démarre, donc la racine du projet, le seul
  endroit où le canvas et le binaire peuvent tomber d'accord. D'où
  `PathBuf::from("…")`, un import qui vient de la base et non d'un appel, et
  un refus des chemins absolus.

  Une image choisie ailleurs sur le disque est **copiée dans
  `assets/images/`** plutôt que refusée : le projet doit porter ses images,
  sinon elles ne s'affichent que sur la machine où on les a choisies. Un
  fichier déjà dans le projet reste où il est, un homonyme différent est
  numéroté, et le même fichier réimporté est reconnu à ses octets.

  Deux finitions qui viennent de l'usage : une image déposée **s'adapte**
  d'emblée — une photographie fait deux mille pixels de large et une vue cinq
  cents, donc la première image poussait tout le reste hors de la planche — et
  l'inspecteur montre une **vignette** de vingt-huit pixels à côté du champ
  Source, parce qu'un chemin faux et un fichier absent se ressemblent
  exactement dans une boîte de texte. Le premier a demandé au catalogue de
  savoir poser des appels par défaut, `default_calls`, et pas seulement des
  arguments : c'est une table, pas un cas particulier dans le code.

  Trois finitions de plus, venues de l'usage : la **Taille** est un choix à
  trois états — naturelle, jamais plus large que le conteneur, ou exactement le
  conteneur — et non un interrupteur, parce que la question a trois réponses ;
  l'**Ajustement** expose `ObjectFit` de gpui, donc `Contain`, `Cover`, `Fill`,
  `ScaleDown` et `None`, ce qui a demandé une cible qui écrit une variante
  d'énumération (`Target::Variant`) — la sorte d'argument que le backlog
  réclamait pour l'icône, et qui existe donc maintenant ; et la **taille
  naturelle** s'affiche à côté de la vignette, sans quoi aucune largeur n'est
  pensable.

  Ce qui reste, et que l'usage a demandé : **écrire les dimensions en
  commentaire dans le code**, ou des annotations à nous. Ce n'est pas
  possible en l'état — `syn` jette les commentaires et `codegen` réécrit la
  zone gérée depuis le modèle, donc tout commentaire posé entre les marqueurs
  disparaîtrait au `⌘S` suivant. Il faudrait que le modèle porte les
  annotations et que le lecteur les récupère avant `syn`, ce qui est un
  chantier à part entière — et le seul qui permettrait aussi de garder les
  commentaires que le développeur écrit *dans* la zone gérée.

  **Le sélecteur ne filtre pas, et ne le peut pas** : `PathPromptOptions` de
  gpui n'a que `files`, `directories`, `multiple` et `prompt`, et son panneau
  macOS n'appelle jamais `setAllowedFileTypes`. Filtrer demanderait une autre
  boîte — `rfd`, qui réclame GTK3 sur Linux, ou un sélecteur écrit dans maxx.
  Ni l'un ni l'autre ne vaut, pour l'instant, un refus qui nomme les formats.

  Ce qui reste, et qui se voit : **le chemin est lu depuis le répertoire
  courant du processus**. `cargo run` démarre à la racine, donc l'image
  s'affiche quand maxx lance le projet ou quand on le lance soi-même depuis
  sa racine — et pas si on double-clique le binaire, dont le répertoire
  courant est ailleurs. C'est ce que **`assets.rs`** réglera : le module qui
  déclarerait un `AssetSource`, donc l'image embarquée dans le binaire. Et les **propriétés de style sur l'aperçu** : `apply` ne prend
  qu'un `Div`, donc la largeur et la hauteur posées sur une image sont
  écrites dans le fichier mais pas montrées sur le canvas.

- ~~Le défilement~~ — fait, des deux côtés. Le gabarit d'une vue porte
  maintenant `id`, `size_full` et `overflow_y_scroll` : une vue plus haute que
  la fenêtre était coupée sans recours, et une image à sa taille naturelle y
  suffit. Et une propriété **Défilement** sur la colonne et la ligne, avec
  l'axe qui va avec le conteneur.

  Ce qu'elle a demandé : une cible qui écrit **deux appels**, `Target::Scrollable`.
  gpui ne garde le décalage de défilement que pour un élément qui a un `id`,
  donc le drapeau seul rogne le contenu sans jamais le faire bouger. Et cet
  identifiant doit être unique entre frères, ce qu'aucun nœud ne peut savoir
  seul : c'est l'espace de travail qui le distribue, comme il distribue le nom
  d'un champ d'état.

  Ce qui reste : **la barre visible**. `Scrollbar` demande un `ScrollHandle`
  dans un champ de la vue — `StateSpec` sait déjà faire — mais la barre est un
  *frère* posé en absolu dans un parent `relative()`, et le modèle de maxx est
  une chaîne unique. C'est un modèle de sous-arbre, pas une propriété.

- ~~Voir une image~~ — faite : un fichier du panneau dont l'extension est dans
  la liste de gpui s'ouvre dans un onglet et s'affiche, au lieu d'être refusé
  par le contrôle UTF-8 du lecteur. La barre d'état nomme son poids plutôt que
  ses lignes, et le plafond est plus haut que celui du texte : décoder une
  photo n'est pas analyser un tampon avec tree-sitter.

## Menus

- **Les sous-menus** sont faits : lus, affichés d'un cran en retrait, créés,
  renommés, remplis, réordonnés et supprimés. Un seul niveau, et c'est un
  choix — un sous-menu de sous-menu est un endroit que personne ne retrouve, et
  s'arrêter là garde la sélection en triplet plutôt qu'en chemin. Un
  `submenu(build())`, dont le contenu n'est pas un littéral, reste opaque :
  lisible, pas modifiable.
- ~~Le raccourci d'une entrée~~ — fait : champ « Raccourci » dans l'inspecteur,
  lu et écrit dans `key_bindings`, avec refus d'une frappe que gpui ne saurait
  pas lire. Reste au clavier : la touche `-` elle-même, dont l'écriture est
  ambiguë avec le séparateur, et la saisie du raccourci en l'appuyant plutôt
  qu'en le tapant. Renommer une action laisse son ancienne liaison derrière,
  comme elle laisse son ancien gestionnaire : maxx ajoute et met à jour ce
  qu'il possède, il n'efface pas ce qu'il ne peut pas prouver inutile.
- **Réordonner** est fait : Monter et Descendre, ⌘⌃↑ et ⌘⌃↓, dans la barre de
  menus comme dans le panneau. Une entrée reste dans son menu et un menu parmi
  les menus — franchir cette limite est ce qu'un glisser-déposer ferait, et
  c'est un geste différent avec ses propres affordances. Le glisser-déposer,
  lui, reste à faire.

## Confort

- ~~**Renommer une vue.**~~ — fait : le champ de l'inspecteur, `scaffold::rename_view`,
  et huit essais dans `tests/scaffold.rs`. Ce qui suit dit ce qu'il a fallu
  trancher. maxx sait en créer une, la supprimer et en adopter une,
  mais pas la renommer — et l'argument qui justifie le nom généré s'appuie
  pourtant là-dessus (`views.rs`, `new_view` : « `view_2` is renamable in Zed in
  two seconds »). C'est vrai du fichier, pas de la vue : il faut aussi la ligne
  de `src/ui/mod.rs`, le nom du type, et, quand c'est la vue d'entrée, l'import
  et l'appel de `main.rs`. Quatre endroits, dont un que rien ne signale si on
  l'oublie.

  Les pièces existent toutes : `scaffold::create_view` sait déclarer dans
  `mod.rs`, `workspace::unregister_view` sait dé-déclarer, `view_module` sait
  reconnaître une vue, `explorer::entry_view` sait laquelle `main.rs` ouvre. Il
  manque la commande qui les enchaîne — et le renommage du type, qui est le seul
  morceau nouveau : textuel comme le reste, sur le `pub struct` et ses `impl`,
  jamais sur le corps des méthodes.

  Ce qu'il faudra décider en l'écrivant : ce que maxx fait des occurrences
  qu'il ne possède pas. Un `Accueil::new` appelé depuis une autre vue est du
  code de l'utilisateur, et le remplacer serait franchir la limite que tout le
  reste du projet respecte. Le dire et s'arrêter est probablement la bonne
  réponse, comme pour un module modifié.

- ~~Palette de commandes `⌘K`~~ — faite, et sans liste à elle : `palette::flatten`
  aplatit ce que `menus::app_menus` renvoie déjà. Une commande ajoutée au menu y
  apparaît sans qu'on y touche, dans la langue de l'interface, avec son
  raccourci relu depuis `actions::key_bindings`. Écrire une seconde liste à côté
  aurait fait deux endroits à tenir à jour, et le second aurait pris du retard.

  `escape`, `↑` et `↓` sont liées au contexte clavier « Palette » et non
  globalement — les prendre partout les aurait reprises au reste de l'interface.
  L'aplatissement est séparé de `commands(cx)` pour être vérifiable sans `App` :
  la feature `test-support` de gpui n'a pas à entrer dans le build pour ça.

- ~~Recherche dans le catalogue~~ — faite, une boîte au-dessus de la palette.
  Elle cherche dans le libellé *et* dans l'identifiant : `input` trouve le champ
  texte quelle que soit la langue de l'interface, ce que tape qui a lu le code
  généré. Et elle ignore les accents — personne ne tape « Étiquette » avec son
  accent dans une boîte de recherche.

  Le champ est construit avant le retour anticipé de `sync_prop_inputs`, parce
  qu'il ne dépend pas de la sélection et qu'une fenêtre sans vue ouverte montre
  quand même la palette.

- ~~Copier, coller, dupliquer un nœud~~ — faits. `⌘D` duplique, `⌘⌥C` et `⌘⌥V`
  copient et collent. Pas `⌘C` / `⌘V` : ils appartiennent aux champs texte de
  l'inspecteur, et les leur reprendre casserait la saisie.

  Le presse-papier porte du Rust, pas un format à maxx : `codegen::render` à
  l'aller, `parser::parse_expr` au retour, donc un sous-arbre copié se colle
  dans Zed et une expression écrite là se colle ici. Un texte que maxx ne sait
  pas lire est refusé plutôt qu'accepté en nœud opaque — coller est un geste
  volontaire, et rendre l'inmodifiable en silence serait une surprise.

  Le point qui n'allait pas de soi : une copie porte le champ d'état de
  l'original. Deux `Input` sur `&self.field` compilent et se recopient l'un
  l'autre à l'exécution. D'où `registry::rebind_state_fields`, appelé avant que
  la copie rejoigne l'arbre.

- ~~Panneaux redimensionnables~~ — faits, par `gpui_component::resizable` et
  non par `dock`, qui apportait un modèle de fenêtres flottantes dont maxx n'a
  pas l'usage. L'explorateur et l'inspecteur ont leur poignée, avec des bornes
  (160–520 px, 220–560 px) pour qu'aucun ne puisse manger l'autre entièrement.
  Les largeurs vivent dans `state.json`, tenues en mémoire pendant le
  glissement et écrites à l'extinction, comme la géométrie de la fenêtre.
  Reste : le panneau de sortie est encore haut de 200 px, et l'éditeur de menus
  garde son inspecteur figé à 280.
- ~~**`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0**~~ —
  fait, et le cas n'était pas tordu : il s'ancrait sur le dernier `use` du
  fichier, `syn` ou pas, donc un `use` placé après l'`impl` emportait vers le
  bas tous les imports que maxx ajoute. Il s'ancre maintenant sur le dernier
  `use` de l'**en-tête** — la suite d'items lue depuis le haut, qu'un `mod x;`
  ou un `extern crate` ne clôt pas, et que le premier item à corps arrête.

  Deux voisins sont tombés avec : le repli textuel, qui portait le même défaut
  pour un fichier que `syn` refuse — c'est-à-dire pendant qu'on l'écrit —, et
  l'ancre à défaut d'import, qui était l'octet 0. Une vue sans aucun `use`
  recevait donc le premier au-dessus de son `//!`, ce qui ne compile pas : c'est
  maintenant la fin des attributs internes du fichier.
- ~~Voir un fichier que maxx ne sait pas dessiner~~ — fait, en lecture seule.
  N'importe quel fichier texte de l'explorateur s'ouvre dans `workspace/code.rs`,
  colorisé par tree-sitter, avec ses numéros de ligne ; un `.rs` sans région
  gérée — `main.rs`, `ui/mod.rs` — y va aussi, au lieu de l'erreur d'analyse
  qu'il recevait.

  `⌘E` retourne la vue en cours : le code montré est celui que `⌘S` écrirait,
  rendu depuis l'arbre — `View::render_source`, sorti de `View::save`, dont il
  ne restait que le `fs::write` à retirer. Le disque aurait été plus simple et
  faux : une vue modifiée aurait montré un code périmé, exactement quand on
  ouvre le panneau pour vérifier. Et une seule tabulation : une vue vue comme
  code reste le même document, d'où `CodeFile::of_view`.

  La colorisation ne coûte aucune dépendance à nous : `gpui-component` porte
  déjà les grammaires, derrière sa feature `tree-sitter-languages`. Le seul
  effet de bord de l'activer est un `cc` retenu en 1.2.67 dans `Cargo.lock` :
  `tree-sitter-sequel` le plafonne en `~1.2`, alors que `gpui` tirait 1.4.4 par
  `embed-resource`, et cargo refusait de résoudre tant que les deux
  cohabitaient. `cargo update -p cc --precise 1.2.67` débloque — et comme `cc`
  pilote la compilation native de tout l'arbre, la vérification qui compte est
  que `gpui` a bien été recompilé derrière : la sortie de ce build porte
  « Compiling cc v1.2.67 » puis « Compiling gpui v0.2.2 », et elle aboutit.

  **Éditer et enregistrer depuis maxx est écarté**, et pas par manque de temps :
  un `.rs` géré a déjà une source de vérité, le canvas, et deux écrivains sur un
  même fichier demandent une politique de conflit dont `Workspace::conflicts`
  n'esquisse aujourd'hui que la moitié — il sait dire qu'un fichier a changé
  dehors, pas fusionner. Tant que cette politique n'est pas décidée, le lecteur
  lit et `⌘⌥Z` reste le chemin vers l'écriture.

## Avant de rendre le dépôt public

- ~~Chemin personnel codé en dur dans un test~~ — fait : la référence est
  `demo/`, versionnée, et `tests/demo.rs` la vérifie par chemin relatif.
- ~~**README en retard sur l'interface**~~ — rattrapé : les trois formes de
  projet, `⌘P`, la navigation entre onglets, ＋ / 🗑 et le menu contextuel de
  l'arborescence, ce que `maxx.toml` porte désormais (l'entrée, `[run]`, les
  deux empreintes), et ce que la zone gérée garde — les expressions opaques et
  les commentaires.
- ~~**Un GIF dans le README.**~~ — fait : `docs/maxx-demo.gif`, la démo de
  `demo/` filmée, en tête du fichier avant la capture fixe. La source pesait
  25 Mo ; le GIF versionné en fait 3,5 (800 px, 10 images/s, 64 couleurs) et
  reste hors du paquet crates.io par `exclude`.
- ~~**Compiler la démo en CI.**~~ — fait, et ce point était périmé : le travail
  « the demo compiles » de `.github/workflows/ci.yml` lance `cargo check` dans
  `demo/`, qui n'est pas membre de l'espace de travail et que rien d'autre ne
  construit. La ligne barrée deux points plus bas le disait déjà.
- ~~Les deux avertissements clippy~~ — faits, et la CI est stricte
  (`clippy -D warnings`).
- ~~CI GitHub Actions~~ — faite, en matrice sur les trois systèmes, plus un
  travail qui compile la démo.
- ~~Métadonnées `Cargo.toml`~~ — faites, `rust-version = "1.88"` compris.
- ~~**Publier.**~~ — fait : `maxx` est sur crates.io, en 0.1.0 puis 0.2.0. Le
  verrou `publish = false` était levé, la description passée en anglais et
  `exclude` écartait les médias du README ; restait le geste lui-même, avec un
  jeton que seul un humain pose. Ce qui vaut toujours d'être su : `cargo
  install` demande à l'utilisateur Linux les paquets de développement de
  Vulkan, de Wayland et de fontconfig.
  C'est assumé — maxx s'adresse à des gens qui compilent, et la version ouverte
  par un tag n'attache plus aucun binaire depuis `v0.1.0`. Et une version
  publiée ne se supprime pas, elle se *yank*.
- ~~`cargo fmt`~~ — fait, avec `use_small_heuristics = "Max"` pour préserver
  les tables du catalogue, et `fmt --check` est dans les deux workflows.

- ~~**Trois documents annoncent des binaires attachés aux versions**~~ — fait :
  les deux qui restaient disent maintenant ce que `release.yml` fait. Le README
  (section sur l'icône) et `ARCHITECTURE.md` décrivent une release qui vérifie,
  contrôle le paquet crates.io et ouvre la version avec la section du CHANGELOG
  pour corps, sans y attacher d'exécutable — un binaire est une distribution au
  sens des licences, avec les mentions qui doivent voyager avec lui, quand
  `cargo install` suffit. `scripts/bundle-macos.sh` est gardé : il sert à la
  main, à qui veut un `.app` local, et le README le dit désormais à l'endroit
  où le script est présenté.

## Portabilité

Le code est porté : plus rien dans `src/` ne suppose macOS. Ce qui diffère est
dans `run.rs` derrière des `cfg` — cache, corbeille (spécification freedesktop
sur Linux, `.trashinfo` compris), arbre de processus (`taskkill /T` sur
Windows), et le repli par paquet d'application, qui n'existe que sur macOS. La
CI compile et teste sur les trois systèmes à chaque poussée.

Ce qui reste, et qui ne se règle pas au clavier :

- **Personne n'a lancé maxx sur Linux ni sur Windows.** La CI prouve que ça
  compile et que la suite passe ; aucun test n'ouvre de fenêtre. Il faut un
  essai humain par système, et c'est la seule façon de savoir.
- **La corbeille Windows est celle de maxx**, pas celle du système : la vraie
  demande l'API du shell, donc une dépendance et un bloc `unsafe`. À revoir si
  quelqu'un utilise maxx là-bas pour de bon.
- **La détection d'éditeurs hors macOS** se limite au `PATH`. Un éditeur
  installé mais sans commande sur le `PATH` reste invisible : Linux devrait lire
  les `.desktop` de `/usr/share/applications`, Windows la base de registre.

## Le code ajouté aux projets

Le module système et les réglages sont des copies : un défaut corrigé dans maxx
doit être reporté à la main dans le gabarit. Ce qui manquait, c'était le moyen
de faire ensuite arriver la correction aux projets déjà écrits — c'est fait,
par `maxx.toml` et les modules versionnés, avec refus de remplacer un fichier
que le développeur a modifié.

Ce qui reste :

- **Montrer ce qui change** avant de remplacer, et proposer quelque chose à un
  fichier modifié — un diff, ou l'écriture du nouveau à côté. Aujourd'hui maxx
  dit seulement « vous l'avez modifié » et s'arrête.
- ~~**`maxx.toml` ne porte que les modules.**~~ — fait : `[project] entry` et
  `[run]` (profil, features, `default-features`, arguments). Ce qui reste de ce
  point : rien ne les édite dans maxx, ils se posent à la main dans le fichier,
  sauf la vue d'entrée qui a sa commande (*Fichier ▸ Ouvrir la fenêtre sur
  cette vue*, et le menu contextuel de l'explorateur, qui marque d'un point la
  vue d'entrée). Les formes de projet, elles, écrivent `entry` toutes seules.
- **Des modules plus fins**, si l'usage le demande : aujourd'hui le module
  système arrive entier, sous `allow(dead_code)`, alors qu'un projet n'en veut
  peut-être que la corbeille.

## La langue

Faite. L'interface passe par `t!`, `locales/app.yml` porte l'anglais et le
français, et le réglage vit dans `settings.json` à côté des autres. `rust-i18n`
était déjà dans l'arbre — `gpui-component` s'en sert pour ses propres chaînes,
et `set_locale` écrit le même global —, donc le composant suit la langue de maxx
sans rien de plus.

Deux endroits où le texte servait de valeur, et qui ne pouvaient pas survivre à
la traduction : `format_after_save` déduisait le succès d'un enregistrement de
la fin du message affiché, et un test lisait « refusé » dans une erreur de
rustfmt. Le premier prend maintenant un booléen, le second vérifie le nom du
fichier.

Ce qui reste de ce côté :

- **Une troisième langue** ne demande qu'une ligne de plus par clé. Le test
  `tests/locales.rs` n'exige `en` et `fr` que parce que ce sont les deux
  livrées : y ajouter un code suffit à le rendre exigeant pour lui aussi.
- **Le pluriel** est écrit à la main là où il apparaît — trois branches dans la
  description des projets récents. `rust-i18n` sait faire mieux ; trois branches
  ne le justifiaient pas encore.
- ~~**Les commentaires du code de maxx**~~ — faits en anglais. Ils étaient en français, alors que
  le README, le code généré et l'interface sont en anglais. C'est le dernier
  morceau, et il ne se règle qu'en le lisant en entier.

## Réglages

Trois choses distinctes, dans cet ordre de dépendance.

**Les réglages de maxx.** Le socle est écrit (`src/settings.rs`) : un TOML dans
le répertoire de configuration du système, chargé au démarrage, avec un défaut
par valeur pour qu'un fichier absent, partiel ou abîmé ne soit jamais pire que
pas de fichier du tout. Portent déjà : les projets récents — donc
`Fichier > Ouvrir un élément récent` et la liste de l'écran d'accueil —, l'état
des trois panneaux, et la géométrie de la fenêtre.

Restent figés dans le code : l'éditeur (`run.rs:91`), le terminal (`run.rs:71`),
le cache partagé (`run.rs:56`), la palette (`theme.rs`, des `const`) et la
largeur des panneaux, qui attend de toute façon les panneaux redimensionnables.

L'écran de préférences existe (`⌘,`, `src/preferences.rs`), bâti sur
`gpui_component::setting` : trois pages, Apparence, Projets, Fichier. Un champ
lit et écrit les réglages directement, sans copie, donc rien ne peut y diverger
de ce qui est sur le disque. Il grandira avec les réglages qu'on lui donnera.

La séparation préférences / état est faite : `settings.json` à l'utilisateur,
`state.json` à la machine, du JSON à commentaires lu comme Zed lit le sien, et
une écriture qui ne touche que la clé changée. SQLite n'apporterait rien à ces
quelques centaines d'octets et coûterait un format illisible à la main.

Ce qui reste de ce côté : **des réglages par projet**, `.maxx/settings.json`
superposé aux réglages globaux — c'est la couche que Zed appelle
`.zed/settings.json`, et elle n'a de sens qu'une fois qu'il y aura des réglages
qui méritent d'être différents d'un projet à l'autre.

**Ce que maxx retient d'un projet** : projets récents, dernière vue ouverte,
dossiers dépliés, largeur des panneaux pour ce projet. N'appartient pas au
projet — ça polluerait son dépôt et n'aurait aucun sens pour qui le clone.
Va dans l'état de maxx, indexé par chemin.

**Les réglages du projet lui-même**, dans les deux sens :

- ~~ce que maxx devine et devrait savoir~~ — fait : la vue d'entrée et la
  commande de lancement vivent dans `maxx.toml`, versionné à la racine. Reste à
  les éditer autrement qu'à la main pour `[run]`.
- ce dont l'application générée a besoin pour elle-même : un `src/settings.rs`
  avec sa zone marquée, que maxx édite comme il édite `src/menus.rs`. Du Rust
  ordinaire qui tourne sans maxx, dans le principe du projet.

Le second suppose le premier.

## Choisir son éditeur et son terminal

Fait : `src/tools.rs` tient le catalogue, la détection et la table des syntaxes
d'ouverture à une ligne ; l'écran de préférences en propose le choix, et la
barre de menus comme le bouton de l'inspecteur nomment l'éditeur retenu.

Ce qui reste de ce côté :

- **La détection hors macOS.** Aujourd'hui : la commande sur le `PATH` partout,
  plus le paquet `/Applications` sur macOS. Linux devrait lire les `.desktop`
  de `/usr/share/applications`, Windows la base de registre — sans quoi un
  éditeur installé mais absent du `PATH` reste invisible.
- **Un éditeur hors catalogue.** Le champ n'accepte que ce que la table
  connaît. Une entrée libre — commande plus forme de l'argument de ligne —
  couvrirait le reste du monde.
- **Terminal.app ne sait pas recevoir de commande** autrement que par
  AppleScript, donc un éditeur de terminal n'y démarre pas. maxx le dit au lieu
  de faire semblant.
