# Guide de développement

## Contribuer

- Faire des changements petits et cohérents, faciles à relire et à annuler.
- Respecter les conventions existantes avant d'en introduire de nouvelles.
- Garder le code simple et explicite ; éviter les abstractions prématurées.
- Ajouter ou adapter les tests quand un comportement change. Vérifier la modification avant de la proposer.
- Mettre à jour la documentation concernée lorsqu'une règle ou une décision change.
- Expliquer dans une proposition de changement le pourquoi, les choix importants et la manière de vérifier le résultat.

## Vérifier le socle Rust et Tauri

Depuis la racine, vérifier la bibliothèque : `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` et `cargo check --target x86_64-pc-windows-gnu` si la cible Windows est installée. Pour le backend : `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`. Toujours depuis la racine, exécuter `pnpm install --frozen-lockfile` et `pnpm run build` pour le frontend, `pnpm run desktop:dev` pour démarrer l'application complète, ou `pnpm run desktop:build` pour construire son binaire sans packaging. Le workspace pnpm, le lockfile et la CLI Tauri sont gérés à la racine ; les scripts `dev` et `build` délèguent au frontend. Le backend Windows complet nécessite la chaîne de compilation et les prérequis Tauri Windows ; le check GNU de la bibliothèque ne le remplace pas. Tester les interactions tray et raccourcis sur des sessions Windows, X11 et Wayland : une compilation croisée ne vérifie pas les comportements du compositeur ou du shell.

Pour vérifier l'identité Wayland sur une session graphique avec un portail XDG actif : `cargo test --manifest-path src-tauri/Cargo.toml native_portal_accepts_mystline_identity_before_creating_session -- --ignored`. Ce test crée si nécessaire l'entrée locale cachée `local.mystline.desktop` dans le dossier XDG des applications, puis enregistre l'identité et ouvre/ferme une session sans attribuer de touche. Les tests ordinaires couvrent les actions sans combinaison attribuée et le renouvellement des deux actions. Pour valider leur déclenchement réel sous KDE, relancer le binaire, accepter la demande du portail, essayer les deux raccourcis, puis modifier l'un et vérifier que l'autre fonctionne toujours.

Après autorisation des deux raccourcis sous KDE, `cargo test --manifest-path src-tauri/Cargo.toml native_kde_restores_granted_shortcuts_in_new_sessions -- --ignored` vérifie leur restauration dans deux connexions et sessions neuves. Il lit les attributions existantes sans dialogue ni réattribution. L'identifiant du portail doit être `local.mystline`, sans suffixe `.desktop` : KGlobalAccel réserve ce suffixe aux lanceurs, ce qui empêche les actions dynamiques de fonctionner et provoque une nouvelle demande à chaque lancement. Les diagnostics `portal_shortcut_activated` permettent de vérifier la réception de chaque action avant l'ouverture de la surface.

## Intégration continue et releases

Le workflow GitHub Actions construit l'application Tauri sur Linux et Windows pour chaque push et pull request. Sur Linux, il exécute aussi le formatage, Clippy, les tests Rust et les tests DOM Vue. Un push de tag `X.Y.Z` crée une release GitHub du même tag et y publie les paquets `.deb` et `.AppImage` pour Linux, ainsi que `.msi` et l'installeur NSIS `.exe` pour Windows. La version du tag doit être alignée avec les manifests avant de le pousser.

Les contrôles Linux s'exécutent dans un job indépendant, en parallèle des deux compilations release sans packaging. La publication attend la réussite des trois jobs. Chaque job conserve son propre cache Rust ; dans la configuration de `Swatinem/rust-cache`, le chemin cible est relatif au projet Cargo : `src-tauri -> target` désigne donc `src-tauri/target`. Un nouveau push sur une même branche ou pull request annule l'exécution précédente encore en cours ; les exécutions de tags ne sont pas annulées.

## Sonde graphique debug

Après `pnpm run build` à la racine, construire avec `cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol`. En debug uniquement, définir `MYSTLINE_PROBE_DIR` vers un répertoire temporaire neuf puis lancer `src-tauri/target/debug/mystline-desktop`. Le rapport `probe-report.json` est écrit dans ce répertoire.

La sonde ouvre, masque et réutilise la capture et le panneau, puis vérifie que les paramètres remplacent le contenu du panneau sans créer de fenêtre supplémentaire. Elle mesure le chargement de la capture, vérifie l'instance unique et le watcher, et mesure la mémoire des processus sous Linux avant/après les fenêtres. Elle utilise une source temporaire et ne modifie ni autostart ni raccourcis système. Elle ne remplace donc pas la validation manuelle de ces intégrations. Elle n'est pas compilée dans un binaire release. Le polling borné est réservé à cette sonde ; l'application utilise les événements natifs et un réveil journalier.

Le portage fonctionnel étend cette sonde avec la soumission DOM de captures valides et invalides, la confirmation d'écriture, les projections natives, la complétion, l'historique, les révisions obsolètes et les changements de source. Les commandes d'observation supplémentaires restent confinées au mode sonde debug. Les changements de configuration utilisent son répertoire temporaire et inhibent les modifications d'autostart et de raccourcis au niveau du système.

Le comportement par défaut de la sonde est celui de D7 : masquage et réutilisation. Pour retrouver la référence historique de destruction, ajouter `--probe-destroy-windows` ; cette variante est confinée à la sonde debug avec `MYSTLINE_PROBE_DIR`. Les deux modes suivent le même parcours et attendent dix secondes après fermeture, avec une source temporaire vide dans un processus neuf. La sonde vérifie également le focus du champ de capture, l'effacement du brouillon à la fermeture et sa conservation lors d'une simple demande de focus. Le mode normal vérifie aussi la fermeture native des paramètres et le basculement du panneau. Les commandes d'observation du DOM sont réservées à la sonde et exclues du profil release.

## Lire et maintenir les tests F2

Les tests F2 servent de documentation : nommer chaque scénario selon le comportement utilisateur, citer sa règle `RG-F2-xx` et structurer les commentaires en **Étant donné / Quand / Alors**. Vérifier des résultats observables (titre, métadonnées, section, état de saisie, fichier après réouverture), plutôt que recopier les branches de l'implémentation.

Les [parcours persistés](../tests/f2_documentation.rs) s'exécutent avec `cargo test --test f2_documentation`. Ils couvrent capture enrichie, édition, annulation, permutations filtrées, erreurs et conflits avec un éditeur externe. Les tests du [parseur](../src/presentation/input.rs) et du [panneau](../src/presentation/panel.rs) s'exécutent avec `cargo test --lib presentation::`. Les tests du snapshot et de sa coordination sont dans `src/runtime/` : `cargo test --lib runtime::`. Les tests spécifiques à Iced ont été retirés ; les scénarios DOM Vue et les tests du contrat IPC complètent désormais les parcours Rust, comme indiqué dans [le suivi de migration](migration-tauri-vue.md).

Injecter des dates fixes dans les fonctions de présentation. Simuler le changement de journée en changeant la date fournie, sans attente réelle ni dépendance au fuseau de la machine. Utiliser des fichiers temporaires pour la persistance ; comparer aussi la révision et le contenu du fichier après une action refusée. L'[index F2](features/F2-organiser-et-retrouver-ses-taches.md#tests-comme-documentation) relie chaque règle aux scénarios exécutables.

La validation graphique complémentaire reste nécessaire sur Windows, X11 et Wayland pour le focus système, le redimensionnement des fenêtres et le défilement. Mentionner explicitement si ces vérifications n'ont pas été effectuées.

## Vérifier le portage fonctionnel Vue

Depuis la racine, `pnpm --dir frontend test` exécute les scénarios DOM de `frontend/tests/App.test.ts` avec Vitest, Vue Test Utils et jsdom. Ils montent le composant Vue et remplacent les appels Tauri par un adaptateur contrôlé : ils vérifient les interactions et les réponses différées du frontend, sans valider l'intégration native ni la persistance réelle.

`cargo test --test migration_runtime` vérifie les captures en attente après conflit, les résultats dupliqués et le rejet d'une commande provenant d'une ancienne source. `cargo test --manifest-path src-tauri/Cargo.toml tasks::tests` vérifie les DTO JSON et les intentions exposées à Vue : projections, recherche et tag, historique, brouillons échappés et voisins visibles.

Ces scénarios complètent les parcours persistés F2/F3 et les vérifications de compilation. Les tests sans fenêtre et les mocks IPC ne remplacent pas une sonde native ni les essais des raccourcis, du tray et de l'autostart dans chaque session graphique.

## Régénérer les icônes

Le symbole source est `frontend/public/mystline-mark.svg`. `python3 scripts/export-icons.py` régénère les logos de `docs/designs/logos/` et les assets de `src-tauri/icons/` : PNG, ICO et buffer RGBA 32 × 32 partagé par les trays Linux et Windows. Cet export utilise PyGObject/Rsvg, Pycairo, Pillow et Noto Sans ; ces outils ne sont pas des dépendances applicatives. Après une modification du dessin, conserver la source et ses exports dans le même changement.

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
