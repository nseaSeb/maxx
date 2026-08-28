# Dépendances et licences

maxx est sous licence MIT (voir `LICENSE`). Les bibliothèques qu'il utilise
restent sous la leur : ce fichier récapitule ce que leur redistribution
demande. Il décrit l'état du graphe de dépendances tel que `Cargo.lock` le fige
pour la cible macOS ; il est à revoir après toute mise à jour du verrou.

**Ce que maxx distribue aujourd'hui : de la source, et rien d'autre.** Le dépôt
ne porte aucun exécutable, la version ouverte par un tag n'en attache aucun, et
crates.io n'en reçoit pas non plus. Or les obligations décrites plus bas — le
texte de la licence Apache-2.0 à joindre, les mentions de copyright à
conserver, la source du crate MPL-2.0 à indiquer — portent toutes sur la
distribution d'un **binaire**. Aucune ne s'applique donc en l'état. Ce fichier
existe pour le jour où elle changerait : un paquet Homebrew, un `.app` signé,
un exécutable joint à une version. Ce jour-là, il faudra faire voyager ces
mentions avec le fichier distribué — la fenêtre « À propos » est l'endroit
habituel pour une application à interface.

La liste complète et à jour se lit avec :

```
cargo tree -e normal --target aarch64-apple-darwin
```

## Apache-2.0

- **gpui 0.2.2** — Zed Industries. Le moteur d'interface.
- **gpui-component 0.5.1** — les composants construits dessus.

Ni l'un ni l'autre ne livre de fichier `NOTICE`, il n'y a donc rien à propager
à ce titre. Distribuer un binaire de maxx demande de joindre le texte de la
licence Apache-2.0 et de conserver les mentions de copyright. maxx ne modifie
aucun fichier de ces deux crates : la clause sur la signalisation des
modifications ne s'applique pas.

Environ un quart des crates transitives sont sous Apache-2.0 seule ; la grande
majorité du reste est sous MIT, ou au choix MIT ou Apache-2.0.

## MPL-2.0

- **option-ext 0.2.0**, tiré par `dirs` ▸ `dirs-sys` ▸ `zed-font-kit` ▸ `gpui`.

La MPL-2.0 est un copyleft par fichier : elle n'atteint pas le code de maxx,
mais distribuer un binaire qui la contient oblige à indiquer aux destinataires
où obtenir la source de ce crate — <https://crates.io/crates/option-ext>.

`cbindgen` (MPL-2.0), `dwrote` (MPL-2.0) et `self_cell` (Apache-2.0 ou
GPL-2.0) figurent dans `Cargo.lock` mais ne sont pas dans le graphe de
compilation macOS : dépendances de compilation ou d'une autre plateforme.

## Une licence qu'aucun outil ne voit

- **tree-sitter-graphql 0.1.0**, tiré par la feature `tree-sitter-languages`.

Son `Cargo.toml` déclare `license-file = "LICENSE"` et non `license`, si bien
que `cargo metadata` — et tout ce qui le lit — rend un champ vide. Le fichier
est là et dit MIT, copyright 2025 Joohwan Oh. Rien de particulier à faire, sauf
ne pas s'en alarmer au prochain audit.

## Autres licences présentes

Unicode-3.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, CC0-1.0, Unlicense.
Toutes permissives, toutes satisfaites par la conservation des mentions de
copyright dans une page d'attributions.
