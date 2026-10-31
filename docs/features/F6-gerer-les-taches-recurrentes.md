# F6 — Gérer les tâches récurrentes

Statut : **implémentée ; recette de haut niveau validée par l’utilisateur avant développement le 31 octobre 2026**. Les règles métier et la syntaxe restent la référence ci-dessous. Les maquettes restent exploratoires ; elles ne constituent pas une validation visuelle du formulaire livré.

## Objectif

Définir une tâche qui revient suivant un calendrier fixe, sans ressaisie et sans accumulation de tâches manquées. La récurrence reste facultative et doit préserver la capture rapide et la liste continue.

## Règles validées pendant le cadrage du 31 octobre 2026

- **RG-F6-01 — Calendrier fixe.** Les dates de répétition sont indépendantes du jour de réalisation. Pour une première date le 1er novembre et un intervalle de 30 jours, une réalisation le 5 novembre laisse la prochaine date au 1er décembre. Un intervalle de 30 jours est distinct d'une répétition mensuelle.
- **RG-F6-02 — Une seule occurrence à faire.** Une série récurrente possède une seule occurrence à faire. Les répétitions manquées ne créent pas une accumulation d'occurrences à réaliser.
- **RG-F6-03 — Deux modes de saisie.** L'utilisateur peut définir une répétition avec une commande explicite dans la saisie ou avec un formulaire. Les valeurs par défaut sont définies par RG-F6-15 ; les formes des commandes sont validées dans la section Syntaxe. Le formulaire et les confirmations sont couverts par les tests DOM ; la validation visuelle native reste à effectuer.
- **RG-F6-04 — Cochage et visibilité.** Au cochage, l'occurrence terminée reste barrée dans la liste pendant la journée et est conservée dans l'historique, selon les principes actuels de F3. La prochaine occurrence apparaît immédiatement dans la liste avec sa date prévue future. Les deux lignes correspondent à deux occurrences distinctes ; une seule reste à faire.
- **RG-F6-05 — Dates manquées.** Après cochage, la prochaine occurrence est prévue à la première date du calendrier strictement postérieure au jour de réalisation et à la date théorique de l'occurrence qui vient d'être réalisée. Cette seconde condition couvre le cochage anticipé (RG-F6-13). Exemple : une tâche prévue vendredi 6 novembre 2026 et cochée lundi 23 novembre donne une prochaine occurrence vendredi 27 novembre. Aucune occurrence n'est créée pour les 13 et 20 novembre.
- **RG-F6-06 — Fréquences.** Le cadrage retient les rythmes proposés : tous les jours, chaque semaine avec un ou plusieurs jours choisis, tous les N jours et chaque mois à la même date. RG-F6-07 précise les jours absents d'un mois.
- **RG-F6-07 — Fin de mois.** Si le jour demandé n'existe pas dans un mois, l'occurrence est prévue le dernier jour de ce mois. Le jour initial reste la référence : une série au 31 passe au 28 février 2027 puis au 31 mars, sans dériver au 28 de chaque mois.
- **RG-F6-08 — Report ponctuel.** Déplacer exceptionnellement la date prévue d'une occurrence ne déplace pas le calendrier de la série. Une occurrence du vendredi reportée au lundi ne transforme pas la série en répétition du lundi. Le report n'accumule pas d'occurrences à faire ; au cochage, RG-F6-05 choisit la prochaine date du calendrier.
- **RG-F6-09 — Cumul sans doublon.** Une occurrence peut cumuler les fonctionnalités applicables à une tâche, notamment date prévue, deadline, tags et récurrence. Ce cumul ne crée pas de ligne supplémentaire pour la même occurrence. Le classement et les indications s'appliquent à cette occurrence unique selon les règles de la SFG. Une deadline ne génère pas sa propre tâche. L'occurrence terminée du jour et la prochaine occurrence, prévues par RG-F6-04, représentent deux réalisations distinctes de la série.
- **RG-F6-10 — Deadline répétée.** La deadline des prochaines occurrences conserve le même écart en jours calendaires avec la date prévue de référence. Exemple : une occurrence prévue vendredi 6 novembre avec une deadline lundi 9 novembre donne une occurrence vendredi 13 novembre avec une deadline lundi 16 novembre. Un report ponctuel ne modifie pas la deadline de l'occurrence reportée et ne change pas cet écart pour la série. La distinction entre date prévue de référence et date prévue reportée doit être conservée fonctionnellement.
- **RG-F6-11 — Titre et tags.** Une modification du titre ou des tags s'applique à l'occurrence à faire et aux suivantes. L'utilisateur ne demande pas de gestion des modifications rétroactives de l'historique ; aucune fonction dédiée n'est prévue. Les anciennes occurrences peuvent conserver leurs informations selon le fonctionnement actuel de F3.
- **RG-F6-12 — Arrêter la répétition.** L'action « Ne plus répéter » conserve l'occurrence à faire et ses informations comme tâche ponctuelle. La cocher ne génère plus d'occurrence suivante.
- **RG-F6-13 — Réalisation anticipée.** Cocher une occurrence future la considère comme réalisée pour sa date théorique, même si le jour du cochage est antérieur. Une tâche prévue vendredi et terminée jeudi donne une prochaine occurrence le vendredi suivant, pas le lendemain du cochage. La date de fin reste le jour réel du cochage.
- **RG-F6-14 — Annuler un cochage.** Décocher une occurrence terminée encore visible dans le panneau quotidien la rouvre et retire l'occurrence suivante encore à faire, afin de garder une seule occurrence à faire. Si cette suivante a déjà été cochée, l'ancienne occurrence n'est plus réouvrable et reste consultable dans l'historique. L'historique n'offre pas d'action de réouverture. Avant réouverture, une confirmation annonce la suppression de la suivante et de ses éventuelles modifications, et permet d’annuler (parcours validé avec la recette).

- **RG-F6-15 — Première date.** Proposer la prochaine date compatible avec la règle, aujourd'hui compris. L'utilisateur peut la modifier. Pour chaque jour ou tous les N jours, la proposition initiale est aujourd'hui ; pour chaque semaine, le prochain jour sélectionné ; pour chaque mois, la prochaine date correspondant au jour demandé, avec RG-F6-07 pour les mois courts. Une date incompatible avec la règle est signalée avant l'enregistrement.
- **RG-F6-16 — Modifier la règle.** Modifier la fréquence concerne l'occurrence à faire et les suivantes, avec une date de départ explicite et un aperçu avant enregistrement. Aucune occurrence à faire supplémentaire n'est créée. Modifier la deadline redéfinit l'écart avec la date théorique de l'occurrence, pour celle-ci et les suivantes ; l'aperçu rend ce résultat visible. Reporter seulement la date prévue reste une opération ponctuelle distincte.
- **RG-F6-17 — Fin de série.** La répétition continue jusqu'à l'action « Ne plus répéter ». Ni nombre d'occurrences ni date de fin programmée dans cette première version.

## Conséquences fonctionnelles des règles validées

- Le passage du temps ou un redémarrage ne crée pas une nouvelle occurrence à faire : celle en attente conserve sa date, y compris si plusieurs répétitions sont passées. Les dates futures ne sont générées qu'après cochage, selon RG-F6-05.
- La date théorique est une date du calendrier de la série. La date prévue est celle affichée à l'utilisateur et peut être reportée. Les distinguer évite de décaler la série ou la deadline lors d'un report.
- Une fréquence hebdomadaire à plusieurs jours suit un seul calendrier : pour lundi et vendredi, cocher lundi prépare vendredi ; cocher vendredi prépare lundi. Une seule occurrence reste à faire, même si plusieurs jours ont été manqués.
- Une répétition mensuelle suit le jour demandé à l'origine ; une répétition en jours suit un nombre de jours calendaires, week-ends compris.
- Le cumul des filtres, du classement et des métadonnées ne duplique jamais une occurrence. Deux tâches créées séparément avec un même titre restent deux tâches distinctes : aucune fusion par titre n'est demandée.

## Parcours implémentés

- L'édition intégrée donne accès à « Répéter… » puis au formulaire. Le formulaire propose les quatre familles de fréquences validées ; chaque semaine permet plusieurs jours.
- Le formulaire affiche les trois prochaines dates théoriques et, s'il y a une deadline, la deadline correspondante. Cet aperçu ne représente pas trois tâches créées.
- L'enregistrement de la tâche et de sa règle se fait ensemble. Annuler conserve l'état précédent ; une erreur conserve la saisie et ne crée pas de tâche partielle.
- Les dates sont des dates locales sans heure, comme dans la SFG. Les jours comprennent les week-ends ; aucune notification n'est ajoutée.
- Les tags, la recherche, le classement et l'ordre manuel restent ceux des tâches ordinaires. La récurrence ne crée pas de section supplémentaire.
- La série s'applique au titre et aux tags, mais la modification de la date prévue seule désigne un report ponctuel. Le formulaire doit distinguer ces intentions sans demander de choix de portée à chaque édition.

## Syntaxe des commandes validée

La syntaxe étend les commandes explicites de F2, sans interprétation du langage naturel.

| Saisie | Répétition |
| --- | --- |
| `/r jour` | Tous les jours |
| `/r vendredi` | Chaque vendredi |
| `/r lundi,vendredi` | Chaque lundi et vendredi |
| `/r 30j` | Tous les 30 jours |
| `/r mois:31` | Chaque mois le 31, ou le dernier jour du mois |

Exemple cumulant les métadonnées : `Faire ma revue /p 2026-11-06 /d 2026-11-09 /r vendredi #perso`.

Comportement implémenté :

- À la création, `/p` fixe la première date. Sans `/p`, RG-F6-15 fournit la prochaine date compatible. Sans `/d`, aucune deadline n'est ajoutée.
- Dans l'édition, une règle inchangée conserve son calendrier ; modifier seulement `/p` reporte l'occurrence. Modifier `/r` ouvre l'aperçu de nouvelle fréquence et de première date avant validation ; la date déjà affichée peut être ajustée si elle n'est pas compatible.
- Retirer `/r` produit « Ne plus répéter ». Retirer `/d` retire la deadline de l'occurrence à faire et des suivantes. Une occurrence récurrente garde une date prévue : retirer `/p` seul propose le formulaire pour choisir une date, sans effacer silencieusement le calendrier.
- Les jours sont écrits en français ; les jours multiples sont séparés par des virgules sans espace ; l'intervalle est un entier strictement positif ; le jour mensuel est entre 1 et 31. Une valeur absente, invalide ou plusieurs commandes `/r` empêchent l'enregistrement et conservent le texte.
- Comme `/p` et `/d`, `/r` peut être échappé avec un antislash pour rester dans le titre. L'édition réaffiche les dates absolues et échappe les tokens littéraux pour ne pas transformer un titre existant en commande.
- L'aide F1 présente la répétition et l'accès au formulaire. Le formulaire s'utilise par Tab/Maj+Tab, Entrée et Échap, sans nouveau raccourci global.

## Parcours illustratifs validés

### Réaliser en retard une tâche tous les 30 jours

1. Première date prévue : 1er novembre 2026 ; intervalle : 30 jours.
2. L'utilisateur coche la tâche le 5 novembre.
3. L'occurrence du 1er novembre reste barrée le 5 et rejoint l'historique.
4. Une seule occurrence à faire est prévue au 1er décembre et apparaît immédiatement.
5. Le calendrier se poursuit le 31 décembre, indépendamment du jour du cochage.

### Réaliser après plusieurs vendredis manqués

1. Une occurrence prévue le vendredi 6 novembre 2026 reste à faire.
2. L'utilisateur la coche le lundi 23 novembre.
3. La prochaine occurrence est prévue le vendredi 27 novembre et apparaît immédiatement.
4. Les 13 et 20 novembre ne donnent pas de tâches à rattraper.

## Scénarios de recette

| Cas | Résultat attendu |
| --- | --- |
| Première date au 1er novembre, intervalle 30 jours, cochage le 5 | Prochaine occurrence le 1er décembre, puis le 31 décembre |
| Vendredi 6 novembre non réalisé, cochage le lundi 23 | Une seule occurrence à faire, prévue le vendredi 27 |
| Vendredi 6 novembre coché le jeudi 5 | Prochaine occurrence vendredi 13 ; réalisation datée du jeudi 5 |
| Mensuel au 31 janvier 2027 | Dates suivantes : 28 février puis 31 mars |
| Mensuel au 31 janvier 2028 | Dates suivantes : 29 février puis 31 mars |
| Tous les lundis et vendredis, lundi coché | Prochaine occurrence vendredi, une seule à faire |
| Vendredi reporté au lundi, deadline lundi | Deadline inchangée ; calendrier des vendredis conservé |
| Date prévue vendredi, deadline lundi ; occurrence terminée | Prochain vendredi avec deadline le lundi suivant |
| Date prévue, deadline dépassée, plusieurs tags, répétition | Une seule ligne pour l'occurrence, classée en retard |
| Titre ou tags modifiés | Occurrence à faire et suivantes mis à jour |
| Fréquence ou règle de deadline modifiée | Aperçu puis mise à jour de l'occurrence à faire et des suivantes, sans ajout d'une seconde occurrence à faire |
| Ne plus répéter, puis cocher | Tâche conservée puis terminée, aucune occurrence suivante |
| Décocher la dernière occurrence terminée du jour | Occurrence rouverte, suivante à faire retirée |
| Deux occurrences successives déjà cochées aujourd'hui | L'ancienne n'est plus réouvrable ; la dernière reste réouvrable selon RG-F6-14 |
| Échec d'enregistrement du cochage | Ni complétion seule ni prochaine occurrence seule ; retour à l'état confirmé |
| Redémarrer ou recharger plusieurs fois le même état | Même occurrence en attente ; aucune génération répétée |
| Commande invalide ou annulation du formulaire | Texte conservé en cas d'erreur, état précédent conservé en cas d'annulation |

Ces scénarios sont couverts par les parcours persistés de [tests/f6_documentation.rs](../../tests/f6_documentation.rs), les tests de contrat [tasks_tests.rs](../../src-tauri/src/tasks_tests.rs) et les interactions DOM [App.test.ts](../../frontend/tests/App.test.ts).

Les tests utilisent des dates fixes, de vrais fichiers temporaires pour les parcours Rust et un adaptateur IPC contrôlé pour Vue. Ils ne remplacent pas une recette graphique native Windows, X11 et Wayland (navigation Tab/Maj+Tab, focus, rendu et défilement).

## Décisions d’implémentation et validation restante

- **Annulation du cochage :** avertissement sur la suppression de la suivante et de ses éventuelles modifications ; possibilité d’annuler. Ce comportement a été validé avant développement.
- **Cochage externe :** décision explicitement validée pendant l’implémentation : charger l’état sans réécrire `tasks.md`, puis générer la suivante à la prochaine sauvegarde déclenchée dans l’application. Aucune génération au seul rechargement ou redémarrage. Une complétion externe sans date conserve le compromis F3 sur la date d’observation.
- **Stockage :** calendrier, date théorique, ancrage et écart de deadline dans `@repeat`, lien de série et génération dans `@occurrence`. Voir [architecture §11.1](../ARCHITECTURE.md#111-récurrence-f6). Les anciennes tâches restent lisibles. Le cochage et la génération partagent le commit atomique existant ; aucune migration spontanée du fichier.
- **Présentation :** formulaire et confirmations implémentés suivant la recette validée ; les SVG demeurent des explorations. La recette graphique native sur les plateformes cibles reste à réaliser.

## Maquettes exploratoires

- [Saisie rapide et liste](../designs/taches-recurrentes/proposition.svg).
- [Formulaire hebdomadaire et intervalle en jours](../designs/taches-recurrentes/formulaire.svg).
- [Les quatre familles de fréquences](../designs/taches-recurrentes/frequences.svg).
- [Cumul des métadonnées et résultat après cochage](../designs/taches-recurrentes/cycle.svg).
- [Suivi du cadrage](../designs/taches-recurrentes/README.md).

Ces maquettes sont des propositions visuelles, sans persistance. Elles ne valent pas validation des choix encore ouverts.
