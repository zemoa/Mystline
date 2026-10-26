# Liste sans sections — variante A retenue le 26 octobre 2026

L'utilisateur a retenu **A** et demandé l'ordre : **Retard → aujourd'hui → demain →
pas de date → plus tard**, sans réintroduire de sections.

- **[Maquette A actualisée](a-triee.png)** ([source SVG](a-triee.svg)).

L'utilisateur a également demandé de faire ressortir l'échéance du lendemain.
La maquette utilise un texte ambre foncé (`#865600`) sur un fond ambre clair
(`#FFF0CD`), avec le libellé explicite « Échéance demain · 27 oct. ».
Ce traitement concerne la deadline du lendemain, indépendamment de la date prévue ;
le rouge reste réservé au retard. L'exemple « Préparer le comité » porte cette échéance.

La variante A, le tri et le traitement ambre de l'échéance demain ont été validés
avant la demande de développement. Les règles applicables sont désormais décrites
dans la [SFG](../../SFG.md), F2, F3 et F5 : demain calendaire ; dates prévues passées
avec aujourd'hui ; absence de date prévue au rang pas de date même avec deadline
future ; tâches terminées triées par date prévue en ignorant le retard ; ordre manuel
à rang égal. Aucun autre champ ni statut n'est ajouté.

Le traitement de l'échéance du lendemain utilise un texte ambre foncé (`#865600`)
sur fond ambre clair (`#FFF0CD`). Le rouge reste réservé au retard. L'exemple
« Préparer le comité » porte cette échéance dans la maquette.

Les propositions initiales ci-dessous sont conservées comme historique de conception.
Elles ne définissent plus les règles applicables.

## Propositions initiales conservées

**Maquettes à valider avant développement.** Aucun code applicatif ni règle de référence
n'est modifié par ces propositions. Elles répondent à la demande de suppression des
sections, y compris « En retard », avec des indications suffisantes sur chaque tâche.
Elles remplaceraient, après validation et implémentation, la présentation en quatre
sections actuellement décrite dans la SFG et les maquettes du 22 octobre.

- [Comparer les deux variantes](index.html).
- **A — Métadonnées sous le titre**, recommandée pour le panneau compact :
  [PNG](a-liste.png), [SVG](a-liste.svg).
- **B — Dates alignées**, pour comparer plus facilement les dates :
  [PNG](b-liste.png), [SVG](b-liste.svg).

## Contenu existant représenté

Les deux variantes utilisent exactement les mêmes exemples fictifs, dans le même ordre :
case à cocher, titre, tags, date prévue et deadline quand elles sont renseignées.
Une tâche sans métadonnées reste une ligne avec un titre et une case.
Les tirets de B signifient une absence de date, sans valeur ajoutée à la tâche.

« En retard » est un indicateur calculé depuis une deadline strictement antérieure
au jour courant ; il ne constitue ni une section ni un nouvel état. Une date prévue
passée reste visible sans être présentée comme un retard. Une tâche terminée conserve
ses métadonnées, reste barrée le jour même et ne porte plus l'indicateur de retard,
conformément aux règles existantes.

Recherche, historique, paramètres, fermeture, ajout, modification et déplacements
Monter/Descendre reprennent les actions déjà présentes. Leur représentation est statique.
Le placement des actions de déplacement sous la liste est une proposition visuelle.

## Portée des propositions initiales

La différence entre A et B concerne uniquement la disposition des informations.
Aucun champ, filtre, statut, priorité, tri automatique ou interaction supplémentaire
n'est proposé. L'ordre des exemples est illustratif : il ne définit pas de tri par
date ou par retard. Le détail du réordonnancement à l'échelle de la liste devra être
explicité avant implémentation puisque la règle actuelle s'applique dans une section.

La date de démonstration est le lundi 26 octobre 2026. Les captures, l'édition par
commandes, l'historique et les paramètres ne sont pas redessinés ici. Aucune donnée
réelle n'est consultée ni modifiée par ces fichiers autonomes.
