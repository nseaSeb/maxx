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
  (`main_rs()` importe `accueil`), la commande de lancement est toujours
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
