# Validation de la refonte fonctionnelle du 22 octobre 2026

Les [maquettes](../designs/refonte-fonctionnelle/README.md) ont été validées par
l'utilisateur avant l'implémentation. Le classement est calculé en Rust : En retard,
En cours, Prochain jour ouvré, Plus tard. Le fichier de tâches garde son format et
les deux états existants.

## Contrôles effectués

| Vérification | Résultat |
| --- | --- |
| `cargo test` | 82 tests réussis : présentation, persistance F2/F3 et coordination. |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 25 tests réussis, 2 tests d'intégration du portail ignorés. |
| `pnpm --dir frontend test` | 29 tests DOM Vue réussis. |
| `pnpm run build` | Typage TypeScript et compilation du frontend réussis. |
| `cargo fmt --check`, avec et sans le manifest Tauri | Réussi. |
| `cargo clippy --all-targets -- -D warnings`, avec et sans le manifest Tauri | Réussi. |
| `cargo check --target x86_64-pc-windows-gnu` | Bibliothèque Rust compilée pour Windows. |
| Build debug Tauri avec `--features custom-protocol` | Réussi. |
| Sonde graphique Wayland sur les vraies WebViews | Réussie ; [rapport](refonte-fonctionnelle-wayland-debug.json). |

Les scénarios couvrent notamment la création sans date automatique, les deadlines seules,
la priorité du retard même avec une date prévue future, les tâches prévues auparavant,
le week-end, le report puis le retrait d'une date, les changements de journée sans
écriture, les tâches terminées et le réordonnancement entre tâches sans date et du jour.
Les tests Vue vérifient les libellés, les compteurs, l'absence de doublons après édition
et les boutons de déplacement aux bornes de section.

La sonde utilise un répertoire temporaire isolé. Elle soumet le véritable formulaire
Vue pour reporter une tâche puis retirer sa date ; elle vérifie son déplacement,
son retour dans En cours, les compteurs et la persistance de la complétion. Les tests
antérieurs de réutilisation des fenêtres, de capture, d'historique, de watcher et de
rejet des anciennes révisions continuent de passer. Les raccourcis globaux et
l'autostart sont inhibés pendant cette sonde.

## Limites

Les interactions natives n'ont pas été rejouées sous Windows ou X11. Le contrôle de
la bibliothèque Windows ne valide pas le backend desktop ni le comportement du système.
La sonde Wayland vérifie les interactions DOM et natives décrites ci-dessus ; elle ne
constitue pas une revue visuelle exhaustive de l'interface.
