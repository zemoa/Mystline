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

Le socle **F0 — Mise en place**, **F1 — Capturer et consulter ses tâches** et **F2 — Organiser et retrouver ses tâches** sont implémentés : processus résident, fichier Markdown local, détection des éditions extérieures, configuration du démarrage automatique, tray et raccourcis initiaux sous Windows, X11 et Wayland. La capture, l'édition, les dates, les tags, les quatre sections, la recherche, le filtre par tag et l'ordre manuel sont disponibles. La complétion et l'historique relèvent de F3 et viendront ensuite.

## Lancer l'application

Installer Rust puis lancer `cargo run` depuis le projet. Le programme crée `config.toml` et `tasks.md` dans le répertoire de configuration de l'utilisateur et reste en arrière-plan. Un clic principal sur l'icône de la zone de notification ouvre le panneau des tâches ou lui redonne le focus s'il est déjà ouvert. Un clic secondaire ouvre le menu du tray, qui donne accès au panneau, aux paramètres et à « Quitter ». Dans les paramètres, saisir un chemin absolu pour utiliser un fichier Markdown existant ou créer un nouveau fichier. La saisie d'un chemin se fait au clavier.

`Ctrl+Shift+Espace` (ou « Capture rapide » dans le tray) ouvre la saisie : saisir un titre et appuyer sur Entrée pour l'enregistrer, ou Échap pour annuler. Facultativement, ajouter `/p jeudi`, `/d 2026-10-09` et `#mission`. Les dates acceptées sont `YYYY-MM-DD`, `aujourd'hui`, `demain` ou un jour de semaine français. Un titre vide, une commande répétée ou une date invalide ne sont pas enregistrés ; une erreur laisse le texte dans la saisie. Pour écrire un token littéral, utiliser `\#mission` ou `\/p vendredi` ; un antislash littéral s'écrit `\\`. F1 affiche l'aide et Échap la ferme en premier.

`Ctrl+Shift+T` (ou « Ouvrir / masquer le panneau » dans le tray) affiche ou masque les tâches actives, réparties entre **En retard**, **Aujourd'hui**, **Prochain jour ouvré** et **Toutes les tâches**. ↑ et ↓ déplacent la sélection ; Entrée édite son titre et ses commandes ; Entrée enregistre et Échap annule. Les dates existantes sont présentées en `YYYY-MM-DD`. Effacer une commande retire sa métadonnée. Une édition devenue obsolète conserve son brouillon et demande de rouvrir la tâche avant de valider.

`Ctrl+F` recherche dans les titres et les tags. Entrée rend la main à la liste en conservant la requête ; Échap retire la recherche avant de fermer le panneau. Cliquer un tag, ou l'atteindre par Tab/Maj+Tab puis Entrée, active son filtre ; « Retirer le filtre » l'enlève. Recherche et filtre se combinent. `Ctrl+↑` et `Ctrl+↓` échangent la tâche sélectionnée avec sa voisine visible dans la même section ; les lignes masquées gardent leur position. Le vendredi, le prochain jour ouvré est lundi, alors que `/p demain` désigne samedi. Une tâche avec seulement un titre ne demande aucun classement préalable.

Sur Wayland, les raccourcis passent par **XDG Global Shortcuts** : le portail du bureau peut demander une autorisation ou refuser une combinaison ; les paramètres indiquent alors son état. L'icône de tray Linux utilise StatusNotifierItem (KSNI) et requiert un bureau qui l'affiche. Pour conserver le démarrage automatique après le développement, lancer un binaire installé à un emplacement stable plutôt que `target/debug/mystline`.

## Documentation

- [Spécification fonctionnelle](docs/SFG.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Guide de développement](docs/DEVELOPPER.md)
- [Scénarios F2 et tests exécutables](docs/features/F2-organiser-et-retrouver-ses-taches.md#tests-comme-documentation)
