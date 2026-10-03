# Spécification fonctionnelle — Gestionnaire de tâches local

## 1. Objectif du produit

Créer une application de gestion de tâches personnelle, locale, rapide et minimaliste.

L’application doit répondre à deux problèmes principaux :

* capturer une tâche immédiatement lorsqu’elle apparaît, sans interrompre fortement l’activité en cours ;
* visualiser rapidement l’ensemble des tâches et identifier ce qui doit être fait aujourd’hui ou prochainement.

L’application ne doit pas devenir elle-même une charge de gestion.

Le produit s’inspire de certains principes de GTD, notamment la capture rapide et la préparation de la journée suivante, sans imposer à l’utilisateur une méthodologie stricte.

---

# 2. Principes de conception

## 2.1 Rapidité avant organisation

Créer une tâche doit demander le minimum d’effort possible.

Le scénario nominal est :

**Raccourci clavier → saisie → Entrée → retour immédiat à l’activité précédente.**

Aucune information complémentaire ne doit être obligatoire.

---

## 2.2 Pas d’organisation forcée

Une tâche peut exister uniquement avec un titre.

L’utilisateur n’est jamais obligé de renseigner :

* une date ;
* une deadline ;
* un tag ;
* une priorité ;
* un statut intermédiaire.

---

## 2.3 Pas de statut « En cours »

Une tâche possède seulement deux états :

**À faire**
**Terminée**

Le fait de vouloir travailler sur une tâche aujourd’hui est représenté par sa planification pour aujourd’hui et non par un statut spécifique.

---

## 2.4 Priorité implicite

Il n’existe pas de champ de priorité du type Haute / Moyenne / Basse.

L’ordre des tâches dans une section représente leur priorité relative.

L’utilisateur peut réordonner les tâches.

---

## 2.5 Silence par défaut

L'application ne génère pas de notifications pour :

* les tâches prévues ;
* les deadlines ;
* les tâches en retard.

Les informations sont visibles lorsque l’utilisateur ouvre le panneau.

---

# 3. Plateformes

Le produit doit fonctionner au minimum sur :

* Windows ;
* Linux.

L’application fonctionne localement.

Il n’existe :

* aucun compte utilisateur ;
* aucun service cloud ;
* aucune synchronisation ;
* aucune dépendance à Internet.

Les données restent sur la machine.

---

# 4. Fonctionnement général

L’application démarre automatiquement à l’ouverture de la session utilisateur.

Elle reste ensuite en arrière-plan.

Elle ne doit pas afficher automatiquement sa fenêtre principale au démarrage.

Deux interactions principales doivent être accessibles par raccourcis clavier globaux.

### Capture rapide

Un raccourci fait apparaître une petite fenêtre de saisie.

L’utilisateur écrit une tâche puis appuie sur Entrée.

La fenêtre disparaît immédiatement.

### Affichage des tâches

Un second raccourci affiche ou masque un panneau léger contenant les tâches.

Le panneau doit pouvoir être appelé et fermé rapidement sans passer par une fenêtre d’application classique.

---

# 5. Modèle d'une tâche

Une tâche contient uniquement les informations suivantes dans le MVP.

### Titre

Obligatoire.

Texte libre décrivant l’action.

Exemple :

`Préparer le support de présentation`

### Date prévue

Facultative.

Elle représente le jour pendant lequel l’utilisateur prévoit de travailler sur la tâche.

Elle ne contient jamais d’heure.

Exemple :

`Prévu : jeudi`

Il s’agit d’une intention et non d’une contrainte.

### Deadline

Facultative.

Elle représente la date à laquelle la tâche doit impérativement être terminée.

Exemple :

`Deadline : vendredi`

La deadline est conceptuellement différente de la date prévue.

Une tâche peut donc avoir simultanément :

`Prévu jeudi`

et

`Deadline vendredi`

### Tags

Facultatifs.

Une tâche peut posséder plusieurs tags.

Exemples :

`#mission`

`#documentation`

`#fond`

Les tags permettent de donner du contexte à une tâche sans introduire de notions de projet ou de sous-tâches.

### État

Deux valeurs uniquement :

`À faire`

`Terminée`

### Ordre

Chaque tâche possède une position permettant son réordonnancement manuel dans les listes.

---

# 6. Capture rapide

La capture rapide est une fonction centrale du produit.

Lorsque le raccourci global de création est utilisé, une petite zone de saisie apparaît immédiatement.

Le focus clavier est automatiquement placé dans la zone de texte.

Exemple :

`Vérifier le dossier d'architecture`

Puis :

`Entrée`

La tâche est créée.

La fenêtre disparaît.

La tâche apparaît ensuite dans **Toutes les tâches**.

Il n’existe pas d’Inbox intermédiaire.

---

# 7. Commandes dans la saisie

L’application n’interprète pas automatiquement le langage naturel.

Les modifications de métadonnées utilisent une syntaxe explicite.

Exemples envisagés :

`#mission`

ajoute le tag `mission`.

`/p demain`

planifie la tâche pour demain.

`/p vendredi`

planifie la tâche pour vendredi.

`/d vendredi`

définit une deadline pour vendredi.

Exemple complet :

`Préparer le support /p jeudi /d vendredi #mission`

L’application extrait les commandes et enregistre comme titre :

`Préparer le support`

avec :

Prévu : jeudi
Deadline : vendredi
Tag : mission

La syntaxe exacte pourra être ajustée lors de la conception détaillée, mais le principe `/commande valeur` doit être conservé.

---

# 8. Aide contextuelle

L’utilisateur ne doit pas avoir à mémoriser immédiatement toutes les commandes.

Depuis la zone de saisie ou d’édition, un raccourci clavier permet d’afficher une aide compacte.

Cette aide présente notamment :

`/p` — planifier une tâche

`/d` — ajouter une deadline

`#tag` — ajouter un tag

L’aide doit pouvoir être ouverte et fermée sans utiliser la souris.

`Échap` peut notamment permettre de la fermer.

---

# 9. Vue principale

Le raccourci d’affichage ouvre un panneau léger.

Le panneau présente les tâches en quatre zones principales, dans cet ordre :

## En retard

Contient les tâches dont la deadline est dépassée et qui ne sont pas terminées.

Cette section doit être visuellement identifiable.

Une tâche en retard doit être difficile à ignorer.

L’utilisateur peut ensuite modifier sa deadline s’il souhaite la reprioriser.

## Aujourd'hui

Contient les tâches planifiées pour aujourd’hui.

Une tâche peut être cochée directement depuis cette section.

## Prochain jour ouvré

Contient les tâches prévues pour le prochain jour compris entre lundi et vendredi.

Exemple :

si nous sommes vendredi, cette section affiche les tâches prévues lundi.

Les jours fériés ne sont pas pris en compte.

## Toutes les tâches

Contient toutes les tâches restantes.

Cela comprend notamment :

* les tâches sans date ;
* les tâches prévues après le prochain jour ouvré ;
* les tâches de fond ;
* les tâches avec une deadline future non imminente.

Une tâche planifiée dans plusieurs jours reste donc dans cette section avec une indication discrète de sa date.

---

# 10. Absence de doublons

Une tâche ne doit jamais apparaître simultanément dans plusieurs sections principales.

Par exemple, une tâche prévue aujourd’hui apparaît dans **Aujourd’hui** et n’apparaît pas également dans **Toutes les tâches**.

Ordre de classification :

En retard
Aujourd’hui
Prochain jour ouvré
Toutes les tâches

---

# 11. Deadlines

Une deadline future éloignée doit rester discrète.

Exemple :

une tâche dont la deadline est dans deux semaines peut simplement afficher :

`échéance 14 oct.`

Elle ne doit pas prendre visuellement le dessus sur les tâches quotidiennes.

Lorsque la deadline arrive aujourd’hui ou très prochainement, son affichage peut devenir plus visible.

Lorsque la deadline est dépassée, la tâche passe dans la zone **En retard**.

---

# 12. Tâches longues ou « de fond »

Le produit ne possède pas de type particulier « tâche de fond ».

Une tâche longue reste une tâche classique.

Exemple :

`Améliorer la documentation de la cellule architecture`

Elle peut simplement recevoir des tags tels que :

`#fond`

`#objectif-mission`

Elle reste visible dans **Toutes les tâches**.

Pour décider de travailler dessus aujourd’hui, l’utilisateur peut simplement la planifier pour aujourd’hui.

Il n'est pas nécessaire de créer des sous-tâches.

---

# 13. Sous-tâches

Les sous-tâches ne font pas partie du MVP.

La complexité générée par une hiérarchie de tâches est volontairement évitée.

Le produit doit privilégier l’adoption quotidienne plutôt que la sophistication fonctionnelle.

---

# 14. Tâches terminées

Une tâche peut être cochée directement depuis la liste.

Lorsqu'une tâche est terminée :

* elle reste visible pendant la journée ;
* son texte apparaît barré ;
* elle permet à l’utilisateur de visualiser ce qu’il a accompli.

À partir du lendemain, elle disparaît de la vue principale.

Elle reste néanmoins conservée dans un historique.

---

# 15. Historique

Une vue secondaire permet de consulter les tâches terminées.

Cette vue n’est pas centrale dans l’usage quotidien.

Elle permet notamment de retrouver ce qui a été réalisé les jours précédents.

---

# 16. Réorganisation

Les tâches peuvent être réordonnées manuellement.

L’ordre représente implicitement leur importance.

Interaction clavier proposée :

`Ctrl + ↑`

déplace la tâche vers le haut.

`Ctrl + ↓`

déplace la tâche vers le bas.

Un glisser-déposer à la souris peut être proposé en complément.

---

# 17. Navigation clavier

L’application doit être utilisable très largement sans souris.

Les interactions suivantes doivent notamment être possibles au clavier :

* ouvrir le panneau ;
* fermer le panneau ;
* créer une tâche ;
* naviguer entre les tâches ;
* sélectionner une tâche ;
* cocher une tâche ;
* modifier une tâche ;
* réordonner une tâche ;
* rechercher ;
* filtrer par tag ;
* afficher l’aide.

Les touches précises seront déterminées lors de la conception de l’interface.

---

# 18. Modification d'une tâche

Une tâche existante doit pouvoir être modifiée directement depuis le panneau.

Exemple d’interaction :

sélection de la tâche ;

`Entrée` ou `E` ;

la tâche passe en édition ;

modification au clavier ;

`Entrée` valide.

La modification ne doit pas nécessiter l’ouverture d’une fenêtre complexe.

La même syntaxe que lors de la création peut être utilisée pour modifier la planification, la deadline ou les tags.

---

# 19. Tags

Une tâche peut posséder plusieurs tags.

Les tags sont visibles discrètement à côté de la tâche.

Cliquer ou sélectionner un tag permet de filtrer la liste.

Le filtrage doit rester simple :

`#mission`

→ affiche les tâches possédant le tag mission.

Il n’est pas nécessaire dans le MVP de proposer des requêtes complexes combinant plusieurs filtres avec des opérateurs logiques.

---

# 20. Recherche

Une recherche instantanée doit permettre de retrouver une tâche en tapant quelques lettres.

La recherche doit fonctionner sur au minimum :

* le titre ;
* les tags.

Elle doit être accessible rapidement au clavier.

---

# 21. Préparation de la journée suivante

L’utilisateur souhaite pouvoir organiser sa journée suivante selon une logique inspirée de GTD.

Cette organisation reste facultative.

L’application ne doit pas supposer que l’utilisateur effectuera systématiquement cette revue.

Préparer une journée consiste essentiellement à sélectionner certaines tâches et à leur attribuer comme date prévue le prochain jour ouvré.

Aucune heure n’est associée aux tâches.

L’utilisateur doit également pouvoir effectuer spontanément une tâche qui n’avait pas été planifiée.

---

# 22. Tâches non réalisées

Une tâche prévue aujourd’hui mais non réalisée ne doit pas être supprimée ni considérée comme un échec particulier.

Elle reste visible.

L’utilisateur pourra ensuite :

* la faire ;
* changer sa date prévue ;
* modifier sa deadline ;
* la replacer parmi les autres tâches.

Le système ne doit pas multiplier les alertes ou les demandes de confirmation.

---

# 23. Récurrence

Les tâches récurrentes ne font pas partie du MVP.

Toutes les tâches sont ponctuelles.

---

# 24. Fonctionnalités explicitement hors périmètre MVP

Sont volontairement exclues :

* synchronisation cloud ;
* compte utilisateur ;
* application web ;
* notifications ;
* rappels ;
* gestion d’heures ;
* calendrier horaire ;
* sous-tâches ;
* projets complexes ;
* états multiples ;
* priorités Haute / Moyenne / Basse ;
* pièces jointes ;
* notes longues ;
* tâches récurrentes ;
* collaboration ;
* dépendances entre tâches ;
* intégration email ou messagerie.

---

# 25. Critères de réussite du produit

Le produit remplit son objectif si l’utilisateur peut :

**capturer une tâche en quelques secondes sans chercher une application ;**

**faire apparaître toutes ses tâches instantanément ;**

**comprendre en quelques secondes ce qui est en retard, prévu aujourd’hui et prévu au prochain jour ouvré ;**

**garder visibles les sujets de fond ;**

**organiser ponctuellement sa journée suivante sans transformer l’outil en agenda ;**

**utiliser l’application principalement au clavier ;**

**arrêter de compter sur sa mémoire pour retenir les tâches.**

---

# 26. Vision du panneau principal

Conceptuellement, le panneau pourrait ressembler à ceci :

**EN RETARD**

☐ Envoyer validation architecture — échéance hier `#mission`

**AUJOURD'HUI**

☐ Relire proposition technique `#client`

☐ Améliorer documentation cellule archi `#fond` `#mission`

~~✓ Répondre à Paul~~

**PROCHAIN JOUR OUVRÉ · JEUDI**

☐ Préparer support comité — échéance vendredi `#mission`

**TOUTES LES TÂCHES**

☐ Vérifier nouvelle version API `#architecture`

☐ Revoir documentation ADR — prévu lundi `#fond`

☐ Faire le point sur les accès

La totalité de cette vue doit rester compacte et immédiatement lisible.

---

# 27. Principe directeur du MVP

À chaque ajout fonctionnel, appliquer la question suivante :

**« Est-ce que cette fonction m’aide à noter ou à voir mes tâches plus facilement, ou est-ce qu’elle m’oblige à davantage gérer mon gestionnaire de tâches ? »**

Si elle augmente principalement la gestion du système, elle doit probablement rester hors du MVP.
