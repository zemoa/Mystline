# F5-consolider-interface — Consolider l'interface

> Migration Tauri/Vue : ces règles restent la référence fonctionnelle. Le portage F0–F5 et sa validation sont suivis dans [la migration](../migration-tauri-vue.md).

## Objectif et périmètre

Donner à toutes les surfaces de Mystline une identité visuelle commune, compacte et conforme à la direction **Brume, palette Indigo**, choisie le 14 octobre 2026 et représentée dans la [maquette](../mockup.png), sans modifier les règles métier ni les parcours établis par F0 à F3. F5 couvre la capture, le panneau quotidien, l'édition, la recherche, l'historique, les états d'aide et d'erreur ainsi que les paramètres existants.

L'éditeur reste fondé sur la syntaxe explicite `/p`, `/d` et `#tag`. Les trois filtres et l'édition structurée de la [maquette initiale](../designs/mockup-initiale.png), la suppression, le glisser-déposer et le thème sombre restent hors périmètre.

## Règles de présentation

- **RG-F5-01 — Design system partagé.** Toutes les surfaces utilisent la palette claire Brume Indigo, les mêmes espacements, rayons, bordures et styles de contrôle. La typographie est sans sérif ; les tags partagent un traitement indigo discret. Les icônes sont des SVG symboliques embarqués et ne dépendent pas des polices installées. Le panneau présente la marque Mystline au-dessus du titre de la vue ; le quotidien et l'historique affichent aussi la date locale fournie par le runtime.
- **RG-F5-02 — Surfaces compactes.** La capture et le panneau ont des dimensions fixes. Les paramètres n'ont pas de barre de titre système et sont assez hauts pour afficher le formulaire complet sur un écran courant, tout en conservant un défilement de secours sur les petits écrans. Les listes et contenus exceptionnellement longs défilent sans dépasser l'écran ; la capture se redimensionne en place vers le bas uniquement pour l'aide ou une erreur.
- **RG-F5-03 — Hiérarchie du panneau.** Le panneau conserve les quatre sections exclusives de F2. Les en-têtes sont compacts, en capitales, soulignés et accompagnés d'un repère vertical et d'un compteur. La sélection clavier associe un fond indigo pâle et un trait indigo. Le retard utilise un rouge distinct ; une échéance du jour, un filtre, une tâche terminée et un état vide possèdent chacun un traitement visuel distinct.
- **RG-F5-04 — Actions explicites.** Recherche, historique, paramètres, retour, fermeture et enregistrement disposent de boutons visibles. Ces boutons complètent les raccourcis existants sans les remplacer et conservent un focus clavier identifiable.
- **RG-F5-05 — États transitoires.** Aide, erreur, recherche, filtre et brouillon restent lisibles dans la surface concernée. Fermer le panneau abandonne ces états non enregistrés ; sa prochaine ouverture présente la vue quotidienne.
- **RG-F5-06 — Comportement préservé.** Le format `tasks.md`, les commandes applicatives, les règles de classement, l'historique, la complétion et la persistance ne changent pas.

## Palette Brume Indigo

| Usage | Couleur |
| --- | --- |
| Fond des surfaces | `#F5F7FB` |
| Texte principal | `#293247` |
| Texte secondaire | `#68738A` |
| Bordures et séparateurs | `#DEE3EE` |
| Actions, focus et sélection | `#4E54A5` |
| Survol des actions principales | `#3E438A` |
| Fond de sélection et filtre | `#E7EAF9` |
| Tags, aide et édition | `#ECEFF7` |
| Retard et erreurs | `#B24450` |
| Fond d'erreur | `#F9E8ED` |

Les champs et les contrôles secondaires sont blancs ; les boutons principaux utilisent un texte blanc sur indigo. Les confirmations et badges actifs conservent un vert discret (`#386552` sur `#E7F1EB`). Le [SVG de référence](../designs/brume-indigo.svg) accompagne l'export PNG.

## Logo Mystline

La proposition **02, « Lignes et coche »**, a été choisie le 14 octobre 2026. Trois lignes indigo représentent les tâches ; la dernière se prolonge en coche. Le panneau utilise ce symbole à côté du nom Mystline, sans cartouche. Le favicon, les icônes de distribution et le tray Linux et Windows reprennent le même dessin. La [source SVG et ses déclinaisons](../designs/logos/propositions.md) remplacent le monogramme « M » provisoire ; la maquette de référence a été actualisée.

## Vérifications fonctionnelles

- La capture reste focalisée à l'ouverture et conserve son texte lorsqu'une validation échoue ; son aide et son erreur ne débordent pas de la fenêtre.
- Les titres longs, plusieurs tags, les dates et les deadlines restent lisibles dans une ligne sélectionnée ou terminée.
- Les boutons du panneau sont activables à la souris et par Tab puis Entrée ; les raccourcis documentés dans F2 et F3 restent opérationnels.
- Les actions de sélection ou de création du fichier de tâches confirment leur succès ou affichent leur erreur dans les paramètres.
- Une fermeture suivie d'une réouverture retire recherche, filtre et brouillon, puis affiche la vue quotidienne.
- Les vérifications automatisées sont complétées par une comparaison avec la maquette sous Windows, X11 et Wayland.

## Frontière avec les autres features

F5 ne possède aucune règle métier. F0 reste responsable des surfaces desktop et des paramètres, F1 de la capture et de la consultation, F2 de l'organisation et de l'édition, F3 de la complétion et de l'historique, et F4 de la configuration des raccourcis.
