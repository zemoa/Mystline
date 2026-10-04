# F2-organiser-et-retrouver-ses-taches — Organiser et retrouver ses tâches

## Objectif et périmètre

Permettre d'ajouter facultativement du contexte aux tâches, de préparer le prochain jour ouvré et de retrouver immédiatement la tâche recherchée. Cette feature possède la syntaxe des métadonnées, l'édition, les quatre sections du panneau, l'ordre manuel, les tags, la recherche, les filtres et l'aide à la saisie. La création avec un simple titre et l'ouverture du panneau restent en F1 ; la complétion et l'historique relèvent de F3.

Références : [SFG, §2–3, §5, §7–13 et §16–22](../SFG.md) ; [architecture, §6–9 et §30](../ARCHITECTURE.md).

## Parcours couverts

1. Dans la capture de F1 ou l'édition, saisir `Préparer le support /p jeudi /d vendredi #mission` : la tâche conserve le titre « Préparer le support » et reçoit une date prévue, une deadline et un tag.
2. Depuis le panneau, sélectionner une tâche de fond, l'éditer pour la planifier au prochain jour ouvré et la retrouver dans la section correspondante ; cette préparation est facultative.
3. Depuis le panneau, rechercher quelques lettres d'un titre, sélectionner un tag pour filtrer, ou déplacer une tâche dans sa section, principalement au clavier.

## Règles de gestion — saisie et édition

- **RG-F2-01 — Métadonnées facultatives.** La date prévue, la deadline et les tags sont indépendants et facultatifs. Une tâche peut avoir simultanément date prévue et deadline ; aucune heure, sous-tâche, projet, priorité explicite ou état « En cours » n'est créé.
- **RG-F2-02 — Syntaxe explicite.** Dans la capture et l'édition, `/p <date>` définit la date prévue, `/d <date>` définit la deadline et `#tag` ajoute un tag. Ces tokens commencent un mot et `/p` et `/d` sont des mots entiers. Les commandes reconnues sont retirées du texte enregistré comme titre ; le reste du texte demeure le titre. Les espaces internes entre mots consécutifs du titre sont conservés ; une commande retirée entre deux fragments du titre est remplacée par un espace. Aucun autre fragment de texte n'est interprété comme une date par langage naturel ; une commande inconnue reste dans le titre. Pour inclure littéralement un token de commande dans le titre, l'utilisateur le préfixe d'un antislash : `\#mission` conserve `#mission` et `\/p vendredi` conserve `/p vendredi` dans le titre. Un antislash littéral s'écrit `\\` ; les antislashs devant d'autres caractères restent littéraux. L'édition réaffiche ces caractères échappés pour ne pas transformer un titre existant en métadonnée.
- **RG-F2-03 — Dates admises.** Les valeurs de date acceptées sont une date calendaire `YYYY-MM-DD`, `aujourd'hui`, `demain` ou un jour de semaine écrit en français (`lundi` à `dimanche`). `demain` est le lendemain calendaire, week-end compris. Un jour de semaine désigne sa prochaine occurrence, aujourd'hui compris. La conversion se fait selon la date locale au moment de la validation ; une date impossible est rejetée.
- **RG-F2-04 — Validation des commandes.** Si `/p` ou `/d` est présent sans valeur valide, apparaît plusieurs fois dans une même saisie, ou si les commandes consomment tout le titre, Entrée ne crée ni ne modifie la tâche. La saisie reste ouverte, intacte, avec une erreur compréhensible et sans écriture dans le fichier. Une édition refusée conserve la tâche antérieure. L'aide et l'erreur n'imposent pas l'usage de la souris.
- **RG-F2-05 — Modification directe.** Depuis le panneau, sélectionner une tâche puis utiliser Entrée pour éditer dans une zone légère. L'édition présente son titre et ses métadonnées actuelles sous forme de commandes explicites ; les dates existantes sont présentées en `YYYY-MM-DD` pour éviter qu'une réouverture ne les décale. Modifier ou retirer `/p`, `/d` ou un `#tag` puis valider met à jour ces valeurs. Échap annule sans modifier la tâche. Le titre reste obligatoire.
- **RG-F2-06 — Tags.** Une tâche accepte plusieurs tags, saisis sous la forme `#tag` ; un nom de tag est une suite de lettres ou chiffres, avec éventuellement `-` ou `_`, sans espace. Un token commençant par `#` sans nom valide est refusé ; on peut l'échapper pour l'inclure dans le titre. Les tags sont visibles discrètement dans la liste. Deux tags ne différant que par la casse désignent le même tag et ne sont pas ajoutés deux fois à la même tâche ; la première graphie est conservée, avec comparaison Unicode en minuscules. Cette règle s'applique aussi au snapshot chargé depuis un fichier externe, sans réécriture spontanée. Ils servent uniquement de contexte et de filtre. Aucun objet « projet » ou « tâche de fond » n'est créé : `#fond` reste un tag ordinaire.
- **RG-F2-07 — Aide contextuelle.** Depuis la saisie de capture ou d'édition, `F1` ouvre ou ferme une aide compacte présentant au minimum `/p`, `/d`, `#tag` et l'antislash d'échappement. Échap ferme d'abord l'aide, sans effacer la saisie. L'aide est intégralement utilisable au clavier.

## Règles de gestion — affichage et actions sur le panneau

- **RG-F2-08 — Sections exclusives.** Pour une tâche « À faire », le panneau applique dans cet ordre le premier critère satisfait : (1) deadline antérieure à aujourd'hui → **En retard** ; (2) date prévue aujourd'hui → **Aujourd'hui** ; (3) date prévue le prochain jour compris entre lundi et vendredi → **Prochain jour ouvré** ; (4) sinon → **Toutes les tâches**. Les jours fériés ne modifient pas ce calcul. Une tâche n'apparaît jamais deux fois.
- **RG-F2-09 — Dates et continuité.** Les dates ne contiennent pas d'heure. Le prochain jour ouvré est le prochain lundi-vendredi strictement postérieur à aujourd'hui : vendredi → lundi. Une tâche prévue hier et non faite reste visible, normalement dans **Toutes les tâches**, sauf si sa deadline la place en **En retard**. Rien n'est automatiquement reporté ou supprimé. Le classement est recalculé lorsque la date locale change, y compris si le panneau reste ouvert.
- **RG-F2-10 — Lisibilité des échéances.** **En retard** est visuellement identifiable. Une deadline future éloignée est indiquée discrètement ; une échéance du jour ou proche peut ressortir davantage, sans créer de notification ou de priorité indépendante de l'ordre des tâches. Une tâche avec deadline future mais sans date prévue reste dans **Toutes les tâches**.
- **RG-F2-11 — Recherche.** Une recherche accessible au clavier depuis le panneau, notamment par `Ctrl+F`, filtre instantanément la vue lorsque l'utilisateur tape quelques lettres. Elle recherche sans tenir compte de la casse dans les titres et les tags, sans modifier la tâche ni sa section calculée.
- **RG-F2-12 — Filtre par tag.** Cliquer un tag ou le sélectionner au clavier puis valider limite la vue aux tâches qui le portent, sans tenir compte de la casse du tag. Le filtre peut être retiré sans altérer les tâches ; il n'impose pas de requêtes combinées complexes. Si recherche et filtre sont actifs, seuls leurs résultats communs sont affichés, en conservant les sections de RG-F2-08.
- **RG-F2-13 — Ordre manuel.** Dans une section, `Ctrl+↑` et `Ctrl+↓` déplacent la tâche sélectionnée respectivement vers la position visible précédente ou suivante de cette section. Le geste permute les deux positions visibles : pour `A, masquée, B`, déplacer B vers le haut donne `B, masquée, A`. Les tâches masquées conservent donc leurs positions et leur ordre entre elles. Aux bornes d'une section, le geste n'a aucun effet et n'écrit pas dans le fichier. L'ordre résultant est conservé dans le fichier de tâches, sans champ de priorité Haute/Moyenne/Basse. Un changement de date ou de deadline change éventuellement de section ; le classement et l'ordre sont recalculés après la modification.
- **RG-F2-14 — Navigation clavier.** Flèches et sélection du panneau (F1), Entrée pour éditer, `Ctrl+F` pour rechercher, Tab ou Maj+Tab puis Entrée sur un tag pour filtrer et `Ctrl+↑`/`Ctrl+↓` pour ordonner permettent le parcours principal sans souris. Le bouton « Retirer le filtre » est accessible par le même parcours. Les flèches dans un champ de texte ne déplacent pas les tâches ; Entrée dans la recherche rend la main à la sélection en conservant la requête. Échap ferme d'abord une aide, une édition ou une recherche active avant de masquer le panneau. Fermer la recherche par Échap efface sa requête ; le filtre par tag reste indépendant. Les commandes de complétion sont définies en F3.

## Vérifications fonctionnelles

- Saisir `/p vendredi` un vendredi prévoit la tâche ce vendredi ; un jeudi, le vendredi suivant. `/p demain` suit le calendrier même le vendredi. Une date impossible ou `/d` sans valeur laisse la saisie ouverte avec son texte.
- Le vendredi, une tâche prévue lundi apparaît dans **Prochain jour ouvré** ; une tâche prévue aujourd'hui dont la deadline était hier apparaît uniquement dans **En retard** ; une tâche prévue hier sans deadline reste consultable dans **Toutes les tâches**.
- Une édition retire une date ou un tag en supprimant sa commande affichée ; Échap ne change rien. La recherche par titre ou tag, le filtre et le déplacement au clavier restent utilisables sur les sections visibles.

## Frontière avec les autres features

F2 classe et ordonne les tâches actives. F3 décide de la visibilité et de la section des tâches cochées le jour même, puis de leur présence dans l'historique. F0 gère l'écriture, la reprise des éditions extérieures et les erreurs de concurrence ; F1 gère l'ouverture des surfaces et la capture minimale ; F4 ne concerne que les raccourcis globaux configurables.

## Tests comme documentation

F2 est implémentée. Les scénarios ci-dessous sont exécutables et portent les commentaires **Étant donné / Quand / Alors** dans le code. Les dates sont fixes : vendredi **2026-10-09**, samedi **2026-10-10**, prochain jour ouvré **2026-10-12**. Aucun test n'attend réellement minuit et les tests de persistance utilisent des fichiers temporaires.

| Règles | Scénarios exécutables | Source |
| --- | --- | --- |
| 01–02 | `un_titre_suffit_a_creer_une_tache`, `une_capture_avec_metadonnees_est_persistee_et_retrouvee` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 01–02 | `les_metadonnees_sont_independantes`, `seules_les_commandes_explicites_sont_interpretees` | [Parseur](../../src/presentation/input.rs) |
| 02–05 | `les_tokens_echappes_restent_litteraux`, `une_edition_sans_changement_preserve_les_tokens_et_antislashs` | [Parseur](../../src/presentation/input.rs) |
| 02–05 | `une_edition_sans_changement_preserve_la_tache_apres_reouverture` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 03 | `demain_est_samedi_meme_si_le_prochain_jour_ouvre_est_lundi`, `les_dates_francaises_suivent_le_calendrier`, `les_dates_calendaires_sont_validees` | [Parseur](../../src/presentation/input.rs) |
| 04 | `les_commandes_vides_repetees_ou_sans_titre_sont_refusees` | [Parseur](../../src/presentation/input.rs) |
| 04 | `une_capture_invalide_conserve_sa_fenetre_son_texte_et_le_fichier` | [Transitions UI](../../src/ui.rs) |
| 04–05 | `une_edition_invalide_conserve_la_saisie_et_la_tache_anterieure` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 05 | `retirer_une_commande_retire_sa_metadonnee`, `echap_annule_ledition_sans_ecrire`, `planifier_une_tache_de_fond_prepare_le_prochain_jour_ouvre` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 06 | `les_tags_sont_valides_et_uniques_sans_casse` | [Parseur](../../src/presentation/input.rs) |
| 06 / F0 | `les_tags_exterieurs_de_casses_differentes_designent_le_meme_tag` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 07–14 | `echap_ferme_laide_puis_ledition_puis_la_recherche` | [Panneau](../../src/presentation/panel.rs) |
| 08–09 | `les_sections_sont_exclusives_et_ordonnees`, `le_prochain_jour_ouvre_ignore_le_weekend` | [Panneau](../../src/presentation/panel.rs) |
| 09 | `le_changement_de_jour_reclasse_sans_modifier_les_taches`, `le_minuteur_vise_le_prochain_minuit_local` | [Panneau](../../src/presentation/panel.rs) |
| 10 | `une_deadline_future_ne_planifie_pas_la_tache` | [Panneau](../../src/presentation/panel.rs) |
| 11–12 | `recherche_et_filtre_par_tag_se_croisent_sans_changer_les_sections` | [Panneau](../../src/presentation/panel.rs) |
| 09–12 | `recherche_filtre_et_changement_de_jour_necrivent_pas` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 12–14 | `tab_puis_entree_active_un_tag_et_permet_de_retirer_le_filtre` | [Widgets Iced sans fenêtre](../../src/ui.rs) |
| 13 | `le_reordonnancement_respecte_les_sections_et_les_bornes` | [Panneau](../../src/presentation/panel.rs) |
| 13 | `le_reordonnancement_filtre_permute_les_positions_visibles` | [Parcours persistés](../../tests/f2_documentation.rs) |
| 13–14 | `les_actions_visent_la_selection_visible_apres_filtrage_et_permutation` | [Transitions UI](../../src/ui.rs) |
| 14 | `une_touche_dedition_capturee_ne_declenche_pas_ledition_du_panneau`, `les_fleches_dans_un_champ_texte_ne_deplacent_pas_la_selection`, `effacer_la_recherche_conserve_la_selection_avant_edition` | [Widgets et événements Iced](../../src/ui.rs) |
| F0 / F2 | `un_conflit_necrase_pas_une_edition_exterieure`, `une_revision_obsolete_refuse_edition_et_permutation`, `un_conflit_de_permutation_preserve_lordre_exterieur` | [Parcours persistés](../../tests/f2_documentation.rs) |
| F0 / F2 | `un_echec_decriture_ne_confirme_pas_la_modification`, `un_changement_de_source_invalide_ledition_meme_a_revision_egale` | [Parcours persistés](../../tests/f2_documentation.rs) |

Exécuter les parcours persistés : `cargo test --test f2_documentation`. Exécuter les règles du parseur et des projections : `cargo test --lib presentation::`. Exécuter les interactions Iced sans fenêtre : `cargo test --bin mystline ui::tests::`. Afficher le nom de chaque scénario : ajouter `-- --nocapture`.

Vérifications manuelles complémentaires sur Windows, X11 et Wayland : retour du focus à l'activité précédente après capture ; ouverture/redimensionnement de l'aide et affichage des erreurs ; navigation et focus visibles sur les tags ; défilement avec titres longs ; panneau laissé ouvert au changement de jour. Les tests de widgets ne remplacent pas une validation dans ces sessions graphiques.
