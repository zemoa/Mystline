---
name: mystline-release
description: Préparer et publier une release Mystline depuis la branche courante, avec analyse gpt-6.1-sol en effort low, choix de version, commit, push, tag et changelog GitHub.
---

# Release Mystline

Effectuer la release lorsque l'utilisateur la demande. Créer ou modifier ce skill ne constitue pas une demande de publication. Une demande de release autorise le commit, les push de branche et de tag, et la publication GitHub ; poursuivre sans confirmations supplémentaires sauf information indispensable ou permission d'exécution imposée par l'environnement.

Lire `AGENTS.md`, `docs/DEVELOPPER.md` et `.github/workflows/ci.yml`. Consulter `docs/SFG.md` et les documents fonctionnels concernés pour qualifier les changements. Préfixer les commandes shell avec `rtk`, ou `rtk proxy` si nécessaire.

## 1. Lister les changements avec gpt-6.1-sol

Exécuter cette analyse exclusivement avec le **modèle `gpt-6.1-sol`** et un effort **`low`**, via la délégation disponible et l'identifiant réel proposé par l'environnement. Ne pas inventer d'identifiant, d'alias ou de champ de configuration du skill. Si `gpt-6.1-sol` ou l'effort `low` n'est pas disponible, le signaler et demander un modèle de remplacement avant de poursuivre l'analyse ; ne pas substituer silencieusement un autre modèle.

Avant de modifier les versions :

1. Identifier branche courante, HEAD, remote et upstream. Un HEAD détaché nécessite une branche cible explicitement indiquée. Vérifier le dépôt GitHub via le remote (actuellement `zemoa/Mystline`).
2. Vérifier index et worktree. Ne pas intégrer des modifications préexistantes sans instruction sur leur inclusion. Continuer les lectures indépendantes si une clarification est nécessaire.
3. Récupérer tags et références distantes : `rtk git fetch REMOTE --tags`. Compléter l'historique d'un checkout superficiel si nécessaire. Ne pas supprimer ou remplacer de tag, forcer un push ou réécrire l'historique.
4. Trouver le dernier tag de release accessible depuis HEAD avec `rtk git describe --tags --abbrev=0 --match '[0-9]*.[0-9]*.[0-9]*' HEAD`. Valider le format exact `^[0-9]+\.[0-9]+\.[0-9]+$` ; si le tag ne convient pas, rechercher le précédent en excluant les tags non conformes. Choisir le tag le plus proche dans l'historique de la branche, pas un tag d'une autre branche.
5. Fournir à gpt-6.1-sol le tag, le SHA de HEAD, les résultats de `git log TAG..HEAD`, `git diff --stat TAG HEAD`, `git diff --name-status TAG HEAD`, et les diffs détaillés nécessaires. Sans tag accessible, analyser l'historique et le contenu depuis le début du projet et annoncer une première release.

Déléguer uniquement l'analyse en lecture seule. Demander à gpt-6.1-sol une liste des fonctionnalités, petites améliorations, corrections et changements de maintenance, avec références aux commits/fichiers, impact utilisateur, ruptures éventuelles et recommandation de version motivée. Vérifier les changements effectifs : les messages de commits seuls ne suffisent pas, et les changements annulés ne sont pas des nouveautés.

Afficher cette liste à l'utilisateur **avant** toute modification de version. Ne pas créer une release vide. Conserver le SHA analysé et refaire l'analyse si HEAD évolue avant le commit de release.

## 2. Choisir X.Y.Z

| Changements depuis le tag | Nouvelle version |
| --- | --- |
| Au moins une fonctionnalité significative : nouvelle capacité ou nouveau parcours utilisateur | `X.(Y+1).0` |
| Corrections, toutes petites fonctionnalités ou améliorations limitées d'un parcours existant | `X.Y.(Z+1)` |
| Maintenance seule réellement nécessaire à la distribution | `X.Y.(Z+1)` |

Une fonctionnalité significative prime sur les corrections dans une release mixte. Justifier les cas limites à partir de l'impact utilisateur ; un commit `feat` n'impose pas automatiquement Y.

**Seul l'utilisateur peut décider de changer X.** Ne jamais augmenter X automatiquement, même pour une rupture de compatibilité. Si une rupture impose une décision, présenter ses conséquences et demander la version majeure souhaitée. Une version majeure explicitement choisie utilise normalement `X.0.0`, sauf version complète donnée par l'utilisateur.

Prendre le dernier tag comme référence publiée. Examiner les versions déjà préparées dans la branche et conserver une version conforme à la règle pour éviter un double incrément. Résoudre toute divergence inexpliquée avant publication. Sans tag, utiliser la version initiale cohérente des manifests, sauf choix explicite de l'utilisateur. Vérifier que la version finale dépasse la dernière version publiée et que son tag/release n'existe pas sur le remote. Ne pas écraser une version existante.

## 3. Modifier la version et préparer le changelog

Aligner la version dans les cinq manifests :

- `Cargo.toml` : paquet `mystline` ;
- `src-tauri/Cargo.toml` : paquet `mystline-desktop` ;
- `src-tauri/tauri.conf.json` : version de l'application distribuée ;
- `package.json` et `frontend/package.json` : paquets locaux.

Synchroniser les entrées des paquets locaux dans `Cargo.lock` et `src-tauri/Cargo.lock`, sans mise à jour des dépendances tierces. Vérifier `pnpm-lock.yaml` et ne le modifier que si son contenu dépend des versions locales. Ne pas remplacer globalement toutes les occurrences de l'ancienne version.

Créer `docs/releases/X.Y.Z.md` : titre `Mystline X.Y.Z`, sections non vides pour fonctionnalités, améliorations, corrections et maintenance pertinente, conséquences des ruptures, puis lien GitHub de comparaison entre les tags. Pour une première release, décrire les capacités livrées sans inventer de comparaison. Décrire les changements effectifs pour l'utilisateur, sans recopier les commits ni compter le bump comme une nouveauté.

Exécuter `rtk proxy node scripts/check-release.mjs X.Y.Z`, puis les contrôles requis dans `docs/DEVELOPPER.md` et le workflow : formatage, Clippy et tests des deux projets Rust, tests DOM Vue, build frontend et build Tauri avec lockfiles verrouillés. Indiquer les vérifications natives restant à faire. Ne pas pousser de tag si un contrôle obligatoire échoue ; résoudre le problème ou signaler le blocage réel.

Présenter version choisie, justification, changelog et résultats de validation. Examiner le diff final et mettre dans l'index uniquement les fichiers préparés pour la release.

## 4. Commit et push, puis tag et release

1. Créer le commit `chore(release): préparer X.Y.Z` avec versions, lockfiles concernés et changelog. Vérifier son contenu et noter son SHA.
2. Pousser explicitement la branche vers le remote et la branche cible. Vérifier que le push est une avance rapide ; en cas de rejet, examiner la divergence sans forcer.
3. Créer un tag annoté **`X.Y.Z` sans préfixe `v`**, sur le SHA exact du commit de release, avec le message `Mystline X.Y.Z`. Après réussite du push de branche, pousser uniquement ce tag. Vérifier son SHA distant.
4. Le tag déclenche `.github/workflows/ci.yml` : création d'une release en brouillon, packaging Linux et Windows, fusion de `latest.json`, puis publication complète. Laisser le workflow gérer création, assets, signatures et publication ; ne pas créer de release concurrente.
5. Identifier l'exécution liée au **tag et au SHA exacts**, suivre sa progression et communiquer régulièrement. Attendre la réussite de tous les jobs, dont `publish`. En cas d'échec, rapporter job et cause ; ne pas annoncer une release réussie ni déplacer le tag. Une correction nécessite un nouveau commit et une version encore disponible.
6. Après réussite du workflow, remplacer le texte générique par le changelog committé : `rtk proxy gh release edit X.Y.Z --repo OWNER/REPO --title 'Mystline X.Y.Z' --notes-file docs/releases/X.Y.Z.md`. Attendre la fin des jobs de packaging, qui utilisent leur propre `releaseBody`, pour éviter qu'ils écrasent les notes.
7. Vérifier avec `gh release view` : bon tag, release publiée et stable, bon changelog, paquets `.deb`, `.AppImage`, `.msi`, NSIS `.exe`, signatures et `latest.json`. Vérifier que ce manifeste annonce la version choisie et contient Linux et Windows.

En cas d'interruption, examiner l'état distant avant reprise. Réutiliser commit, tag et release uniquement s'ils correspondent exactement à cette tentative ; ne pas refaire un bump, créer un doublon ou modifier une autre release. Si seules les notes manquent, reprendre à leur mise à jour.

Terminer par version, SHA du commit, lien de release et validations. Distinguer un push effectué, une CI en cours et une release publiée avec changelog vérifié.
