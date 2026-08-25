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
- ~~Les deux avertissements clippy~~ — faits, et la CI est stricte
  (`clippy -D warnings`).
- ~~CI GitHub Actions~~ — faite, en matrice sur les trois systèmes, plus un
  travail qui compile la démo.
- ~~Métadonnées `Cargo.toml`~~ — faites, `rust-version = "1.88"` compris.
- **`cargo fmt` une fois** — 82 blocs sur 11 fichiers, surtout des imports.
  Volontairement laissé de côté pour l'instant : le diff toucherait du code
  mis en forme à la main. Le jour où c'est fait, ajouter `fmt --check` à la CI.

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

Le module système et les réglages sont des copies de ce que maxx a écrit pour
lui-même : un défaut corrigé d'un côté doit être porté de l'autre, à la main.
La fiche `.trashinfo` non conforme l'a déjà été. Deux façons d'en sortir un
jour, aucune gratuite : extraire un crate que les projets générés
dépendraient — ce qui casse la promesse « rien ne doit à maxx » —, ou faire
générer le gabarit depuis le code de maxx, ce qui demande que les deux soient
écrits pour ça.

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
