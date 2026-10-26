# Spécification fonctionnelle — Gestionnaire de tâches local

> Migration Tauri/Vue : ces règles restent la référence fonctionnelle. Le portage F0–F5 et sa validation sont suivis dans [la migration](migration-tauri-vue.md).

> Présentation validée le 26 octobre 2026 : une liste sans sections, triée par
> **Retard → aujourd'hui → demain → pas de date → plus tard**. Le retard est indiqué
> sur la tâche et une échéance demain ressort en ambre. La [maquette A validée](designs/liste-sans-sections/README.md)
> remplace les regroupements des maquettes du 22 octobre.

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

## 2.3 Deux états, une liste sans sections

Une tâche possède seulement deux états : **À faire** et **Terminée**.
Le panneau présente une liste continue. Les dates déterminent un rang de tri, sans
ajouter de statut à gérer ni de section à l'écran.

---

## 2.4 Priorité implicite

Il n’existe pas de champ de priorité du type Haute / Moyenne / Basse.

À rang de tri égal, l’ordre manuel des tâches représente leur priorité relative.

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

Par défaut, l’application démarre automatiquement à l’ouverture de la session utilisateur. L’utilisateur peut désactiver ce démarrage automatique dans les paramètres.

Les paramètres affichent également le nom de l'application et sa version.

### Mises à jour de l’application

Les distributions Windows et Linux AppImage vérifient au démarrage, puis toutes les 24 heures,
la dernière release stable du dépôt GitHub `zemoa/Mystline`. Une version plus récente est
téléchargée automatiquement et sa signature est vérifiée. Les paramètres indiquent son état
et proposent **Installer et redémarrer** lorsque le téléchargement est prêt. Le redémarrage
reste déclenché par l’utilisateur ; il est refusé tant qu’un enregistrement est en cours ou
que la capture rapide est ouverte. Les paramètres permettent aussi de relancer la vérification.

Une absence de connexion ou une release indisponible n’empêche jamais de gérer les tâches.
Les échanges réseau concernent uniquement les versions et les installateurs ; aucune tâche
ni configuration personnelle n’est transmise. Le paquet Linux `.deb` se met à jour par
réinstallation. Les builds de développement ne vérifient pas les releases.

À chaque démarrage de l’application, la liste quotidienne des tâches s’ouvre par défaut pour
voir les tâches à faire aujourd’hui. L’option **Ouvrir la liste des tâches au démarrage**,
dans les paramètres, permet de désactiver ou réactiver cet affichage. Elle est activée
par défaut, y compris avec une configuration existante, et son choix est conservé.
Elle est indépendante du démarrage automatique avec la session. Lorsqu’elle est désactivée,
l’application démarre en arrière-plan sans fenêtre visible.

Deux interactions principales doivent être accessibles par raccourcis clavier globaux.

### Capture rapide

Un raccourci fait apparaître une petite fenêtre de saisie.

L’utilisateur écrit une tâche puis appuie sur Entrée.

La fenêtre disparaît dès que l'enregistrement est confirmé.

### Affichage des tâches

Un second raccourci affiche ou masque un panneau léger contenant les tâches.

Le panneau doit pouvoir être appelé et fermé rapidement sans passer par une fenêtre d’application classique.

Un clic avec le bouton principal sur l'icône de l'application dans la zone de notification ouvre également le panneau. S'il est déjà ouvert, ce clic lui redonne le focus sans le fermer ni ouvrir une seconde fenêtre. Le bouton secondaire donne accès au menu de l'icône.

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

Une capture avec un titre seul apparaît dans la liste au rang **pas de date**, sans date ajoutée automatiquement.

Le panneau principal propose également une action d'ajout. Elle affiche une saisie intégrée qui
accepte la même syntaxe et enregistre la tâche avec Entrée, sans imposer de passer par le raccourci
global.

Il n’existe pas d’Inbox intermédiaire.

---

# 7. Commandes dans la saisie

L’application n’interprète pas automatiquement le langage naturel.

Les modifications de métadonnées utilisent une syntaxe explicite.

Syntaxe de capture et d'édition :

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

Les dates admises sont `YYYY-MM-DD`, `aujourd'hui`, `demain` et les jours de semaine français. Un jour de semaine désigne sa prochaine occurrence, aujourd'hui compris ; `demain` suit le calendrier, week-end compris. Chaque commande `/p` ou `/d` ne peut apparaître qu'une fois dans une saisie. Une commande sans date valide ou une saisie sans titre est refusée en conservant le texte et sans écrire dans le fichier.

Un token littéral peut être échappé : `\#mission` conserve `#mission` dans le titre, `\/p vendredi` conserve `/p vendredi`. Un antislash littéral s'écrit `\\`. L'édition réaffiche les dates en `YYYY-MM-DD` et échappe les tokens du titre pour ne pas les transformer en métadonnées. Les règles détaillées figurent dans [F2](features/F2-organiser-et-retrouver-ses-taches.md).

---

# 8. Aide contextuelle

L’utilisateur ne doit pas avoir à mémoriser immédiatement toutes les commandes.

Depuis la fenêtre principale, la zone de saisie ou d’édition, `F1` permet d’afficher ou de fermer une aide compacte.

Cette aide présente notamment :

`/p` — planifier une tâche

`/d` — ajouter une deadline

`#tag` — ajouter un tag

L’aide doit pouvoir être ouverte et fermée sans utiliser la souris.

`Échap` peut notamment permettre de la fermer.

---

# 9. Vue principale

Le raccourci d’affichage ouvre un panneau léger.

Une barre dédiée au-dessus de l'en-tête permet de déplacer la fenêtre principale en
maintenant le bouton principal de la souris et en faisant glisser. Elle reste disponible
dans l'historique et les paramètres affichés dans cette même fenêtre.

Le panneau présente une liste continue, sans section ni en-tête de regroupement.
Chaque tâche affiche sa case, son titre, puis ses tags, sa date prévue et sa deadline
lorsqu'ils sont renseignés. Une tâche avec seulement un titre reste suffisante.

Le tri applique le premier critère satisfait, dans cet ordre :

1. **Retard** : tâche non terminée dont la deadline est strictement passée, quelle que soit sa date prévue.
2. **Aujourd'hui** : date prévue aujourd'hui ou passée, sans retard de deadline.
3. **Demain** : date prévue le lendemain calendaire, week-end compris.
4. **Pas de date** : aucune date prévue, même avec une deadline aujourd'hui ou future.
5. **Plus tard** : autre date prévue future.

Ces noms expliquent le tri ; ils ne sont pas des sections affichées. À rang égal,
l'ordre du fichier, ajustable manuellement, est conservé. Aucun tri supplémentaire
par date prévue ou deadline n'est imposé à l'intérieur d'un rang.

Une date prévue passée reste visible et n'est pas réécrite. Une deadline future seule
ne planifie pas la tâche. Un compteur global distingue les tâches à faire et terminées
aujourd'hui dans la vue filtrée. Une liste vide affiche « Aucune tâche » ; une recherche
ou un filtre sans correspondance affiche « Aucun résultat ».

---

# 10. Absence de doublons

Chaque tâche apparaît une seule fois dans la liste, y compris si elle possède une date
prévue et une deadline. Le tri et les indications se recalculent lorsque la date locale
change, sans modification automatique des dates enregistrées.

---

# 11. Deadlines

Une deadline future éloignée doit rester discrète.

Exemple :

une tâche dont la deadline est dans deux semaines peut simplement afficher :

`échéance 14 oct.`

Elle ne doit pas prendre visuellement le dessus sur les tâches quotidiennes.

Lorsque la deadline arrive aujourd’hui ou très prochainement, son affichage peut devenir plus visible.

Une échéance demain porte le libellé « Échéance demain » accompagné de sa date,
en ambre foncé sur fond ambre clair. Cette indication dépend de la deadline, pas de la date prévue.

Lorsque la deadline est dépassée, la tâche remonte au rang Retard et porte un indicateur
« En retard » rouge sur sa ligne. Aucune section dédiée n'est créée.

---

# 12. Tâches longues ou « de fond »

Le produit ne possède pas de type particulier « tâche de fond ».

Une tâche longue reste une tâche classique.

Exemple :

`Améliorer la documentation de la cellule architecture`

Elle peut simplement recevoir des tags tels que :

`#fond`

`#objectif-mission`

Sans date prévue, elle reste visible au rang **pas de date**, sauf retard de deadline.

L'utilisateur peut la travailler immédiatement ou la reporter en lui attribuant une date
prévue future. Retirer cette date la remet au rang **pas de date**, sauf retard de deadline.

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

Une tâche terminée encore visible dans le panneau quotidien peut être décochée. Elle redevient
alors « À faire » et sa date de fin est retirée. L'historique reste une vue consultative.

La complétion conserve le titre, la date prévue, la deadline et les tags. La tâche
terminée reste dans la liste, au rang de sa date prévue (aujourd'hui ou passée, demain,
pas de date, plus tard), en ignorant le critère Retard. Sa deadline ne porte plus de
signal de retard ni d'imminence. Sans date prévue, elle est donc au rang pas de date.

À partir du lendemain, elle disparaît de la vue principale.

Elle reste néanmoins conservée dans un historique.

Lorsqu'une tâche est cochée directement dans un fichier de tâches externe sans date de fin, l'application considère qu'elle vient d'être cochée au moment où elle observe ce changement. Elle la traite comme une tâche terminée ce jour-là. La date observée est inscrite dans le fichier lors de la prochaine écriture effectuée par l'application, sans réécriture spontanée du fichier après l'édition extérieure.

Si l'application redémarre avant cette écriture, la date réelle du cochage ne peut pas être retrouvée : la tâche est de nouveau considérée comme venant d'être cochée au moment de son observation au démarrage. Dans ce seul cas, sa visibilité le jour même et son retrait le lendemain se calculent à partir de cette nouvelle observation.

---

# 15. Historique

Une vue secondaire permet de consulter les tâches terminées.

Cette vue n’est pas centrale dans l’usage quotidien.

Elle permet notamment de retrouver ce qui a été réalisé les jours précédents.

---

# 16. Réorganisation

Les tâches peuvent être réordonnées manuellement entre tâches de même rang de tri.

L’ordre représente implicitement leur importance.

Interaction clavier proposée :

`Ctrl + ↑`

déplace la tâche vers le haut.

`Ctrl + ↓`

déplace la tâche vers le bas.

Un glisser-déposer à la souris peut être proposé en complément.

Les boutons « Monter » et « Descendre » de la tâche active sélectionnée permettent le
même déplacement à la souris ; ils sont désactivés aux bornes du rang de tri. Ces actions sont placées sous la liste et concernent la sélection. Les tâches
terminées et les lignes masquées par un filtre gardent leurs positions.

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

Pour l'organisation F2 : ↑/↓ sélectionnent une tâche, Entrée l'édite, `Ctrl+F` recherche, Tab/Maj+Tab puis Entrée activent un tag et `Ctrl+↑`/`Ctrl+↓` réordonnent. F1 ouvre l'aide dans la capture ou l'édition. Échap ferme d'abord l'aide, l'ajout intégré, l'édition ou la recherche active avant le panneau. Espace bascule la tâche quotidienne sélectionnée entre « À faire » et « Terminée ». Les commandes d'historique restent définies par F3.

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

La [maquette A validée le 26 octobre 2026](designs/liste-sans-sections/a-triee.png)
constitue la référence de présentation du panneau : liste sans sections, métadonnées
sous le titre, indicateur rouge de retard sur la ligne, échéance demain en ambre et
actions Monter/Descendre sous la liste. [Sources et décisions](designs/liste-sans-sections/README.md).

La direction **Brume, palette Indigo**, choisie le 14 octobre 2026, reste applicable :
fond gris bleuté clair, typographie sans sérif, lignes compactes, repères indigo pour
les actions et la sélection clavier, tags discrets. Les couleurs sont définies dans
[F5](features/F5-consolider-interface.md). La maquette ne remplace pas les règles
fonctionnelles, notamment la syntaxe explicite d'édition.

Le logo **« Lignes et coche »**, proposition 02 choisie le même jour, associe trois lignes indigo à une coche intégrée à la dernière. Ses [déclinaisons](designs/logos/propositions.md) servent de référence pour la marque et les icônes de l’application.

---

# 27. Principe directeur du MVP

À chaque ajout fonctionnel, appliquer la question suivante :

**« Est-ce que cette fonction m’aide à noter ou à voir mes tâches plus facilement, ou est-ce qu’elle m’oblige à davantage gérer mon gestionnaire de tâches ? »**

Si elle augmente principalement la gestion du système, elle doit probablement rester hors du MVP.
