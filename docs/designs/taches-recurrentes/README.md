# Tâches récurrentes — exploration fonctionnelle

Statut : règles métier et syntaxe des commandes validées le 31 octobre 2026 ; maquettes et compléments de parcours proposés. Évolution non implémentée. La référence fonctionnelle est désormais [F6](../../features/F6-gerer-les-taches-recurrentes.md), synthétisée dans la [SFG §23](../../SFG.md).

## Besoin exprimé

Pouvoir ajouter une tâche récurrente, par exemple tous les vendredis ou tous les 30 jours.

## Première proposition visuelle

[Voir la maquette SVG](proposition.svg).

La maquette reprend la liste continue et la palette Indigo de la présentation actuelle. Elle propose un libellé de répétition sous le titre et un aperçu de la règle pendant la saisie. Les commandes `/r vendredi` et `/r 30j` sont validées, sans modification du parseur ni de l'application à ce stade.

## Lecture des maquettes

Les exemples sont fictifs. Les SVG sont des maquettes statiques, sans persistance. La palette Indigo et la liste continue reprennent la présentation actuelle. Le détail des interactions reste proposé ; la validation des règles métier ne vaut pas validation visuelle de ces écrans.

| Maquette | Contenu |
| --- | --- |
| [Saisie et liste](proposition.svg) | Commande `/r` validée, aperçu et libellé de récurrence proposés sous le titre |
| [Formulaire détaillé](formulaire.svg) | Deux exemples : hebdomadaire et intervalle de 30 jours |
| [Les quatre fréquences](frequences.svg) | Quotidien, plusieurs jours par semaine, intervalle et mensuel ; aperçu des dates |
| [Cycle après cochage](cycle.svg) | Cumul des métadonnées et distinction entre l'occurrence terminée et la suivante à faire |

## Décisions validées le 31 octobre 2026

- La répétition suit un calendrier fixe, indépendant de la date de réalisation : une tâche prévue le 1er novembre et terminée le 5 conserve sa prochaine date au 1er décembre pour un intervalle de 30 jours.
- Une seule occurrence reste à faire ; les périodes manquées n'accumulent pas plusieurs tâches.
- La récurrence peut être définie par une commande de saisie ou par un formulaire. Les formes validées sont `/r jour`, `/r vendredi`, `/r lundi,vendredi`, `/r 30j` et `/r mois:31`.
- Après cochage, l'occurrence terminée reste barrée aujourd'hui et la prochaine occurrence apparaît immédiatement avec sa date future.
- Après une réalisation tardive, la prochaine date est la première date du calendrier strictement après le jour de réalisation. Un cochage anticipé consomme la date théorique prévue. Les dates manquées ne produisent pas d'occurrences à rattraper.
- Fréquences retenues à la suite de la proposition : tous les jours, chaque semaine avec un ou plusieurs jours, tous les N jours et chaque mois à la même date.
- Pour un jour absent d'un mois, utiliser le dernier jour puis revenir au jour demandé le mois suivant.
- Un report ponctuel conserve le calendrier de la série.
- Date prévue, deadline, tags et récurrence doivent pouvoir se cumuler sur une occurrence unique, sans doublon dans la liste.
- La deadline des prochaines occurrences conserve l'écart initial avec la date prévue de référence. Un report ponctuel conserve la deadline existante.
- Modifier le titre ou les tags concerne l'occurrence à faire et les suivantes. Aucune fonction de modification rétroactive de l'historique n'est demandée.
- « Ne plus répéter » conserve l'occurrence en cours comme tâche ponctuelle.
- Un cochage anticipé consomme l'occurrence prévue et passe à la date théorique suivante.
- Décocher l'occurrence terminée rouvre celle-ci et retire la suivante encore à faire. Si la suivante est déjà terminée, l'ancienne n'est plus réouvrable.
- La première date proposée est la prochaine date compatible, aujourd'hui compris ; l'utilisateur peut la modifier.
- Modifier la fréquence ou la règle de deadline concerne l'occurrence à faire et les suivantes, avec aperçu avant validation.
- La répétition continue jusqu'à « Ne plus répéter », sans fin programmée ni nombre maximal de répétitions.

## Deuxième proposition : formulaire et complétion

[Voir le formulaire proposé](formulaire.svg).

Le formulaire propose « Chaque semaine » avec un jour ou « Tous les N jours » avec une date de départ. Un aperçu rend les prochaines dates explicites. Les valeurs et la présentation sont des propositions.

Le cycle après cochage et l'absence de rattrapage sont validés. La présentation du formulaire reste une proposition ; la [planche des fréquences](frequences.svg) complète les exemples initiaux. Tant qu'elle n'est pas cochée, l'occurrence en attente reste la seule à faire ; un changement de jour n'en crée pas d'autre.

La [spécification F6](../../features/F6-gerer-les-taches-recurrentes.md) rassemble les règles et la syntaxe validées, les propositions de parcours et les points à instruire avant développement.

[Troisième maquette : métadonnées cumulées et résultat après cochage](cycle.svg). Les informations validées sont illustrées sur une occurrence unique ; la ligne terminée et la prochaine occurrence correspondent à deux réalisations distinctes.
