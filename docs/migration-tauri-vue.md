# Migration Iced → Tauri / Vue

## Décisions validées

Le 12 octobre 2026, les choix suivants ont été validés avec l'utilisateur.

| ID | Décision |
| --- | --- |
| D1 | Préserver les fonctionnalités et la direction graphique actuelles. Le minimalisme concerne la structure Vue. |
| D2 | Remplacement direct sur une branche dédiée, sans maintien des deux interfaces. |
| D3 | Garder le domaine, les commandes, le parsing, les dates, sections, recherches et filtres en Rust. Vue porte l'affichage et les interactions. |
| D4 | Tauri 2, Vue 3, Vite, TypeScript, npm, CSS simple et état réactif Vue. |
| D5 | Conserver la bibliothèque Rust à la racine et ajouter `frontend/` et `src-tauri/`. |
| D6 | API tray Tauri sur Windows ; adaptation KSNI sur Linux ; plugin global-shortcut sur Windows/X11 ; portail XDG via ashpd sur Wayland ; plugins autostart et single-instance. |
| D7 | Conserver et masquer les fenêtres après leur première ouverture ; choix validé après le test comparatif Wayland. |
| D8 | Garder l'affichage optimiste, le retour à l'état confirmé après erreur et la fermeture de la capture seulement après confirmation. |

La règle de processus devient **une seule instance applicative avec un cœur Rust résident**. Les WebViews peuvent utiliser des processus supplémentaires. Le second lancement reste silencieux.

La migration prend comme base l'état du dossier, modifications non commitées comprises. La branche de travail est `migration/tauri-vue`. Une copie initiale des sources et documents a été conservée dans `/tmp/mystline-before-tauri-20261012.tar.gz` ; cette sauvegarde locale temporaire n'est pas un artefact pérenne du projet.

## Lot 1 autorisé — Socle desktop

- Retirer le point d'entrée, les composants et les dépendances Iced.
- Extraire le stockage asynchrone et la coordination dans `src/runtime/`.
- Garder la sérialisation des écritures, la FIFO de captures et les protections de révision/génération.
- Installer un frontend Vue commun à trois fenêtres : capture, panneau, paramètres.
- Brancher les intégrations système validées, le watcher et le réveil au changement de journée.
- Vérifier démarrage invisible, absence de duplication, focus, fermeture/réouverture et survie du cœur résident.
- Mesurer mémoire au repos et délai d'ouverture.

Les surfaces de ce lot sont des prototypes. La capture ne crée pas encore de tâche, le panneau ne rend pas encore la liste et les paramètres sont consultatifs. Le cœur et ses tests de comportements persistés restent disponibles. Aucun contrat de commandes de tâches Vue/Rust n'est encore figé.

## Lots suivants — Non autorisés pour implémentation

1. Définir et faire valider le contrat des commandes de tâches, snapshots, erreurs et événements, y compris la resynchronisation de chaque nouvelle fenêtre.
2. Porter la capture persistée et l'ajout intégré.
3. Porter le panneau, l'édition, la recherche, les tags et les permutations.
4. Porter la complétion, la réouverture quotidienne et l'historique.
5. Porter la modification des paramètres et des raccourcis.
6. Valider les parcours complets et décider du packaging Windows/Linux.

Les choix supplémentaires et le passage à chaque lot font l'objet d'une validation utilisateur. L'architecture est la référence des décisions techniques ; la SFG et F0–F5 restent les références des comportements attendus.

## Validation et couverture

Avant migration : 107 tests passaient. Les tests spécifiques aux widgets Iced ont été retirés avec leur implémentation. Ils ne sont pas considérés comme remplacés par les tests du socle : leur couverture clavier, sélection, aide, édition et défilement devra être rétablie pendant les lots de portage concernés.

Les scénarios de domaine, repository, parsing, projections et persistance sont conservés. Les tests du snapshot optimiste sont déplacés dans `src/runtime/task_store.rs`. `src/runtime/coordinator.rs` vérifie la FIFO, la déduplication des validations et le refus d'un changement de source tant qu'une capture reste en attente. Le backend teste les réservations des fenêtres et les activations du tray Linux.

Les résultats de compilation, tests et mesures ainsi que les vérifications graphiques non effectuées doivent être indiqués à la livraison. Une compilation Linux ne valide pas Windows ni une autre session graphique.

## Bilan du lot 1 — 12 octobre 2026

- Construction complète `npm run desktop:build -- --debug` réussie, frontend TypeScript/Vite compris.
- Contrôle du backend en profil release réussi ; la sonde de développement est exclue de ce profil.
- 77 tests de la bibliothèque et 5 tests du backend passent ; Clippy passe avec les avertissements traités comme erreurs.
- Le contrôle de la bibliothèque pour `x86_64-pc-windows-gnu` passe.
- Le contrôle du backend pour cette cible est arrêté par l'absence de `x86_64-w64-mingw32-windres` dans l'environnement. Il n'est pas présenté comme une validation Windows.
- La sonde sur Wayland vérifie démarrage sans fenêtre, ouverture sans duplication, trois réouvertures de capture avec focus natif observé, panneau et paramètres simultanés, destruction des surfaces, second lancement silencieux et watcher valide/invalide après fermeture des fenêtres.

Le [rapport de mesure](validation/lot1-wayland-debug.json) concerne des surfaces prototypes en debug sous WebKitGTK 2.54.0, avec Tauri 2.12.3. Les temps incluent l'attente de chargement Vue et la résolution de la sonde (20 ms) ; ce ne sont pas des mesures finales de capture utilisateur.

| Observation | Mesure |
| --- | --- |
| Capture : première ouverture | 222 ms |
| Capture : deux réouvertures | 182 / 183 ms |
| Mémoire avant toute fenêtre | 1 processus, 56 Mio PSS |
| Mémoire 1 seconde après destruction des cinq fenêtres créées | 7 processus, 516 Mio PSS |
| Mémoire 10 secondes après destruction | 7 processus, 449 Mio PSS |

La PSS répartit les pages partagées entre les processus ; la somme des RSS, également conservée dans le rapport, compte certaines pages plusieurs fois. La sonde additionne le cœur et ses descendants. Malgré la destruction des fenêtres natives, des processus WebKit restent présents pendant cette mesure : ces observations ont motivé le test comparatif D7 présenté ci-dessous.

Les clics et menus du tray, les raccourcis système, l'autostart et les interactions DOM de saisie restent à vérifier manuellement. La sonde a volontairement évité de modifier autostart et raccourcis. Aucune session graphique Windows/X11 n'a été validée.

Le test comparatif avec réutilisation de fenêtres masquées a ensuite été autorisé par l'utilisateur. Les autres lots ne sont pas commencés.

## Comparaison D7 — Fenêtres détruites ou masquées

Le [rapport comparatif](validation/lot1-window-comparison-wayland-debug.json) conserve les deux mesures effectuées avec le même binaire debug sous Wayland, dans deux processus neufs successifs. Chaque variante ouvre trois fois la capture, puis le panneau et les paramètres, et attend dix secondes après fermeture. Dans le mode masqué, les fenêtres sont créées à leur première ouverture puis conservées.

| Observation | Destruction | Masquage et réutilisation |
| --- | --- | --- |
| Première ouverture de capture | 211 ms | 202 ms |
| Deux réouvertures de capture | 164 / 164 ms | 0,19 / 0,14 ms |
| Mémoire avant les fenêtres | 56 Mio PSS | 56 Mio PSS |
| Mémoire après fermeture et dix secondes | 456 Mio PSS | 436 Mio PSS |
| Processus après dix secondes | 7 | 5 |
| Fenêtres natives conservées | 0 | 3 |
| Fenêtres visibles après fermeture | 0 | 0 |

Focus natif, absence de doublons, second lancement silencieux et watcher valide/invalide passent dans les deux variantes. La sonde remplace maintenant les versions de son fichier temporaire atomiquement : une écriture par troncature pouvait exposer un fichier vide valide avant le contenu invalide et rendre ce contrôle instable. Le repository et le watcher de l'application n'ont pas été modifiés.

Les délais s'arrêtent au chargement Vue et au signal de visibilité native ; ils ne mesurent pas le rendu effectif des pixels à l'écran. Ces essais portent sur les prototypes en debug, pas sur l'interface fonctionnelle finale ni sur Windows/X11. L'écart mémoire observé reste modeste ; la réutilisation supprime surtout le coût de recréation lors des réouvertures.

**Décision validée après comparaison :** conserver et masquer les fenêtres après leur première ouverture, pour privilégier les réouvertures. Le comportement normal applique désormais ce choix. Les fermetures natives sont interceptées et masquent la surface ; le panneau se bascule selon sa visibilité, et une activation distincte permet la remise à zéro des états transitoires sans effacer une saisie lors d'une simple demande de focus. Les autres lots restent non autorisés.

## Vérification du comportement D7 adopté

Le [rapport D7](validation/lot1-d7-adopted-wayland-debug.json) vérifie le chemin normal de l'application après adoption, avec observations du DOM Vue dans la sonde debug :

- trois ouvertures de capture réutilisent la même session WebView ;
- le champ reçoit le focus, le brouillon est effacé à la fermeture, et une demande de focus conserve une saisie en cours ;
- Échap masque la capture via son handler Vue et sa commande Tauri ;
- une fermeture native masque les paramètres et le basculement du panneau réaffiche sa fenêtre conservée ;
- trois fenêtres natives restent conservées, aucune n'est visible après fermeture ;
- instance unique et watcher valide/invalide restent fonctionnels.

La première ouverture mesurée prend 207 ms ; les deux réouvertures prennent 0,28 et 0,35 ms jusqu'au signal de visibilité native. La mémoire après dix secondes est de 435 Mio PSS. Ces valeurs concernent toujours les prototypes en debug et ne remplacent pas les validations Windows/X11.

Les 7 tests du backend passent, y compris fermeture avant chargement et distinction entre réouverture et simple focus. La construction frontend, Clippy et le contrôle release passent. Les observations DOM de la sonde sont exclues du profil release. La bibliothèque conserve ses 77 tests validés.
