# F3-terminer-et-suivre-ses-taches — Terminer et suivre ses tâches

## Objectif et périmètre

Permettre de cocher une tâche depuis le panneau, de voir ce qui a été accompli dans la journée et de consulter ultérieurement les tâches terminées. F3 possède exclusivement le changement d'état, l'affichage des tâches terminées du jour et l'historique ; F2 conserve la responsabilité du classement des tâches à faire, de leur édition et de leur ordre.

Références : [SFG, §2.3, §9–10 et §14–15](../SFG.md) ; [architecture, §6, §10–12 et §28](../ARCHITECTURE.md).

## Parcours couverts

1. Dans le panneau, sélectionner une tâche et la cocher, à la souris ou au clavier : elle apparaît immédiatement barrée, toujours visible aujourd'hui.
2. Ouvrir le panneau le lendemain : la tâche achevée n'apparaît plus dans la vue principale ; elle est accessible dans la vue secondaire d'historique.
3. Modifier directement `tasks.md` pour y cocher une tâche, avec ou sans `@completed(YYYY-MM-DD)`, puis la voir se mettre à jour dans l'application.

## Règles de gestion

- **RG-F3-01 — Deux états seulement.** Une tâche est « À faire » ou « Terminée ». Cocher une tâche « À faire » dans le panneau, notamment avec `Espace` lorsque la tâche est sélectionnée, la fait passer à « Terminée ». Aucun état intermédiaire n'est créé. La réouverture d'une tâche terminée ne fait pas partie du MVP.
- **RG-F3-02 — Date de fin.** Une complétion depuis l'application enregistre la date locale du cochage, sans heure, avec la tâche. La modification est immédiatement persistée par F0 ; si elle échoue, le panneau revient à l'état confirmé et signale l'erreur.
- **RG-F3-03 — Visibilité le jour même.** Une tâche terminée aujourd'hui reste visible, barrée, dans une seule section du panneau. La complétion ne supprime ni titre, ni date prévue, ni deadline, ni tags. Si la date prévue est aujourd'hui, elle apparaît dans **Aujourd'hui** ; si elle correspond au prochain jour ouvré, dans **Prochain jour ouvré** ; sinon dans **Toutes les tâches**. Elle n'apparaît jamais dans **En retard**, même si sa deadline est dépassée, et cette deadline n'est plus signalée comme un retard. Ainsi, une tâche auparavant en retard sans date prévue du jour passe barrée dans **Toutes les tâches**.
- **RG-F3-04 — Passage au lendemain.** Dès que la date locale dépasse la date de fin, la tâche disparaît de la vue principale ; elle reste enregistrée et consultable dans l'historique. Le passage de jour recalcule la vue même si le panneau demeure ouvert.
- **RG-F3-05 — Historique secondaire.** Une vue secondaire accessible depuis le panneau permet de consulter toutes les tâches terminées, y compris celles d'aujourd'hui. Elle n'ajoute pas de section persistée au fichier de tâches et ne remplace pas le panneau quotidien. Les tâches terminées aujourd'hui restent visibles dans le panneau, sans duplication entre ses sections.
- **RG-F3-06 — Édition extérieure datée.** Une ligne cochée dans `tasks.md` avec `@completed(YYYY-MM-DD)` utilise cette date pour décider de sa visibilité dans le panneau et dans l'historique. Une ligne datée d'un jour antérieur n'est pas présentée comme accomplie aujourd'hui.
- **RG-F3-07 — Édition extérieure sans date.** Si une ligne devient cochée sans `@completed`, l'application la considère comme venant d'être cochée le jour où elle l'observe. Elle reste visible barrée pendant ce jour. L'application ne réécrit pas spontanément l'édition extérieure ; elle ajoute `@completed` avec la date d'observation lors de sa prochaine écriture du fichier. Si elle redémarre avant cette écriture, l'absence de date oblige à considérer le cochage comme venant d'être observé à nouveau au démarrage : le jour visible est alors celui de cette nouvelle observation, conformément à l'exception explicitée dans la SFG.
- **RG-F3-08 — Clavier.** La sélection et le parcours des tâches sont ceux de F1 ; la complétion depuis le panneau est accessible sans souris. L'accès à l'historique et son parcours sont également accessibles au clavier.

## Vérifications fonctionnelles

- Cocher une tâche dans **Aujourd'hui** la laisse barrée dans **Aujourd'hui** jusqu'à la fin de journée ; elle n'est plus dans le panneau le lendemain, mais figure dans l'historique après un redémarrage.
- Cocher une tâche en retard sans date prévue du jour l'enlève immédiatement de **En retard** et la rend visible barrée dans **Toutes les tâches** jusqu'au lendemain.
- Cocher extérieurement une ligne sans date ne provoque aucune écriture immédiate ; la prochaine modification enregistrée par l'application fixe sa date de fin. Un redémarrage préalable réinitialise la date d'observation selon RG-F3-07.

## Frontière avec les autres features

F3 ne définit ni la création (F1), ni les commandes `/p`, `/d`, les tags, la recherche, la classification des tâches à faire ou leur réordonnancement (F2). Elle consomme la persistance et la détection de changements fournies par F0, sans créer un second stockage d'historique. F4 n'intervient que sur les raccourcis globaux de capture et de panneau.
