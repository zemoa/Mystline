# F0-mise-en-place — Mise en place

## Objectif et périmètre

Rendre l'application exploitable localement, sans compte ni Internet, sur Windows et Linux. Cette feature fournit le processus résident, la configuration locale, le fichier de tâches et les points d'accès système. Elle ne définit ni la création des tâches (F1), ni leur organisation (F2), ni leur fin de vie (F3), ni la modification des raccourcis (F4).

Références : [SFG, §3–4 et §24](../SFG.md) ; [architecture, §2–5 et §10–25](../ARCHITECTURE.md).

## Parcours couverts

1. Au premier lancement, l'application prépare sa configuration et un fichier de tâches local vide, puis reste en arrière-plan sans ouvrir le panneau.
2. Aux lancements suivants, elle reprend la source de tâches configurée et n'ouvre aucune fenêtre principale.
3. Depuis le tray, l'utilisateur peut accéder à la capture et au panneau, ouvrir les paramètres ou quitter explicitement l'application.
4. Depuis les paramètres, il peut désactiver ou réactiver le démarrage automatique et choisir un nouveau fichier de tâches ou un fichier existant.
5. Une modification valide de `tasks.md` depuis un autre outil devient visible dans l'application sans nécessiter son redémarrage.

## Règles de gestion

- **RG-F0-01 — Exécution locale.** Les données restent sur la machine ; aucune fonction ne requiert de compte, de connexion Internet, de cloud ou de synchronisation.
- **RG-F0-02 — Processus résident.** L'application démarre à l'ouverture de session par défaut, sans montrer son panneau ou sa capture. Fermer une surface la masque et ne quitte pas le processus ; seule l'action explicite « Quitter » y met fin. Un second lancement dans la même session quitte silencieusement.
- **RG-F0-03 — Configuration initiale.** La configuration est conservée dans `config.toml` dans le répertoire de configuration standard de l'utilisateur. Au premier lancement, une source locale `tasks.md` est créée si nécessaire dans un emplacement utilisateur, distinct de l'exécutable. Le chemin du fichier de tâches et l'activation du démarrage automatique sont conservés entre les sessions.
- **RG-F0-04 — Source configurable.** Dans les paramètres, l'utilisateur peut créer un fichier de tâches à l'emplacement choisi ou sélectionner un fichier existant. Le nouveau fichier doit pouvoir être créé et chargé, ou le fichier existant doit être valide, avant de devenir la source active et avant l'enregistrement de son chemin. L'ancien fichier n'est jamais déplacé ni supprimé. En cas d'échec, la source actuelle reste active.
- **RG-F0-05 — Source manquante.** Au démarrage, si le chemin configuré n'existe plus, l'application recrée les répertoires parents et un fichier vide, puis poursuit. Une erreur d'accès réelle n'est pas traitée comme un simple fichier absent : l'application ne remplace pas la source par un fichier vide silencieusement.
- **RG-F0-06 — Raccourcis initiaux.** Deux raccourcis globaux distincts sont installés au démarrage : `Ctrl+Shift+Espace` pour la capture et `Ctrl+Shift+T` pour le panneau. Leur modification et la gestion des conflits d'enregistrement appartiennent à F4. Le tray reste un accès aux actions si un raccourci ne fonctionne pas.
- **RG-F0-07 — Tray.** Le menu système donne accès à « Capture rapide », « Ouvrir / masquer le panneau », « Paramètres » et « Quitter ». Il n'affiche ni tâche ni compteur et n'émet pas de notification.
- **RG-F0-08 — Source de vérité.** Toutes les tâches sont enregistrées dans un unique `tasks.md`, dans un Markdown lisible et modifiable par un autre outil. Les regroupements du panneau ne sont pas écrits dans ce fichier. Les dates persistées sont au format `YYYY-MM-DD` ; l'ordre des lignes représente l'ordre relatif des tâches. La forme canonique et les invariants techniques du codec sont ceux de l'architecture.
- **RG-F0-09 — Écriture fiable.** Chaque action applicative qui modifie des tâches est enregistrée immédiatement et de façon atomique. Si l'écriture échoue, l'ancien état confirmé reste la référence ; l'interface revient à cet état et signale l'échec sans présenter l'action comme enregistrée. Une édition extérieure plus récente ne doit pas être écrasée par une action fondée sur une ancienne version.
- **RG-F0-10 — Édition extérieure.** Une édition extérieure valide est reflétée dans le panneau et les autres vues à partir de la nouvelle version du fichier. Une version temporairement invalide ne remplace ni le dernier état valide ni le fichier ; l'application attend une modification ultérieure. Elle ne réécrit jamais spontanément le fichier pour normaliser une édition extérieure. Le cas spécifique d'une tâche cochée sans date de fin est traité en F3.
- **RG-F0-11 — Silence.** Aucun rappel ou notification n'est émis pour les tâches, les dates prévues, les deadlines ou les retards.

## Vérifications fonctionnelles

- Sur Windows et Linux, un premier lancement crée une source locale utilisable, puis les lancements suivants restent discrets et une seconde instance ne duplique pas le processus.
- Le tray permet d'atteindre les surfaces et de quitter ; la fermeture d'une surface laisse l'application accessible.
- La désactivation du démarrage automatique est conservée ; choisir un fichier existant valide bascule sur ce fichier, alors qu'un fichier invalide laisse la source actuelle intacte.
- Une édition extérieure valide s'affiche ; une sauvegarde extérieure temporairement invalide ne détruit ni la vue précédente ni le fichier. Une écriture applicative concurrente ne fait pas perdre cette édition.

## Frontière avec les autres features

F0 fournit et sécurise le stockage, les événements de raccourcis et l'accès aux surfaces. F1 définit ce que font la capture et le panneau ; F2 définit les sections, l'édition et la recherche ; F3 définit la complétion et l'historique ; F4 permet de changer les deux raccourcis globaux.
