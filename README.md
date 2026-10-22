# Mystline

Mystline est un projet de gestionnaire de tâches personnel, local et minimaliste. Son objectif est de permettre de noter une tâche en quelques secondes et de retrouver rapidement ce qui est à faire, sans imposer une méthode d’organisation complexe.

## Fonctionnement prévu

- **Capture rapide** : un raccourci clavier global ouvre une petite zone de saisie ; un titre suffit pour créer une tâche.
- **Vue d’ensemble** : un second raccourci affiche un panneau compact répartissant les tâches entre **En retard**, **En cours**, **Prochain jour ouvré** et **Plus tard**, sans doublons. Les tâches sans date prévue, celles du jour et celles prévues auparavant restent en cours ; une deadline dépassée les place dans En retard.
- **Organisation facultative** : une tâche peut avoir une date prévue, une deadline et plusieurs tags. L’ordre manuel des tâches tient lieu de priorité ; aucune heure ni statut « En cours » n’est nécessaire.
- **Suivi** : les tâches peuvent être modifiées, recherchées, filtrées par tag et cochées. Une tâche terminée reste visible le jour même, puis peut être retrouvée dans l’historique.
- **Utilisation au clavier** : la capture, la consultation et les principales actions sur les tâches sont pensées pour être accessibles sans souris.

L’application est prévue pour Windows et Linux, avec des données conservées sur la machine, sans compte, cloud, synchronisation ni notifications. L’architecture prévoit un fichier de tâches Markdown `tasks.md`, lisible et modifiable avec d’autres outils.

## État du projet

La branche `migration/tauri-vue` remplace directement Iced par Tauri 2 et Vue 3. Le cœur Rust, le format Markdown et les tests de comportements persistés sont conservés.

Le portage fonctionnel F0–F5 vers Tauri/Vue est implémenté : capture persistée, panneau quotidien, organisation, historique, paramètres et raccourcis modifiables. Des agents dédiés à l'implémentation, la revue et les tests ont vérifié les changements. Les résultats et les limites de validation native par plateforme figurent dans [le suivi de migration](docs/migration-tauri-vue.md) et le [tableau d'avancement](docs/validation/migration-agents.html).

## Lancer le socle Tauri

Installer Rust, Node.js et pnpm, ainsi que les [prérequis Tauri de la plateforme](https://v2.tauri.app/start/prerequisites/). Depuis la racine du projet :

```bash
pnpm install --frozen-lockfile
pnpm run desktop:dev
```

`pnpm run desktop:build` construit un binaire local, sans installer. `pnpm run dev` et `pnpm run build` démarrent ou construisent uniquement le frontend Vue. Le workspace pnpm et son lockfile sont à la racine ; les sources Vue restent dans `frontend/`.

Chaque push et pull request construit l'application sur Linux et Windows. Un tag de version `X.Y.Z` (par exemple `0.1.0`) déclenche en plus la création de la release GitHub correspondante, avec les paquets Linux `.deb` et `.AppImage`, ainsi que les installateurs Windows `.msi` et `.exe` (NSIS).

Les versions Windows et Linux AppImage vérifient automatiquement les releases GitHub et
téléchargent les nouvelles versions signées. Dans les paramètres, **Installer et redémarrer**
applique la mise à jour au moment choisi. Sans connexion, les tâches restent utilisables.
Le paquet `.deb` se met à jour en installant le nouveau paquet téléchargé depuis la release.

Le programme reprend `config.toml` et `tasks.md` dans le répertoire de configuration habituel. Il ouvre par défaut la liste quotidienne des tâches à chaque démarrage. Dans **Paramètres → Démarrage**, décocher **Ouvrir la liste des tâches au démarrage** permet de démarrer en arrière-plan ; ce choix est conservé et indépendant du démarrage avec la session. Le tray ouvre la capture ou le panneau ; les paramètres remplacent le contenu du panneau dans cette même fenêtre. Échap, le bouton de fermeture ou une fermeture native masque la fenêtre sans arrêter l'application. Elle est réutilisée au prochain appel, avec un brouillon neuf. « Quitter » arrête le cœur résident.

## Parcours fonctionnels

Les parcours ci-dessous sont portés dans les surfaces Vue. Leur validation et les vérifications restant à effectuer par plateforme figurent dans le suivi de migration.

`Ctrl+Shift+Espace` (ou « Capture rapide » dans le tray) ouvre la saisie : saisir un titre et appuyer sur Entrée pour l'enregistrer, ou Échap pour annuler. Facultativement, ajouter `/p jeudi`, `/d 2026-10-09` et `#mission`. Les dates acceptées sont `YYYY-MM-DD`, `aujourd'hui`, `demain` ou un jour de semaine français. Un titre vide, une commande répétée ou une date invalide ne sont pas enregistrés ; une erreur laisse le texte dans la saisie. Pour écrire un token littéral, utiliser `\#mission` ou `\/p vendredi` ; un antislash littéral s'écrit `\\`. F1 affiche l'aide et Échap la ferme en premier.

`Ctrl+Shift+T` (ou « Ouvrir / masquer le panneau » dans le tray) affiche ou masque les tâches actives et celles terminées aujourd'hui, réparties entre **En retard**, **En cours**, **Prochain jour ouvré** et **Plus tard**. Un titre seul rejoint En cours sans date ajoutée ; les tâches prévues aujourd'hui ou auparavant y restent aussi tant que leur deadline n'est pas dépassée. Une date prévue future reporte la tâche vers le prochain jour ouvré ou Plus tard ; une deadline future seule ne la reporte pas. Les compteurs distinguent tâches à faire et terminées. Le bouton `+` ajoute une tâche directement depuis le panneau. ↑ et ↓ déplacent la sélection ; Entrée édite le titre et les commandes d'une tâche active ; Entrée enregistre et Échap annule. Les dates existantes sont présentées en `YYYY-MM-DD`. Effacer une commande retire sa métadonnée ; retirer `/p ...` replace la tâche En cours, sauf retard de deadline. Une édition devenue obsolète conserve son brouillon et demande de rouvrir la tâche avant de valider.

`Ctrl+F` recherche dans les titres et les tags. Entrée rend la main à la liste en conservant la requête ; Échap retire la recherche avant de fermer le panneau. Cliquer un tag, ou l'atteindre par Tab/Maj+Tab puis Entrée, active son filtre ; « Retirer le filtre » l'enlève. Recherche et filtre se combinent. `Ctrl+↑` et `Ctrl+↓`, ou les boutons « Monter » et « Descendre », échangent la tâche sélectionnée avec sa voisine active visible dans la même section ; les lignes masquées et terminées gardent leur position. Le vendredi, le prochain jour ouvré est lundi, alors que `/p demain` désigne samedi et place la tâche dans Plus tard. Une tâche avec seulement un titre ne demande aucun classement préalable.

Cliquer la case d'une tâche ou presser `Espace` sur sa sélection bascule son état. La terminer enregistre immédiatement la date locale ; la décocher pendant qu'elle est encore visible dans le panneau quotidien la réouvre et retire cette date. Elle reste barrée aujourd'hui, avec ses métadonnées ; une tâche en retard rejoint sa section de planification, **En cours** si elle n'a pas de date prévue ou si celle-ci est aujourd'hui ou passée, sans alerte de deadline. Dès le lendemain, même avec le panneau ouvert, elle quitte la vue quotidienne. L'historique reste consultatif et les déplacements de tâches actives ignorent les tâches terminées.

`Ctrl+H` ou le bouton « Historique », accessible par Tab puis Entrée, affiche toutes les tâches terminées du plus récent au plus ancien, avec leur date de fin. Recherche et filtre par tag restent disponibles et sont conservés entre les vues. ↑/↓ parcourent l'historique ; « Retour », `Ctrl+H` ou Échap revient au quotidien, après avoir retiré une recherche active. Une nouvelle ouverture du panneau présente le quotidien.

Cocher extérieurement une ligne avec `@completed(YYYY-MM-DD)` utilise cette date. Sans date, le jour d'observation sert de date de fin ; aucune écriture immédiate n'est déclenchée et la prochaine sauvegarde applicative fixe cette date. Un redémarrage avant cette sauvegarde renouvelle la date d'observation.

Les deux raccourcis globaux se modifient dans **Paramètres** en saisissant une combinaison puis en choisissant « Valider » (ou avec Entrée dans le champ). Une combinaison identique pour les deux actions est refusée. Si le système refuse une combinaison, elle reste enregistrée et apparaît « Inactif » ; le tray permet toujours d'accéder à l'action.

Les diagnostics techniques sont écrits dans des fichiers quotidiens `logs/mystline.log.YYYY-MM-DD` du répertoire de configuration. Ils décrivent les opérations, révisions et erreurs de stockage sans enregistrer le contenu des tâches.

Sur Wayland, les raccourcis passent par **XDG Global Shortcuts** : le portail du bureau peut demander une autorisation ou refuser une combinaison ; les paramètres indiquent alors son état. L'icône de tray Linux utilise StatusNotifierItem (KSNI) et requiert un bureau qui l'affiche. Pour conserver le démarrage automatique après le développement, lancer un binaire installé à un emplacement stable plutôt que `src-tauri/target/debug/mystline-desktop`.

Sous KDE, accepter la demande de raccourcis de Mystline une fois lors du premier lancement ou après un changement de combinaison. Une action sans touche attribuée apparaît « Inactif », même si le portail connaît son nom. Les attributions se consultent aussi dans les paramètres système de KDE, rubrique « Raccourcis », sous « Mystline » (`local.mystline`). Les anciennes demandes des versions de développement peuvent apparaître sous Konsole ou VSCodium.

## Documentation

- [Spécification fonctionnelle](docs/SFG.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Guide de développement](docs/DEVELOPPER.md)
- [Scénarios F2 et tests exécutables](docs/features/F2-organiser-et-retrouver-ses-taches.md#tests-comme-documentation)
- [Scénarios F3 et tests exécutables](docs/features/F3-terminer-et-suivre-ses-taches.md#tests-comme-documentation)
- [Consolidation visuelle F5](docs/features/F5-consolider-interface.md)
