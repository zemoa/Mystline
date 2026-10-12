# Guide de développement

## Contribuer

- Faire des changements petits et cohérents, faciles à relire et à annuler.
- Respecter les conventions existantes avant d'en introduire de nouvelles.
- Garder le code simple et explicite ; éviter les abstractions prématurées.
- Ajouter ou adapter les tests quand un comportement change. Vérifier la modification avant de la proposer.
- Mettre à jour la documentation concernée lorsqu'une règle ou une décision change.
- Expliquer dans une proposition de changement le pourquoi, les choix importants et la manière de vérifier le résultat.

## Vérifier le socle Rust et Tauri

Depuis la racine, vérifier la bibliothèque : `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` et `cargo check --target x86_64-pc-windows-gnu` si la cible Windows est installée. Pour le backend : `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`. Depuis `frontend/`, exécuter `npm ci` et `npm run build`. Le backend Windows complet nécessite la chaîne de compilation et les prérequis Tauri Windows ; le check GNU de la bibliothèque ne le remplace pas. Tester les interactions tray et raccourcis sur des sessions Windows, X11 et Wayland : une compilation croisée ne vérifie pas les comportements du compositeur ou du shell.

## Sonde graphique du lot 1

Après `npm run build` dans `frontend/`, construire depuis la racine avec `cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol`. En debug uniquement, définir `MYSTLINE_PROBE_DIR` vers un répertoire temporaire neuf puis lancer `src-tauri/target/debug/mystline-desktop`. Le rapport `probe-report.json` est écrit dans ce répertoire.

La sonde ouvre, masque et réutilise les trois surfaces, mesure le chargement de la capture, vérifie l'instance unique et le watcher, et mesure la mémoire des processus sous Linux avant/après les fenêtres. Elle utilise une source temporaire et ne modifie ni autostart ni raccourcis système. Elle ne remplace donc pas la validation manuelle de ces intégrations. Elle n'est pas compilée dans un binaire release. Le polling borné est réservé à cette sonde ; l'application utilise les événements natifs et un réveil journalier.

Le comportement par défaut de la sonde est celui de D7 : masquage et réutilisation. Pour retrouver la référence historique de destruction, ajouter `--probe-destroy-windows` ; cette variante est confinée à la sonde debug avec `MYSTLINE_PROBE_DIR`. Les deux modes suivent le même parcours et attendent dix secondes après fermeture, avec une source temporaire vide dans un processus neuf. La sonde vérifie également le focus du champ de capture, l'effacement du brouillon à la fermeture et sa conservation lors d'une simple demande de focus. Le mode normal vérifie aussi la fermeture native des paramètres et le basculement du panneau. Les commandes d'observation du DOM sont réservées à la sonde et exclues du profil release.

## Lire et maintenir les tests F2

Les tests F2 servent de documentation : nommer chaque scénario selon le comportement utilisateur, citer sa règle `RG-F2-xx` et structurer les commentaires en **Étant donné / Quand / Alors**. Vérifier des résultats observables (titre, métadonnées, section, état de saisie, fichier après réouverture), plutôt que recopier les branches de l'implémentation.

Les [parcours persistés](../tests/f2_documentation.rs) s'exécutent avec `cargo test --test f2_documentation`. Ils couvrent capture enrichie, édition, annulation, permutations filtrées, erreurs et conflits avec un éditeur externe. Les tests du [parseur](../src/presentation/input.rs) et du [panneau](../src/presentation/panel.rs) s'exécutent avec `cargo test --lib presentation::`. Les tests du snapshot et de sa coordination sont dans `src/runtime/` : `cargo test --lib runtime::`. Les tests spécifiques à Iced ont été retirés ; leur couverture des interactions devra être rétablie dans les lots Vue concernés, comme indiqué dans [le suivi de migration](migration-tauri-vue.md).

Injecter des dates fixes dans les fonctions de présentation. Simuler le changement de journée en changeant la date fournie, sans attente réelle ni dépendance au fuseau de la machine. Utiliser des fichiers temporaires pour la persistance ; comparer aussi la révision et le contenu du fichier après une action refusée. L'[index F2](features/F2-organiser-et-retrouver-ses-taches.md#tests-comme-documentation) relie chaque règle aux scénarios exécutables.

La validation graphique complémentaire reste nécessaire sur Windows, X11 et Wayland pour le focus système, le redimensionnement des fenêtres et le défilement. Mentionner explicitement si ces vérifications n'ont pas été effectuées.

## Conventional Commits

Format : `type(portée): description courte`, à l'impératif et sans point final. La portée est facultative.

| Type | Usage |
| --- | --- |
| `feat` | Nouvelle fonctionnalité |
| `fix` | Correction d'un défaut |
| `docs` | Documentation uniquement |
| `refactor` | Restructuration sans changement de comportement |
| `test` | Ajout ou modification de tests |
| `perf` | Amélioration des performances |
| `build` | Construction ou dépendances |
| `ci` | Intégration continue |
| `chore` | Entretien sans effet fonctionnel |

Un commit porte un changement cohérent. Ajouter `!` après le type ou la portée pour signaler une rupture de compatibilité, et en expliquer les conséquences dans le corps du message.

```text
docs: clarifier les règles de contribution
fix(ui): corriger la navigation au clavier
feat!: modifier le format des données
```
