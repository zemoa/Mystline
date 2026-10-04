# Guide de développement

## Contribuer

- Faire des changements petits et cohérents, faciles à relire et à annuler.
- Respecter les conventions existantes avant d'en introduire de nouvelles.
- Garder le code simple et explicite ; éviter les abstractions prématurées.
- Ajouter ou adapter les tests quand un comportement change. Vérifier la modification avant de la proposer.
- Mettre à jour la documentation concernée lorsqu'une règle ou une décision change.
- Expliquer dans une proposition de changement le pourquoi, les choix importants et la manière de vérifier le résultat.

## Vérifier le socle Rust

Depuis la racine du projet : `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` et `cargo check --target x86_64-pc-windows-gnu` si la cible Windows est installée. Tester les interactions tray et raccourcis sur des sessions Windows, X11 et Wayland : une compilation croisée ne vérifie pas les comportements du compositeur ou du shell.

## Lire et maintenir les tests F2

Les tests F2 servent de documentation : nommer chaque scénario selon le comportement utilisateur, citer sa règle `RG-F2-xx` et structurer les commentaires en **Étant donné / Quand / Alors**. Vérifier des résultats observables (titre, métadonnées, section, état de saisie, fichier après réouverture), plutôt que recopier les branches de l'implémentation.

Les [parcours persistés](../tests/f2_documentation.rs) s'exécutent avec `cargo test --test f2_documentation`. Ils couvrent capture enrichie, édition, annulation, permutations filtrées, erreurs et conflits avec un éditeur externe. Les tests du [parseur](../src/presentation/input.rs) et du [panneau](../src/presentation/panel.rs) s'exécutent avec `cargo test --lib presentation::`. Les tests Iced dans `src/ui.rs` vérifient les messages de capture, le routage clavier et les vrais widgets de focus/activation sans ouvrir de fenêtre : `cargo test --bin mystline ui::tests::`.

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
