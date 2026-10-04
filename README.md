# Mystline

Mystline est un projet de gestionnaire de tâches personnel, local et minimaliste. Son objectif est de permettre de noter une tâche en quelques secondes et de retrouver rapidement ce qui est à faire, sans imposer une méthode d’organisation complexe.

## Fonctionnement prévu

- **Capture rapide** : un raccourci clavier global ouvre une petite zone de saisie ; un titre suffit pour créer une tâche.
- **Vue d’ensemble** : un second raccourci affiche un panneau compact répartissant les tâches entre **En retard**, **Aujourd’hui**, **Prochain jour ouvré** et **Toutes les tâches**, sans doublons.
- **Organisation facultative** : une tâche peut avoir une date prévue, une deadline et plusieurs tags. L’ordre manuel des tâches tient lieu de priorité ; aucune heure ni statut « En cours » n’est nécessaire.
- **Suivi** : les tâches peuvent être modifiées, recherchées, filtrées par tag et cochées. Une tâche terminée reste visible le jour même, puis peut être retrouvée dans l’historique.
- **Utilisation au clavier** : la capture, la consultation et les principales actions sur les tâches sont pensées pour être accessibles sans souris.

L’application est prévue pour Windows et Linux, avec des données conservées sur la machine, sans compte, cloud, synchronisation ni notifications. L’architecture prévoit un fichier de tâches Markdown `tasks.md`, lisible et modifiable avec d’autres outils.

## État du projet

Le socle **F0 — Mise en place** est implémenté : processus résident, fichier Markdown local, détection des éditions extérieures, configuration du démarrage automatique, tray et raccourcis initiaux sous Windows, X11 et Wayland. La capture d'une tâche depuis la fenêtre et la consultation au clavier appartiennent à F1 ; l'organisation et la complétion viendront ensuite.

## Lancer l'application

Installer Rust puis lancer `cargo run` depuis le projet. Le programme crée `config.toml` et `tasks.md` dans le répertoire de configuration de l'utilisateur et reste en arrière-plan. Le tray donne accès au panneau, aux paramètres et à « Quitter ». Dans les paramètres, saisir un chemin absolu pour utiliser un fichier Markdown existant ou créer un nouveau fichier. La saisie d'un chemin se fait au clavier.

Sur Wayland, les raccourcis passent par **XDG Global Shortcuts** : le portail du bureau peut demander une autorisation ou refuser une combinaison ; les paramètres indiquent alors son état. L'icône de tray Linux utilise StatusNotifierItem (KSNI) et requiert un bureau qui l'affiche. Pour conserver le démarrage automatique après le développement, lancer un binaire installé à un emplacement stable plutôt que `target/debug/mystline`.

## Documentation

- [Spécification fonctionnelle](docs/SFG.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Guide de développement](docs/DEVELOPPER.md)
