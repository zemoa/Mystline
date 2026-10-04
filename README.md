# Mystline

Mystline est un projet de gestionnaire de tâches personnel, local et minimaliste. Son objectif est de permettre de noter une tâche en quelques secondes et de retrouver rapidement ce qui est à faire, sans imposer une méthode d’organisation complexe.

## Fonctionnement prévu

- **Capture rapide** : un raccourci clavier global ouvre une petite zone de saisie ; un titre suffit pour créer une tâche.
- **Vue d’ensemble** : un second raccourci affiche un panneau compact répartissant les tâches entre **En retard**, **Aujourd’hui**, **Prochain jour ouvré** et **Toutes les tâches**, sans doublons.
- **Organisation facultative** : une tâche peut avoir une date prévue, une deadline et plusieurs tags. L’ordre manuel des tâches tient lieu de priorité ; aucune heure ni statut « En cours » n’est nécessaire.
- **Suivi** : les tâches peuvent être modifiées, recherchées, filtrées par tag et cochées. Une tâche terminée reste visible le jour même, puis peut être retrouvée dans l’historique.
- **Utilisation au clavier** : la capture, la consultation et les principales actions sur les tâches sont pensées pour être accessibles sans souris.

L’application est prévue pour Windows et Linux, avec des données conservées sur la machine, sans compte, cloud, synchronisation ni notifications. L’architecture prévoit un fichier de tâches Markdown `tasks.md`, lisible et modifiable avec d’autres outils.

Le projet est actuellement décrit par ses spécifications et son architecture ; ce README présente le fonctionnement visé, pas des fonctionnalités déjà disponibles.

## Documentation

- [Spécification fonctionnelle](docs/SFG.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Guide de développement](docs/DEVELOPPER.md)
