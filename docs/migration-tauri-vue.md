# Migration Iced → Tauri / Vue

## Décisions validées

Le 12 octobre 2026, les choix suivants ont été validés avec l'utilisateur.

| ID | Décision |
| --- | --- |
| D1 | Préserver les fonctionnalités et la direction graphique actuelles. Le minimalisme concerne la structure Vue. |
| D2 | Remplacement direct sur une branche dédiée, sans maintien des deux interfaces. |
| D3 | Garder le domaine, les commandes, le parsing, les dates, sections, recherches et filtres en Rust. Vue porte l'affichage et les interactions. |
| D4 | Tauri 2, Vue 3, Vite, TypeScript, pnpm, CSS simple et état réactif Vue. |
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
- Installer un frontend Vue commun à la capture et au panneau ; les paramètres sont une vue qui remplace le contenu du panneau.
- Brancher les intégrations système validées, le watcher et le réveil au changement de journée.
- Vérifier démarrage invisible, absence de duplication, focus, fermeture/réouverture et survie du cœur résident.
- Mesurer mémoire au repos et délai d'ouverture.

Les surfaces livrées par ce lot étaient des prototypes : capture sans création, panneau sans liste et paramètres consultatifs. Le cœur et ses tests de comportements persistés ont été conservés. Le contrat fonctionnel Vue/Rust est défini dans le portage autorisé le 13 octobre.

## Portage fonctionnel autorisé — 13 octobre 2026

L'utilisateur a demandé d'implémenter les fonctionnalités manquantes avec trois agents dédiés à l'implémentation, la revue de code et les tests. Le superviseur coordonne l'équipe et tranche les désaccords ; une question produit importante que les références ne permettent pas de résoudre doit être soumise à l'utilisateur. Cette demande autorise les lots fonctionnels F0–F5, en conservant les décisions D1–D8 et les règles produit existantes.

1. Définir le contrat des commandes de tâches, snapshots, erreurs et événements, y compris la resynchronisation de chaque nouvelle fenêtre.
2. Porter la capture persistée et l'ajout intégré.
3. Porter le panneau, l'édition, la recherche, les tags et les permutations.
4. Porter la complétion, la réouverture quotidienne et l'historique.
5. Porter la modification des paramètres et des raccourcis.
6. Valider les parcours complets et consolider la présentation F5.

L'architecture est la référence des décisions techniques ; la SFG et F0–F5 restent les références des comportements attendus. Le packaging Windows/Linux reste distinct du portage fonctionnel ; sa distribution est définie dans l'architecture et automatisée par la pipeline GitHub.

L'avancement des agents est présenté dans le [tableau visuel](validation/migration-agents.html) et son [aperçu SVG](validation/migration-agents.svg). Les états sont actualisés par le superviseur aux jalons ; la réussite des tests initiaux ne vaut pas validation du portage.

## Validation et couverture

Avant migration : 107 tests passaient. Les tests spécifiques aux widgets Iced ont été retirés avec leur implémentation. Les tests du socle ne les remplaçaient pas : des tests DOM Vue et du contrat IPC ont été ajoutés pendant le portage fonctionnel. Les essais de focus natif et de défilement restent distincts des tests sans fenêtre.

Les scénarios de domaine, repository, parsing, projections et persistance sont conservés. Les tests du snapshot optimiste sont déplacés dans `src/runtime/task_store.rs`. `src/runtime/coordinator.rs` vérifie la FIFO, la déduplication des validations et le refus d'un changement de source tant qu'une capture reste en attente. Le backend teste les réservations des fenêtres et les activations du tray Linux.

Les résultats de compilation, tests et mesures ainsi que les vérifications graphiques non effectuées doivent être indiqués à la livraison. Une compilation Linux ne valide pas Windows ni une autre session graphique.

## Bilan du lot 1 — 12 octobre 2026

- Construction complète `pnpm run desktop:build -- --debug` réussie, frontend TypeScript/Vite compris.
- Contrôle du backend en profil release réussi ; la sonde de développement est exclue de ce profil.
- 77 tests de la bibliothèque et 5 tests du backend passent ; Clippy passe avec les avertissements traités comme erreurs.
- Le contrôle de la bibliothèque pour `x86_64-pc-windows-gnu` passe.
- Le contrôle du backend pour cette cible est arrêté par l'absence de `x86_64-w64-mingw32-windres` dans l'environnement. Il n'est pas présenté comme une validation Windows.
- La sonde sur Wayland vérifie démarrage sans fenêtre, ouverture sans duplication, trois réouvertures de capture avec focus natif observé, remplacement du panneau par les paramètres sans fenêtre supplémentaire, destruction des surfaces, second lancement silencieux et watcher valide/invalide après fermeture des fenêtres.

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

Le test comparatif avec réutilisation de fenêtres masquées a ensuite été autorisé par l'utilisateur. À cette étape, les autres lots n'étaient pas commencés.

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

**Décision validée après comparaison :** conserver et masquer les fenêtres après leur première ouverture, pour privilégier les réouvertures. Le comportement normal applique désormais ce choix. Les fermetures natives sont interceptées et masquent la surface ; le panneau se bascule selon sa visibilité, et une activation distincte permet la remise à zéro des états transitoires sans effacer une saisie lors d'une simple demande de focus. Les autres lots n'étaient pas encore autorisés à cette étape ; leur autorisation ultérieure est indiquée plus haut.

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

## Bilan du portage fonctionnel — 13 octobre 2026

| Feature | Comportement porté |
| --- | --- |
| F0 | Changement transactionnel du fichier, création ou choix natif, autostart modifiable, intégrations desktop conservées. |
| F1 | Capture persistée en FIFO, fermeture après confirmation, ajout intégré et consultation des tâches. |
| F2 | Métadonnées, aide F1, quatre sections, édition, recherche, tags, filtres et réordonnancement clavier. |
| F3 | Complétion et réouverture quotidiennes, historique consultatif, projections au changement de jour. |
| F4 | Raccourcis persistés, collision refusée, remplacement par action et état actif/inactif. |
| F5 | Palette claire, contrôles et SVG communs, tags colorés, états distincts, capture dimensionnée pour aide/erreur et listes défilantes. |

Le contrat IPC et les règles de resynchronisation sont décrits dans l'architecture, §9.2. Les propositions optimistes, confirmations et erreurs restent gérées par Rust. Les fenêtres et les promesses Vue vérifient la session et l'activation pour ne pas affecter une surface rouverte. La récupération d'une ancienne saisie échouée préserve les autres brouillons.

La revue indépendante a identifié et fait corriger les risques de versions obsolètes, de fermetures retardées, de perte de brouillons, de focus, de sélection après déplacement, de réponses de paramètres et de renouvellement des raccourcis. Le repository, le watcher et la configuration sont alignés sur le même chemin canonique. Aucun défaut bloquant restant n'a été identifié lors de la revue finale du code.

La construction frontend TypeScript/Vite, les contrôles backend dev/release, Clippy des deux crates et le contrôle Windows GNU de la bibliothèque passent. Les derniers compteurs observés sont 80 tests Rust, 14 tests backend et 22 tests DOM Vue. Les régressions finales vérifient aussi les gardes de fermeture native, la récupération de brouillons et les réponses retardées des actions et dialogues.

La sonde fonctionnelle debug est écrite et son lancement graphique nécessite une autorisation du sandbox. Aucun résultat natif du portage n'est encore présenté comme validé. Les intégrations réelles du tray, des raccourcis, de l'autostart et des dialogues doivent également être essayées dans les sessions Windows, X11 et Wayland ; une sonde isolée qui inhibe les modifications système ne les valide pas. Les releases GitHub publiées sur les tags `X.Y.Z` produisent des paquets `.deb` et `.AppImage` pour Linux, ainsi que `.msi` et NSIS pour Windows.

## Correction et validation des raccourcis KDE — 14 octobre 2026

Sous KDE Plasma 6.7.5 / Wayland avec xdg-desktop-portal 1.22.1, l'identité du portail est désormais `local.mystline`, sans le suffixe `.desktop` que KGlobalAccel réserve aux lanceurs. Une entrée locale cachée fournit les métadonnées requises par le registre. Les deux actions partagent une session ; une réponse sans touche attribuée reste inactive.

L'utilisateur a confirmé que `Ctrl+Shift+Espace` et `Ctrl+Shift+T` fonctionnent après autorisation initiale, puis qu'ils fonctionnent encore après fermeture et relancement sans nouvelle fenêtre KDE. Les journaux montrent la réception des deux activations et KDE conserve les deux attributions dans le composant `local.mystline`. Le test natif de restauration dans deux sessions neuves passe, ainsi que les 23 tests backend ordinaires, Clippy et la compilation du binaire debug avec `custom-protocol`.

Cette vérification couvre le déclenchement des deux valeurs initiales et leur restauration sous KDE. La modification d'une combinaison et les sessions Windows/X11 restent à valider manuellement.
