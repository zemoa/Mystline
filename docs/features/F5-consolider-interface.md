# F5-consolider-interface — Consolider l'interface

## Objectif et périmètre

Donner à toutes les surfaces de Mystline une identité visuelle commune, compacte et proche de la [maquette](../mockup.png), sans modifier les règles métier ni les parcours établis par F0 à F3. F5 couvre la capture, le panneau quotidien, l'édition, la recherche, l'historique, les états d'aide et d'erreur ainsi que les paramètres existants.

L'éditeur reste fondé sur la syntaxe explicite `/p`, `/d` et `#tag`. Les trois filtres illustrés dans la maquette, l'édition structurée, la suppression, le glisser-déposer et le thème sombre restent hors périmètre.

## Règles de présentation

- **RG-F5-01 — Design system partagé.** Toutes les surfaces utilisent la même palette claire, les mêmes espacements, rayons, bordures, ombres et styles de contrôle. Les icônes sont des SVG symboliques embarqués et ne dépendent pas des polices installées.
- **RG-F5-02 — Surfaces compactes.** La capture, le panneau et les paramètres ont des dimensions fixes. Les listes et contenus longs défilent sans agrandir la fenêtre ; la capture s'agrandit uniquement pour l'aide ou une erreur.
- **RG-F5-03 — Hiérarchie du panneau.** Le panneau conserve les quatre sections exclusives de F2. Une sélection clavier, un retard, une échéance du jour, un filtre, une tâche terminée et un état vide possèdent chacun un traitement visuel distinct.
- **RG-F5-04 — Actions explicites.** Recherche, historique, paramètres, retour, fermeture et enregistrement disposent de boutons visibles. Ces boutons complètent les raccourcis existants sans les remplacer et conservent un focus clavier identifiable.
- **RG-F5-05 — États transitoires.** Aide, erreur, recherche, filtre et brouillon restent lisibles dans la surface concernée. Fermer le panneau abandonne ces états non enregistrés ; sa prochaine ouverture présente la vue quotidienne.
- **RG-F5-06 — Comportement préservé.** Le format `tasks.md`, les commandes applicatives, les règles de classement, l'historique, la complétion et la persistance ne changent pas.

## Vérifications fonctionnelles

- La capture reste focalisée à l'ouverture et conserve son texte lorsqu'une validation échoue ; son aide et son erreur ne débordent pas de la fenêtre.
- Les titres longs, plusieurs tags, les dates et les deadlines restent lisibles dans une ligne sélectionnée ou terminée.
- Les boutons du panneau sont activables à la souris et par Tab puis Entrée ; les raccourcis documentés dans F2 et F3 restent opérationnels.
- Une fermeture suivie d'une réouverture retire recherche, filtre et brouillon, puis affiche la vue quotidienne.
- Les vérifications automatisées sont complétées par une comparaison avec la maquette sous Windows, X11 et Wayland.

## Frontière avec les autres features

F5 ne possède aucune règle métier. F0 reste responsable des surfaces desktop et des paramètres, F1 de la capture et de la consultation, F2 de l'organisation et de l'édition, F3 de la complétion et de l'historique, et F4 de la future configuration des raccourcis.
