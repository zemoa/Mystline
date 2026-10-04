# F1-capturer-et-consulter-ses-taches — Capturer et consulter ses tâches

## Objectif et périmètre

Permettre de noter une tâche avec son seul titre puis de la retrouver immédiatement. La capture et la consultation fonctionnent au clavier, sans ouvrir une fenêtre d'application classique. F0 fournit le processus, la persistance et les raccourcis ; F2 enrichit la saisie avec les métadonnées et introduit les quatre sections métier.

Références : [SFG, §2, §4–6, §9 et §17](../SFG.md) ; [architecture, §9 et §26](../ARCHITECTURE.md).

## Parcours couvert

Raccourci de capture → zone de saisie focalisée → titre → Entrée → retour à l'activité précédente → raccourci du panneau → tâche visible. Aucun classement préalable, aucune date et aucun tag ne sont requis.

## Règles de gestion

- **RG-F1-01 — Ouverture.** Le raccourci global de capture ou l'action correspondante du tray affiche immédiatement une petite fenêtre de saisie, avec le focus dans le champ texte. Une nouvelle capture commence avec une saisie vide.
- **RG-F1-02 — Minimum requis.** Un titre non vide suffit à créer une tâche « À faire ». Dates, deadline, tags et priorité ne sont jamais imposés. Une saisie vide ou constituée uniquement d'espaces n'est pas enregistrée ; elle reste disponible pour correction.
- **RG-F1-03 — Validation.** Avec une saisie valide, Entrée soumet la tâche, la fenêtre disparaît immédiatement et le focus retourne à l'activité précédente. Échap ferme la capture sans création. En cas d'échec de validation ou d'enregistrement, la saisie originale reste récupérable et une erreur est présentée dans la capture ; une tâche non confirmée n'est pas annoncée comme enregistrée.
- **RG-F1-04 — Destination.** La tâche créée est persistée immédiatement par F0 et rejoint directement l'ensemble des tâches consultables dans le panneau, sans Inbox ni étape de classement.
- **RG-F1-05 — Panneau.** Le second raccourci global ou l'entrée « Ouvrir / masquer le panneau » du tray affiche le panneau s'il est masqué et le masque s'il est visible. Un clic principal sur l'icône du tray ouvre le panneau ou lui redonne le focus s'il est déjà ouvert, sans le masquer ni dupliquer sa fenêtre. Échap le ferme lorsqu'aucune interaction interne n'est en cours. Le panneau est compact, rapidement accessible, et affiche le snapshot courant plutôt qu'une copie indépendante des tâches.
- **RG-F1-06 — Lecture au clavier.** À l'ouverture du panneau, la liste peut recevoir le focus ; l'utilisateur peut parcourir les tâches avec les flèches et en sélectionner une sans souris. Les actions sur la sélection (édition, réordonnancement, complétion) sont définies respectivement en F2 et F3.
- **RG-F1-07 — Consultation initiale.** Avant l'ajout des projections de F2, le panneau permet de consulter les tâches actives, y compris celles chargées depuis un fichier existant. F2 détermine leur répartition finale entre les quatre sections ; F3 détermine la visibilité des tâches terminées.

## Vérifications fonctionnelles

- En partant d'une activité extérieure, on peut saisir « Vérifier le dossier d'architecture » et revenir à cette activité après Entrée ; la tâche est présente à la réouverture du panneau et après un redémarrage.
- Appuyer sur Entrée sans titre n'ajoute rien ; Échap n'ajoute rien ; après une erreur d'écriture, le texte n'est pas perdu.
- Le même raccourci ouvre puis masque le panneau ; on peut parcourir les tâches sans souris.

## Frontière avec les autres features

F1 traite exclusivement de la capture d'une tâche par titre et de l'accès à la liste. La syntaxe `/p`, `/d`, `#tag`, la classification, la modification, les filtres et l'ordre manuel relèvent de F2 ; cocher et consulter l'historique relève de F3 ; personnaliser les raccourcis relève de F4.
