# Backlog

Ce qui est connu, décidé, et remis à plus tard. Rien ici n'est un oubli.

## Composants

- **Liste déroulante.** `Select` réclame un délégué et une entité d'état par
  nœud, comme `Input` : il ne rentre pas dans une entrée de catalogue statique.
  À faire avec la machinerie d'insertion de champs déjà écrite pour le champ
  texte (`view::ensure_input_field`).

## Menus

- **Sous-menus et raccourcis.** L'éditeur gère les menus, les entrées et les
  séparateurs ; un `MenuItem::submenu(..)` est conservé mais pas modifiable, et
  le raccourci d'une entrée se déclare encore à la main dans `key_bindings`.
- **Ni réordonnancement ni glisser-déposer** dans l'éditeur de menus : une
  entrée s'ajoute après la sélection et se supprime, c'est tout.

## Confort

- **Panneaux redimensionnables** via `gpui_component::dock`. Les onglets et le
  défilement sont faits ; la largeur des colonnes est encore figée.
- **`view::ensure_imports` s'ancre sur le dernier `use` en colonne 0** du
  fichier : un `use` placé après l'`impl` attirerait les imports insérés vers le
  bas du fichier. Cas tordu, mais réel.

## Avant de rendre le dépôt public

- **`tests/scaffold.rs:361` code en dur `/Users/sebastienportrait/rust/maxx-demo`**
  et le test s'arrête sans échouer quand le dossier manque : chez quelqu'un
  d'autre, un chemin personnel dans le dépôt et une couverture morte qui ne le
  dit pas. À passer par une variable d'environnement, ou à déplacer en fixture
  sous `tests/`.
- **README en retard sur l'interface** : rien sur ＋ / 🗑 et le clic droit dans
  l'explorateur, sur `Affichage > Barre de menus du projet`, sur
  `Édition > Ajouter un menu`, ni sur la fenêtre À propos.
- **Une capture ou un GIF dans le README.** Pour un outil visuel, c'est
  l'élément à plus fort rendement de toute cette liste.
- **`cargo fmt` une fois** — 82 blocs sur 11 fichiers, surtout des imports —
  puis les deux avertissements clippy : `expect` après `is_some` dans
  `workspace.rs`, `filter().next()` dans `tests/round_trip.rs`. Aucun n'est un
  bug ; c'est ce qui permet de brancher une CI stricte ensuite.
- **CI GitHub Actions** : `fmt --check`, `clippy -D warnings`, `test`. Avec
  cache — gpui et gpui-component représentent environ 750 crates, un run à
  froid est long.
- **Métadonnées `Cargo.toml`** : `readme`, `keywords`, `categories`, et
  `rust-version = "1.88"`. Le code utilise les chaînes `&& let`
  (`actions.rs:244`, `workspace.rs:1356`), donc 1.88 est le vrai plancher, pas
  le 1.85 que l'édition 2024 laisserait supposer.
- `publish = false` si crates.io n'est pas l'objectif.

## Portabilité

Rien dans le principe de maxx n'est propre à macOS : gpui 0.2.2 livre les trois
dorsales (`platform/mac`, `platform/linux` en Wayland et X11, `platform/windows`)
et les active par défaut. La feature `runtime_shaders` est vide hors macOS, donc
sans effet ailleurs.

Tout ce qui suppose le système est dans `src/run.rs`, à une exception près :

- `shared_target_dir` écrit dans `~/Library/Caches`. Ailleurs :
  `$XDG_CACHE_HOME` ou `~/.cache`, et `%LOCALAPPDATA%`.
- `move_to_trash` écrit dans `~/.Trash`. Linux suit la spécification XDG
  (`~/.local/share/Trash/files` plus un `.trashinfo`), Windows demande la
  corbeille du shell. Le crate `trash` fait les trois ; à peser contre une
  implémentation maison.
- `open_terminal` et `open_editor` appellent `open -a`. Linux : `$TERMINAL`,
  `gnome-terminal`, `konsole`, et `zed` ou `$EDITOR`. Windows : `wt.exe`,
  `start`.
- `stop` tue un groupe de processus par `kill -TERM -pid`, et `run` appelle
  `process_group(0)` derrière `std::os::unix::process::CommandExt` : cette
  ligne seule empêche la compilation sur Windows. Équivalent Windows :
  `taskkill /T /PID`, ou un objet Job.
- `traffic_light_position` (`about.rs`, `workspace.rs`) est ignoré hors macOS,
  rien à faire.

La forme : un module `platform` avec une dorsale par `#[cfg(target_os)]`, et
`run.rs` qui garde son interface actuelle.

Ce qu'une CI en matrice prouve et ne prouve pas : elle prouve que ça compile et
que les tests passent sur les trois systèmes — c'est déjà l'essentiel, et c'est
ce qui empêche une régression comme le `os::unix` ci-dessus de passer
inaperçue. Elle ne prouve pas que l'interface est utilisable : aucun test ici
n'ouvre de fenêtre, et Linux réclame en plus les paquets de développement
X11/Wayland et Vulkan que gpui attend. Il faudra que quelqu'un lance maxx une
fois sur chaque système.

## Réglages

Trois choses distinctes, dans cet ordre de dépendance.

**Les réglages de maxx.** Sont figés dans le code : l'éditeur (`run.rs:91`), le
terminal (`run.rs:71`), le cache partagé (`run.rs:56`), la palette
(`theme.rs`, des `const`), la largeur des panneaux (240 px et 280 px), et
l'état des panneaux qui repart à zéro à chaque lancement (`workspace.rs:177`).
Le trou visible est `Fichier > Ouvrir un élément récent`, câblé sur `NoAction`
(`menus.rs:36`) : sans réglages persistés il ne peut rien faire. La géométrie
de la fenêtre n'est pas restaurée non plus.

`gpui_component::setting` livre déjà la présentation — `Settings`,
`SettingPage`, `SettingGroup`, `SettingItem` — donc l'écran de préférences est
presque gratuit, seule la persistance est à écrire. `serde_json` et `toml` sont
déjà compilés dans l'arbre via gpui : les prendre en dépendance directe ne
coûte rien en temps de build. L'emplacement du fichier suit la section
Portabilité ci-dessus.

**Ce que maxx retient d'un projet** : projets récents, dernière vue ouverte,
dossiers dépliés, largeur des panneaux pour ce projet. N'appartient pas au
projet — ça polluerait son dépôt et n'aurait aucun sens pour qui le clone.
Va dans l'état de maxx, indexé par chemin.

**Les réglages du projet lui-même**, dans les deux sens :

- ce que maxx devine et devrait savoir : la vue d'entrée est écrite en dur
  (`main_rs()` importe `accueil`), la commande de lancement est toujours
  `cargo run`, sans profil ni features. Un `maxx.toml` à la racine, versionné.
- ce dont l'application générée a besoin pour elle-même : un `src/settings.rs`
  avec sa zone marquée, que maxx édite comme il édite `src/menus.rs`. Du Rust
  ordinaire qui tourne sans maxx, dans le principe du projet.

Le second suppose le premier.

## Choisir son éditeur et son terminal

`open_editor` essaie `zed` puis `open -a Zed`, `open_terminal` essaie Ghostty
puis Terminal : deux chaînes en dur, sans recours si l'utilisateur tient à
autre chose. À remplacer par une détection, une liste, et un choix rangé dans
les réglages de maxx.

La détection, par système : sur macOS, balayage de `/Applications` plus les
binaires sur le `PATH` ; sur Linux, les fichiers `.desktop` de
`/usr/share/applications` ; sur Windows, la base de registre. Et partout, en
dernier recours, `$VISUAL`, `$EDITOR`, `$TERMINAL`, qui coûtent une ligne et
couvrent le cas de celui qui sait ce qu'il veut.

Le vrai travail n'est pas la détection mais `open_editor_at` : ouvrir un
fichier *à une ligne* a une syntaxe par éditeur — `zed fichier:ligne`,
`code -g fichier:ligne`, `subl fichier:ligne`, `nvim +ligne fichier`,
`idea --line N fichier`. C'est une table, pas une heuristique, et c'est elle
qui fait vivre le bouton `→ Zed` de l'inspecteur — dont le libellé devra
suivre l'éditeur choisi.

Piège à ne pas manquer : `hx`, `nvim`, `vim`, `emacs -nw` sont des éditeurs de
terminal. « Ouvrir dedans » veut dire lancer le terminal choisi qui lance
l'éditeur — les deux réglages ne sont pas indépendants.

Relevé sur la machine de développement, à titre d'exemple de ce qu'une
détection trouve : applications Zed et Xcode, terminaux Ghostty et Terminal,
binaires `zed`, `hx`, `nvim`, `vim`.
