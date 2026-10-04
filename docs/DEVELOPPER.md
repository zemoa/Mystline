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
