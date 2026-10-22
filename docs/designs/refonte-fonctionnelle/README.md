# Refonte fonctionnelle — maquettes validées

Maquettes validées par l'utilisateur le 22 octobre 2026, avant l'implémentation.
Elles servent de référence aux regroupements et parcours décrits dans la SFG.
Leur présentation reprend les repères visuels existants pour concentrer la discussion
sur le fonctionnement.

- [Maquette interactive locale](index.html) : quatre scènes, création, édition,
  complétion, recherche, filtre par tag, historique et simulation du lendemain.
- [Vue principale annotée](01-vue-principale.png) ([source SVG](01-vue-principale.svg)).
- [Capture, replanification et lendemain](02-parcours.png) ([source SVG](02-parcours.svg)).

Ouvrir `index.html` dans un navigateur. Les exemples sont fictifs et restent en mémoire ;
« Réinitialiser » les restaure. Aucun fichier de tâches n'est lu ou modifié. La date
de démonstration est fixée au jeudi 22 octobre 2026.

## Ce que j'ai compris de la demande

Une tâche créée sans date doit être considérée comme une tâche en cours et apparaître
dans la partie utile au quotidien. Elle ne doit plus être reléguée dans une liste
résiduelle « Toutes les tâches ». La saisie d'un titre doit continuer à suffire.

**« Sans date » signifie sans date prévue**. Une deadline indique
une contrainte de fin ; elle ne reporte pas le début du travail. Ainsi, une tâche
sans date prévue avec une deadline dans deux semaines reste en cours dès maintenant.
Cette interprétation fait partie de la maquette validée.

## Organisation validée

Une liste unique conserve une vue d'ensemble, avec quatre sections exclusives :

| Ordre | Section | Tâches non terminées affichées |
| --- | --- | --- |
| 1 | En retard | Deadline strictement antérieure à aujourd'hui, quelle que soit la date prévue. |
| 2 | En cours | Aucune date prévue, date prévue aujourd'hui, ou date prévue passée. |
| 3 | Prochain jour ouvré | Date prévue au prochain jour du lundi au vendredi. Le jour est écrit dans l'en-tête. |
| 4 | Plus tard | Autres dates prévues futures, dont un week-end avant le prochain jour ouvré. La date reste visible sur chaque ligne. |

« En cours » est un **regroupement automatique**. Les deux états existants
restent « À faire » et « Terminée » ; aucune activation manuelle ni troisième état
n'est nécessaire.

Le retard reste prioritaire : une tâche sans date prévue dont la deadline est dépassée
apparaît une seule fois, dans « En retard ». Une deadline aujourd'hui est signalée sur
la ligne sans déplacer la tâche dans « En retard ».

Le maintien des dates prévues passées dans « En cours » fait partie de la validation :
une tâche prévue hier mais inachevée reste à portée de main, sans devoir la replanifier.
Le libellé « Prévu le 20 oct. » conserve l'intention initiale ; aucune date n'est réécrite.

## Parcours représentés

### 1. Consulter et agir

« Appeler le garage », créée avec un titre seul, figure dans « En cours ». La liste
contient aussi une tâche prévue aujourd'hui, une tâche prévue auparavant et une tâche
sans date prévue avec une deadline aujourd'hui. Les dates et deadlines restent distinctes.

Le titre ou « Modifier » ouvre l'édition intégrée. Une case termine la tâche ; elle
reste barrée le jour même et peut être décochée. L'ordre manuel représente toujours
la priorité relative. Recherche et filtre par tag conservent les regroupements.

### 2. Capturer sans organiser

Saisir `Acheter des piles`, puis Entrée : la capture se ferme et la nouvelle tâche
rejoint « En cours », sans date ajoutée automatiquement. La maquette interactive montre
ce résultat dans le panneau. La capture globale réelle rendra la main à l'activité
précédente comme aujourd'hui ; le prototype affiche le panneau pour expliquer le résultat.

La syntaxe `/p`, `/d` et `#tag` reste disponible et n'est pas obligatoire. Une saisie
invalide conserve le brouillon et affiche une erreur.

### 3. Planifier et retirer une date

Dans l'édition intégrée, ajouter `/p vendredi` à « Appeler le garage » et enregistrer :
la tâche quitte « En cours » et rejoint « Prochain jour ouvré · vendredi 23 octobre ».
Elle n'apparaît jamais dans les deux sections.

Retirer `/p ...` puis enregistrer fait l'inverse : la tâche revient dans « En cours ».
La deadline et les tags sont conservés si leurs commandes restent dans la saisie.
Annuler conserve la tâche initiale.

### 4. Passer au lendemain

Le vendredi 23 octobre : les tâches sans date restent en cours ; les tâches prévues
jeudi non terminées y restent aussi ; celles prévues vendredi rejoignent « En cours ».
Une deadline du jeudi devient un retard. Le prochain jour ouvré est lundi 26 octobre ;
une tâche prévue samedi reste dans « Plus tard ». Une tâche terminée jeudi disparaît
du quotidien mais reste consultable dans l'historique.

Une tâche terminée aujourd'hui reste dans sa section de planification calculée sans
la priorité « En retard », comme dans le comportement existant. Les compteurs distinguent
les tâches à faire et celles terminées. Les sections vides restent identifiables ; une
recherche sans correspondance affiche « Aucun résultat ».

## Portée du prototype

Le HTML est une maquette autonome, sans connexion à Tauri ni persistance. Il démontre
les regroupements et les interactions centrales ; il ne remplace pas le parseur de
l'application, ses protections d'écriture ni ses raccourcis globaux. Les paramètres
et les intégrations système ne sont pas remaniés dans cette proposition.

L'ordre manuel est illustré par les boutons Monter/Descendre sur la sélection et
`Ctrl+↑` / `Ctrl+↓`, dans la même section et entre tâches actives visibles. Les raccourcis
locaux ↑/↓, Espace, Entrée, Ctrl+F, Ctrl+H, F1 et Échap permettent de parcourir la maquette.

## Décisions validées avant l'implémentation

1. « En cours » rassemble automatiquement les tâches sans date prévue, celles du jour
   et celles prévues dans le passé, sans nouveau statut à gérer.
2. Une deadline seule ne place pas une tâche dans le futur ; le retard conserve sa priorité.
3. La liste conserve quatre sections, avec « En cours » et « Plus tard » à la place
   d'« Aujourd'hui » et « Toutes les tâches ».

Les règles fonctionnelles sont décrites dans la [SFG](../../SFG.md), F1, F2, F3 et F5,
mis à jour avec l'implémentation. Le HTML reste une démonstration isolée des données réelles.
