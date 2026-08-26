# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

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

- **Éditer la palette dans maxx.** `src/theme.rs` est un module copié, pas une
  zone gérée : maxx l'écrit et ne le relit pas. Lui donner des marqueurs, comme
  `src/menus.rs` en a, en ferait un écran de plus.
- **Une palette par projet.** Aujourd'hui les valeurs du gabarit sont les mêmes
  pour tout le monde ; c'est un point de départ, pas une identité.

## Composants

- ~~Élargir le catalogue~~ — fait, cinq entrées de plus : bouton radio, lien,
  alerte, pastille et barre de progression. Une seule a demandé de la
  machinerie, la barre : sa valeur est un `f32` nu, là où `Kind::Number`
  écrivait toujours `px(…)`. D'où `Kind::Ratio`, et `pixel_literal` qui n'est
  plus qu'un enrobage de `float_literal`.

  Ce qui reste hors de portée sans une nouvelle sorte d'argument : **l'icône**,
  dont le constructeur prend une variante d'énumération et non une chaîne ;
  **le curseur**, qui vit dans une entité comme le champ texte ; **les variantes
  de la pastille**, qui sont des constructeurs (`Tag::primary()`) et non des
  méthodes, donc changer de variante changerait la base du nœud. Et **le badge**
  n'implémente pas `Styled` : les propriétés communes ne compileraient pas
  dessus.

- ~~Liste déroulante~~ — faite. Elle a demandé de généraliser la machinerie du
  champ texte plutôt que de la copier : une entrée du catalogue porte
  maintenant un `StateSpec` optionnel, et `view::ensure_state_field` sert les
  deux. Ce qui reste de ce côté : le contenu de la liste est écrit dans
  l'initialiseur, donc dans le code que vous éditez à la main. maxx pose deux
  entrées pour que quelque chose s'affiche et ne prétend pas gérer la source
  des données — un jour, peut-être, une propriété « Éléments ».

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

- **Renommer une vue.** maxx sait en créer une, la supprimer et en adopter une,
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
- **`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0** du
  fichier : un `use` placé après l'`impl` attirerait les imports insérés vers le
  bas du fichier. Cas tordu, mais réel.

## Avant de rendre le dépôt public

- ~~Chemin personnel codé en dur dans un test~~ — fait : la référence est
  `demo/`, versionnée, et `tests/demo.rs` la vérifie par chemin relatif.
- **README en retard sur l'interface** : rien sur ＋ / 🗑 et le clic droit dans
  l'explorateur, sur `Affichage > Barre de menus du projet`, sur
  `Édition > Ajouter un menu`, ni sur la fenêtre À propos.
- **Une capture ou un GIF dans le README.** Pour un outil visuel, c'est
  l'élément à plus fort rendement de toute cette liste. La démo de `demo/` est
  faite pour ça : c'est elle qu'il faut photographier.
- **Compiler la démo en CI.** `cargo check` dans `demo/` prouve que ce que maxx
  écrit compile encore. Elle n'est pas membre de l'espace de travail, donc rien
  ne la construit aujourd'hui hors d'une commande explicite.
- ~~Les deux avertissements clippy~~ — faits, et la CI est stricte
  (`clippy -D warnings`).
- ~~CI GitHub Actions~~ — faite, en matrice sur les trois systèmes, plus un
  travail qui compile la démo.
- ~~Métadonnées `Cargo.toml`~~ — faites, `rust-version = "1.88"` compris.
- **Publier, si on y va.** Le nom `maxx` est libre sur crates.io (vérifié).
  Trois choses manquent alors : retirer `publish = false` du `Cargo.toml`, un
  jeton dans les secrets du dépôt, et la décision elle-même — `cargo install`
  demande à l'utilisateur Linux d'avoir les paquets de développement de Vulkan,
  de Wayland et de fontconfig, là où le binaire attaché à la version ne demande
  rien. crates.io sert surtout à réserver le nom et à rendre `cargo install`
  possible pour qui a déjà de quoi compiler.
- ~~`cargo fmt`~~ — fait, avec `use_small_heuristics = "Max"` pour préserver
  les tables du catalogue, et `fmt --check` est dans les deux workflows.

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
- **`maxx.toml` ne porte que les modules.** La vue d'entrée est toujours écrite
  en dur dans `main_rs()`, et la commande de lancement est toujours
  `cargo run`, sans profil ni features. C'est le même fichier qui les
  accueillera.
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
- **Les commentaires du code de maxx** sont toujours en français, alors que
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

- ce que maxx devine et devrait savoir : la vue d'entrée est écrite en dur
  (`main_rs()` importe `home`), la commande de lancement est toujours
  `cargo run`, sans profil ni features. Un `maxx.toml` à la racine, versionné.
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
