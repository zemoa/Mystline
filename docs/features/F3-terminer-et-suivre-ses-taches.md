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
- **RG-F3-05 — Historique secondaire.** Une vue secondaire accessible depuis le panneau permet de consulter toutes les tâches terminées, y compris celles d'aujourd'hui. Elle les trie par date de fin effective décroissante, en conservant l'ordre du fichier à date égale. La recherche dans le titre et les tags et le filtre par tag sont disponibles ; leur valeur est conservée lors du changement de vue. Elle n'ajoute pas de section persistée au fichier de tâches et ne remplace pas le panneau quotidien. Les tâches terminées aujourd'hui restent visibles dans le panneau, sans duplication entre ses sections. L'historique est consultatif : aucune édition, réouverture ou permutation d'une tâche terminée n'est proposée.
- **RG-F3-06 — Édition extérieure datée.** Une ligne cochée dans `tasks.md` avec `@completed(YYYY-MM-DD)` utilise cette date pour décider de sa visibilité dans le panneau et dans l'historique. Une ligne datée d'un jour antérieur n'est pas présentée comme accomplie aujourd'hui.
- **RG-F3-07 — Édition extérieure sans date.** Si une ligne devient cochée sans `@completed`, l'application la considère comme venant d'être cochée le jour où elle l'observe. Elle reste visible barrée pendant ce jour. L'application ne réécrit pas spontanément l'édition extérieure ; elle ajoute `@completed` avec la date d'observation lors de sa prochaine écriture du fichier. Si elle redémarre avant cette écriture, l'absence de date oblige à considérer le cochage comme venant d'être observé à nouveau au démarrage : le jour visible est alors celui de cette nouvelle observation, conformément à l'exception explicitée dans la SFG.
- **RG-F3-08 — Clavier.** La sélection et le parcours des tâches sont ceux de F1 ; `Espace` termine la tâche sélectionnée lorsque le focus est dans la liste. Dans un champ de capture, de recherche ou d'édition, cette touche conserve son rôle de saisie. `Ctrl+H` ou Tab/Maj+Tab puis Entrée sur « Historique » ouvre la vue secondaire ; les flèches la parcourent. `Ctrl+H` ou le bouton « Retour » revient au panneau quotidien. Échap efface d'abord une recherche active, puis quitte l'historique avant de fermer le panneau. Une nouvelle ouverture du panneau présente la vue quotidienne.

## Vérifications fonctionnelles

- Cocher une tâche dans **Aujourd'hui** la laisse barrée dans **Aujourd'hui** jusqu'à la fin de journée ; elle n'est plus dans le panneau le lendemain, mais figure dans l'historique après un redémarrage.
- Cocher une tâche en retard sans date prévue du jour l'enlève immédiatement de **En retard** et la rend visible barrée dans **Toutes les tâches** jusqu'au lendemain.
- Cocher extérieurement une ligne sans date ne provoque aucune écriture immédiate ; la prochaine modification enregistrée par l'application fixe sa date de fin. Un redémarrage préalable réinitialise la date d'observation selon RG-F3-07.

## Frontière avec les autres features

F3 ne définit ni la création (F1), ni les commandes `/p`, `/d`, les tags, la recherche, la classification des tâches à faire ou leur réordonnancement (F2). Elle consomme la persistance et la détection de changements fournies par F0, sans créer un second stockage d'historique. F4 n'intervient que sur les raccourcis globaux de capture et de panneau.

## Tests comme documentation

F3 est implémentée. Les scénarios emploient des noms français, les références `RG-F3-xx` et les commentaires **Étant donné / Quand / Alors**. Les parcours persistés utilisent des fichiers temporaires et des dates fixes : vendredi **2026-10-09**, samedi **2026-10-10**, prochain jour ouvré **2026-10-12**. Aucun test n'attend minuit ni ne dépend de la date courante pour ces scénarios.

Les [spécifications exécutables F3](../../tests/f3_documentation.rs) vérifient le résultat dans le panneau, l'historique et le fichier après réouverture. Les [tests Iced](../../src/ui.rs) exercent les vrais widgets sans ouvrir de fenêtre, ainsi que les transitions utilisées par les messages du panneau. Les [tests du snapshot UI](../../src/ui/task_store.rs) vérifient l'affichage avant écriture, la resynchronisation après échec et le rejet des commandes ou résultats provenant d'un ancien fichier source.

| Règles | Scénarios exécutables | Source |
| --- | --- | --- |
| 01–02 | `terminer_une_tache_conserve_ses_metadonnees_et_persiste_sa_date`, `une_tache_terminee_ne_peut_pas_etre_reouverte` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 02 | `un_echec_de_completion_preserve_letat_confirme_et_signale_lerreur`, `un_conflit_de_completion_preserve_ledition_exterieure` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 02 | `les_quatre_actions_saffichent_avant_le_commit_puis_sont_confirmees`, `un_echec_de_sauvegarde_annule_la_completion_optimiste`, `un_conflit_remplace_la_proposition_par_la_version_exterieure`, `le_worker_bloque_sur_le_disque_laisse_lexecuteur_ui_progresser` | [Snapshot UI et sauvegarde](../../src/ui/task_store.rs) |
| 01–02 | `une_ancienne_commande_ne_coche_pas_le_nouveau_fichier_a_revision_egale`, `une_reponse_de_lancienne_source_ne_remplace_pas_la_nouvelle_operation` | [Changement de source](../../src/ui/task_store.rs) |
| 02–03–08 | `cliquer_la_case_termine_et_reclasse_la_tache_en_conservant_la_selection`, `une_completion_refusee_restaure_le_snapshot_et_un_brouillon_interdit_le_cochage` | [Widgets et transitions Iced](../../src/ui.rs) |
| 03 | `une_tache_terminee_aujourdhui_reste_dans_une_seule_section`, `terminer_une_tache_en_retard_la_reclasse_sans_signaler_sa_deadline` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 03 / F2 | `le_reordonnancement_des_taches_actives_ignore_les_terminees_du_jour` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 04–05 | `le_lendemain_retire_la_tache_du_panneau_et_la_conserve_dans_lhistorique` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 04 | `le_message_de_changement_de_jour_retire_la_terminee_et_repare_la_selection` | [Transitions Iced](../../src/ui.rs) |
| 05 | `lhistorique_inclut_aujourdhui_et_se_filtre_du_plus_recent_au_plus_ancien` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 06 | `une_completion_exterieure_datee_utilise_la_date_du_fichier` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 07 | `une_completion_sans_date_attend_la_prochaine_ecriture_pour_etre_datee`, `un_redemarrage_avant_ecriture_renouvelle_la_date_dobservation` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 07 | `un_rechargement_conserve_la_date_observee_apres_deplacement_de_la_ligne`, `une_correspondance_ambigue_ou_modifiee_est_observee_a_nouveau` | [Parcours persistés](../../tests/f3_documentation.rs) |
| 07 | `un_grand_fichier_reordonne_conserve_les_dates_sans_recriture` (10 000 tâches sans date) | [Rechargement du repository](../../src/repository/mod.rs) |
| 05–08 | `lhistorique_se_consulte_sans_modifier_les_taches`, `tab_et_entree_ouvrent_lhistorique_puis_les_fleches_et_echap_le_parcourent` | [Parcours persistés](../../tests/f3_documentation.rs), [Iced](../../src/ui.rs) |
| 01–08 | `espace_termine_la_selection_et_une_activation_capturee_nest_pas_rejouee`, `espace_dans_les_champs_saisit_du_texte_sans_cocher_une_tache` | [Widgets et événements Iced](../../src/ui.rs) |

Exécuter les parcours : `cargo test --test f3_documentation -- --nocapture`. Exécuter les interactions Iced : `cargo test --bin mystline ui::tests:: -- --nocapture`. Exécuter les scénarios du snapshot UI : `cargo test --bin mystline ui::task_store::tests:: -- --nocapture`. Ces commandes affichent les noms des scénarios pour faciliter leur lecture comme documentation.

Vérifications graphiques complémentaires sur Windows, X11 et Wayland : case et titre barré, deadline terminée sans alerte de retard, focus visible du bouton d'historique, titres longs et défilement, passage de jour avec le panneau ouvert. Les tests sans fenêtre ne remplacent pas ces vérifications.
