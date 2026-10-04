# Architecture — Gestionnaire de tâches local

> **Statut : Architecture v1**
>
> Ce document décrit les décisions architecturales validées pour le MVP.
> Il couvre les principes, les frontières, les flux principaux et les responsabilités.
> Certains choix d'implémentation détaillés (crates exactes, délais de debounce, etc.) ne sont pas figés ici.

---

## 1. Objectifs architecturaux

L'application est un gestionnaire de tâches personnel, local, rapide et minimaliste.

Les objectifs architecturaux principaux sont :

- capture immédiate d'une tâche ;
- consultation rapide des tâches ;
- fonctionnement local, sans compte ni cloud ;
- utilisation majoritairement au clavier ;
- faible empreinte système ;
- stockage lisible par un humain et exploitable par des outils externes ;
- architecture suffisamment simple pour rester maintenable ;
- aucune sophistication technique sans besoin concret.

---

## 2. Principes architecturaux

### 2.1 Indépendance du domaine

Le domaine ne dépend :

- ni de l'interface graphique ;
- ni d'Iced ;
- ni du système d'exploitation ;
- ni du format Markdown ;
- ni du système de fichiers.

Il contient uniquement les concepts et invariants intrinsèques aux tâches.

---

### 2.2 Une seule source de vérité persistée

Le fichier `tasks.md` est la source de vérité persistée.

Le repository conserve un cache mémoire pour l'exécution, et l'UI conserve un snapshot destiné à l'affichage, mais ces représentations ne remplacent pas la persistance.

```text
tasks.md
   ↓
Repository cache
   ↓
UI snapshot
```

---

### 2.3 Offline par conception

L'application :

- ne dépend pas d'Internet ;
- ne possède pas de compte utilisateur ;
- ne dépend d'aucun service cloud ;
- ne synchronise rien à distance.

Toutes les données restent sur la machine.

---

### 2.4 Séparation domaine / présentation

Les concepts intrinsèques d'une tâche appartiennent au domaine.

Les notions suivantes sont des projections de présentation :

- En retard ;
- Aujourd'hui ;
- Prochain jour ouvré ;
- Toutes les tâches ;
- Historique ;
- recherche ;
- filtre par tag.

Ces catégories ne sont jamais persistées.

---

### 2.5 Intégration desktop isolée

Les responsabilités liées au système d'exploitation sont isolées du domaine et de l'application :

- raccourcis globaux ;
- autostart ;
- tray ;
- unicité du processus ;
- cycle de vie des fenêtres.

L'intégration desktop déclenche des actions, mais ne porte aucune logique métier.

---

### 2.6 Keyboard-first

L'architecture doit permettre une utilisation principalement au clavier :

- capture ;
- ouverture/fermeture du panneau ;
- navigation ;
- sélection ;
- completion ;
- édition ;
- réordonnancement ;
- recherche ;
- filtre ;
- aide.

Les raccourcis clavier déclenchent des intentions explicites plutôt que de contenir de la logique métier.

---

### 2.7 Le stockage ne dicte pas le domaine

Le domaine manipule des `Task`.

Le format Markdown est un détail de persistance.

La transformation entre les deux est centralisée dans un codec dédié.

---

### 2.8 Minimalisme architectural

Ne pas introduire de mécanisme complexe sans besoin concret :

- pas de CQRS cérémoniel ;
- pas de bus d'événements complexe ;
- pas de microservices ;
- pas d'IPC sans nécessité ;
- pas de base de données sans besoin réel ;
- pas de polling permanent.

---

### 2.9 Empreinte système minimale

L'application est résidente mais doit rester discrète :

- CPU quasi nul au repos ;
- pas de polling inutile ;
- consommation mémoire raisonnable ;
- latence faible pour la capture et l'affichage du panneau.

La simplicité de développement est privilégiée tant que ces objectifs restent respectés.

---

### 2.10 Processus unique

L'application utilise un seul processus résident.

Ce processus gère :

- l'intégration desktop ;
- l'UI ;
- l'application ;
- le domaine ;
- le repository ;
- le watcher de fichier.

Une seule instance est autorisée par session utilisateur.

Un second lancement quitte immédiatement et silencieusement.

---

## 3. Stack retenue

### Langage

**Rust**

Objectifs :

- binaire natif ;
- empreinte raisonnable ;
- bon accès au système ;
- robustesse ;
- exécution résidente adaptée.

### UI

**Iced**

Toutes les surfaces graphiques utilisent Iced :

- capture rapide ;
- panneau principal ;
- historique ;
- paramètres.

Une seule technologie UI est utilisée pour éviter la duplication.

---

## 4. Vue d'ensemble

```text
┌─────────────────────────────────────────────────────────┐
│                    DESKTOP / OS                         │
│                                                         │
│  Global shortcuts   Tray   Autostart   Single instance  │
│  Window lifecycle                                      │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                 PRÉSENTATION — ICED                     │
│                                                         │
│  App                                                    │
│  ├── QuickCapture                                      │
│  ├── TaskPanel                                         │
│  ├── History                                           │
│  └── Settings                                          │
│                                                         │
│  Input parser (/p, /d, #tag)                           │
│  Recherche / filtres                                   │
│  Sections calculées                                    │
│  Navigation clavier                                    │
│  Snapshot UI optimiste                                 │
└──────────────────────────┬──────────────────────────────┘
                           │ intentions structurées
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     APPLICATION                         │
│                                                         │
│  CreateTask                                             │
│  UpdateTask                                             │
│  CompleteTask                                           │
│  ReorderTask                                            │
│  PurgeTasks                                             │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                       DOMAINE                           │
│                                                         │
│  Task                                                   │
│  Concepts et invariants intrinsèques                    │
│  Propriétés calculées runtime                           │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│              REPOSITORY / INFRASTRUCTURE                │
│                                                         │
│  TaskRepository                                         │
│  ├── cache mémoire                                      │
│  ├── revision runtime                                   │
│  ├── fingerprint fichier                                │
│  └── commit atomique                                    │
│                                                         │
│  MarkdownTaskCodec                                      │
│  File watcher                                           │
│  Configuration TOML                                     │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
                       tasks.md
```

---

## 5. Règle de dépendance

```text
Desktop ───────┐
               ▼
              UI
               │
               ▼
          Application
               │
          ┌────┴────┐
          ▼         ▼
       Domain     Ports
                    ▲
                    │
             Infrastructure
```

Interdictions importantes :

```text
UI      ✗→ Markdown directement
Domain  ✗→ Iced
Domain  ✗→ filesystem
Desktop ✗→ TaskRepository directement
```

---

## 6. Domaine

### 6.1 Modèle `Task`

Le modèle runtime peut contenir des données persistées et des données calculées.

```text
Task
│
├── données persistées
│   ├── title
│   ├── plannedDate?
│   ├── deadline?
│   ├── tags[]
│   ├── state
│   └── completedDate?
│
└── données calculées
    ├── isOverdue
    ├── displaySection
    ├── effectiveCompletionDate
    └── autres propriétés utiles
```

Les propriétés calculées :

- peuvent exister en mémoire ;
- ne sont jamais une source de vérité ;
- ne sont pas sérialisées directement.

---

### 6.2 Ordre des tâches

L'ordre physique des tâches dans `tasks.md` représente leur ordre relatif.

Aucun champ `priority` ou `order` explicite n'est nécessaire dans le fichier.

Le réordonnancement revient à modifier l'ordre des lignes.

---

### 6.3 Cycle de vie

MVP :

```text
Création
   ↓
À faire
   ├── modification
   ├── planification
   ├── changement de tags
   └── réordonnancement
   ↓
Terminée
   ↓
Historique calculé
   ↓
Purge éventuelle
```

Hors MVP :

- réouverture d'une tâche terminée ;
- suppression quotidienne explicite.

La suppression est traitée comme une opération de maintenance via la purge.

---

## 7. Couche Application

La couche Application expose des actions structurées.

### Commands

```text
CreateTask
UpdateTask
CompleteTask
ReorderTask
PurgeTasks
```

### Lecture

L'UI travaille sur le snapshot courant.

Les opérations suivantes restent côté présentation :

- recherche ;
- filtre par tag ;
- regroupement en sections ;
- historique ;
- calcul de la vue du jour.

---

## 8. Parsing des saisies utilisateur

La syntaxe utilisateur appartient à la présentation.

Exemple :

```text
Préparer le support /p jeudi /d vendredi #mission
```

est transformé en une entrée structurée :

```text
CreateTaskInput
├── title
├── plannedDate?
├── deadline?
└── tags[]
```

Puis seulement transmis à la couche Application.

Le domaine ne connaît pas :

- `/p` ;
- `/d` ;
- `#tag` en tant que syntaxe de saisie.

---

## 9. Architecture UI Iced

### 9.1 État modulaire

Chaque fonctionnalité importante possède son propre état et ses propres messages.

```text
App
├── QuickCapture
│   ├── State
│   ├── Message
│   ├── update()
│   └── view()
│
├── TaskPanel
│   ├── State
│   ├── Message
│   ├── update()
│   └── view()
│
├── History
│   ├── State
│   ├── Message
│   ├── update()
│   └── view()
│
└── Settings
    ├── State
    ├── Message
    ├── update()
    └── view()
```

La racine Iced coordonne ces modules.

Exemple :

```rust
enum AppMessage {
    QuickCapture(QuickCaptureMessage),
    TaskPanel(TaskPanelMessage),
    History(HistoryMessage),
    Settings(SettingsMessage),
}
```

---

### 9.2 Snapshot UI

`AppState` possède le snapshot affiché.

Les sous-modules UI ne modifient pas directement le repository.

Ils émettent des intentions vers la racine.

```text
TaskPanel
   ↓
Message
   ↓
App::update()
   ↓
Application
   ↓
Repository
```

---

### 9.3 Mise à jour optimiste

L'UI est optimiste.

Exemple :

```text
Utilisateur coche une tâche
        ↓
AppState mis à jour immédiatement
        ↓
Iced redessine
        ↓
CompleteTask
        ↓
Repository
```

En cas de réussite :

```text
Repository
   ↓
nouveau snapshot confirmé
   ↓
AppState
```

En cas d'échec :

```text
Repository
   ↓
dernier snapshot confirmé
   ↓
AppState se resynchronise
```

---

## 10. Persistance Markdown

### 10.1 Fichier canonique

Un seul fichier contient toutes les tâches :

```text
tasks.md
```

Exemple :

```md
- [ ] Préparer le support #mission @planned(2026-10-01) @deadline(2026-10-02)
- [ ] Vérifier nouvelle version API #architecture
- [x] Répondre à Paul @completed(2026-09-30)
- [ ] Améliorer documentation #fond #mission
```

Le fichier est :

- lisible par un humain ;
- modifiable par une édition extérieure ;
- compréhensible par des outils ;
- exploitable par une IA ;
- versionnable facilement.

---

### 10.2 Format

Forme générale :

```text
- [ ] <titre> [#tags...] [@planned(YYYY-MM-DD)] [@deadline(YYYY-MM-DD)]
- [x] <titre> [#tags...] [@planned(YYYY-MM-DD)] [@deadline(YYYY-MM-DD)] [@completed(YYYY-MM-DD)]
```

Les dates persistées utilisent `YYYY-MM-DD`.

La completion ne supprime pas les métadonnées existantes : date prévue, deadline et tags restent attachés à la tâche. Les champs facultatifs peuvent être absents sur une ligne cochée.

Les sections de l'interface ne sont jamais écrites dans le Markdown.

---

### 10.3 Historique

L'historique n'est pas une section physique du fichier.

```text
tasks.md
   ↓
Task[]
   ↓
projection
   ├── tâches actives
   └── historique
```

Une tâche terminée reste physiquement dans la même liste.

---

## 11. MarkdownTaskCodec

Le parsing et la sérialisation sont centralisés.

```text
MarkdownTaskCodec

parse(text)
    -> Task[]

serialize(Task[])
    -> text
```

Aucun composant UI ne parse directement les métadonnées Markdown.

Le codec est responsable du format canonique lors des écritures initiées par l'application.

---

## 12. Éditions extérieures

`tasks.md` est une interface publique locale de l'application.

Une édition extérieure valide doit être détectée et reflétée automatiquement.

### Règle fondamentale

L'application ne normalise jamais spontanément une édition extérieure.

```text
édition extérieure
      ↓
tasks.md
      ↓
watcher
      ↓
parse
      ↓
snapshot
```

Jamais :

```text
édition extérieure
      ↓
parse
      ↓
réécriture automatique
```

Cela évite les conflits avec des outils qui sauvegardent fréquemment.

---

### 12.1 `[x]` sans `@completed`

La forme suivante est acceptée :

```md
- [x] Répondre à Paul
```

En mémoire :

```text
completed = true
completedDate = None
```

L'application utilise comme date effective le jour où elle observe cette completion pendant la session courante.

Elle ne réécrit pas automatiquement cette ligne.

Lors de la prochaine écriture initiée par l'application, les lignes cochées sans date reçoivent leur date effective avant la sérialisation. Une sauvegarde extérieure ne déclenche aucune réécriture par l'application.

Lorsqu'une completion est initiée depuis l'application, la date est persistée immédiatement :

```md
- [x] Répondre à Paul @completed(2026-10-01)
```

> **Compromis MVP :**
> une completion extérieure sans `@completed` ne possède pas de date persistante.
> Si l'application redémarre avant qu'une écriture applicative ne fixe cette date, la date effective est recalculée lors de la nouvelle session, comme précisé dans la SFG.

---

## 13. File watcher

Le watcher utilise les événements natifs du système de fichiers.

Aucun polling permanent.

```text
filesystem events
   │ │ │
   └─┴─┴───┐
           ▼
     coalescence courte
           ↓
      lecture fichier
           ↓
          parse
```

Le délai exact de coalescence relève de l'implémentation.

---

### 13.1 Fichier temporairement invalide

Une sauvegarde extérieure peut produire temporairement un fichier invalide.

Dans ce cas :

```text
version extérieure
       ↓
parse invalide
       ↓
ne pas modifier le fichier
ne pas remplacer le cache
       ↓
conserver le dernier état valide
       ↓
attendre une nouvelle modification
```

L'application ne corrige pas le fichier automatiquement.

---

## 14. Repository

Le repository possède :

```text
TaskRepository
├── cache: Task[]
├── revision: u64
└── fileFingerprint
```

Responsabilités :

```text
load()
save()
reload()
```

Le repository :

- maintient le cache mémoire ;
- sérialise via `MarkdownTaskCodec` ;
- persiste immédiatement les actions de l'application ;
- recharge après une édition extérieure ;
- protège contre les lost updates.

---

## 15. Révisions runtime

Les tâches n'ont pas d'identifiant persistant dans le Markdown.

Pour éviter qu'une action basée sur un ancien snapshot s'applique à la mauvaise ligne, chaque snapshot possède une révision runtime.

```text
Snapshot
├── revision
└── tasks[]
```

Une commande peut transporter :

```text
CompleteTask
├── revision
└── position
```

Si la révision ne correspond plus au repository :

```text
commande revision 12
repository revision 13
        ↓
commande refusée
        ↓
resynchronisation UI
```

La révision :

- n'est jamais persistée ;
- n'est pas un identifiant de tâche ;
- ne sert qu'à sécuriser la concurrence runtime.

---

## 16. Protection contre les lost updates

Avant toute écriture initiée par l'application, le repository vérifie que le fichier sur disque correspond encore à la version connue.

```text
Repository
   ↓
compare fingerprint
  /               \
identique        différent
   ↓                ↓
commit        édition extérieure
               détectée
                    ↓
                 reload
```

Une modification extérieure non encore intégrée ne doit jamais être écrasée.

---

## 17. Déduplication des événements fichier

Les écritures de l'application déclenchent elles-mêmes le watcher.

Elles sont distinguées via le contenu/fingerprint, pas via un délai arbitraire.

```text
application écrit
       ↓
fingerprint = XYZ
       ↓
watcher event
       ↓
fingerprint = XYZ
       ↓
aucune action
```

---

## 18. Persistance immédiate et atomique

Toute modification initiée par l'application est persistée immédiatement.

Exemples :

- création ;
- modification ;
- completion ;
- réordonnancement ;
- purge.

Écriture :

```text
cache proposé
    ↓
serialize
    ↓
tasks.md.tmp
    ↓
flush
    ↓
rename / replace atomique
    ↓
tasks.md
```

Le cache confirmé du repository n'est remplacé qu'après réussite du commit.

```text
nouvel état
    ↓
écriture atomique
   /             \
succès          échec
  ↓               ↓
commit cache   conserver ancien cache
revision + 1
```

---

## 19. Configuration

La configuration applicative est séparée des tâches.

Format :

```text
config.toml
```

Emplacement :

- répertoire de configuration utilisateur standard de l'OS ;
- jamais à côté de l'exécutable.

Exemple :

```toml
tasks_file = "/chemin/vers/tasks.md"
autostart = true

[shortcuts]
capture = "Ctrl+Shift+Espace"
panel = "Ctrl+Shift+T"
```

Les valeurs par défaut de capture et du panneau sont respectivement `Ctrl+Shift+Espace` et `Ctrl+Shift+T` sur Windows et Linux.

---

## 20. Emplacement de `tasks.md`

Le chemin est configurable.

L'utilisateur peut :

1. créer un nouveau fichier de tâches ;
2. utiliser un fichier existant.

### Nouveau fichier

```text
choisir emplacement
      ↓
créer tasks.md
      ↓
charger repository
      ↓
démarrer watcher
      ↓
enregistrer chemin
```

### Fichier existant

```text
choisir tasks.md
      ↓
lecture + parse
   /             \
valide          invalide
  ↓                ↓
activer         conserver source actuelle
```

Un changement de source est transactionnel.

Le nouveau chemin n'est enregistré dans `config.toml` qu'après validation.

---

### 20.1 Aucun déplacement automatique

Changer d'emplacement ne déplace jamais l'ancien fichier.

```text
ancien/tasks.md
    reste intact

nouvel emplacement
    ↓
nouveau tasks.md
```

---

### 20.2 Fichier ou répertoire disparu

Si le chemin configuré n'existe plus au démarrage :

1. recréer silencieusement les répertoires parents manquants ;
2. créer un nouveau `tasks.md` vide ;
3. poursuivre le démarrage.

Les erreurs d'accès réelles (permissions, disque inaccessible, etc.) ne sont pas assimilées à une simple absence.

---

## 21. Autostart

L'application démarre avec la session utilisateur par défaut.

Cette option est désactivable.

```toml
autostart = true
```

L'implémentation dépend de l'OS et reste confinée dans `desktop/`.

---

## 22. Raccourcis globaux

Deux raccourcis globaux sont configurables :

- capture rapide ;
- affichage/masquage du panneau.

Leurs combinaisons par défaut sont `Ctrl+Shift+Espace` et `Ctrl+Shift+T`.

La configuration conserve la valeur demandée même si l'OS refuse son enregistrement.

Runtime :

```text
Shortcut
├── configured
└── active
```

En cas de conflit :

```text
configured = true
active = false
```

L'application ne retente pas automatiquement l'enregistrement.

Nouvelle tentative uniquement :

- au démarrage ;
- lorsqu'un raccourci est modifié.

---

## 23. Tray

Le tray est une surface de contrôle système minimale.

```text
Tray
├── Capture rapide
├── Ouvrir / masquer le panneau
├── Paramètres
└── Quitter
```

Le tray ne contient :

- aucune tâche ;
- aucun compteur ;
- aucune notification ;
- aucune logique métier.

---

## 24. Cycle de vie des surfaces

Toutes les surfaces Iced sont créées au démarrage.

```text
QuickCapture
TaskPanel
Settings
```

Elles sont ensuite uniquement affichées ou masquées.

```text
fermer QuickCapture → hide
fermer TaskPanel    → hide
fermer Settings     → hide
```

La fermeture d'une surface ne quitte jamais le processus.

Seule une action explicite `Quitter` arrête l'application.

---

## 25. Arrêt du processus

Séquence logique :

```text
Quitter
   ↓
arrêt watcher
   ↓
désenregistrement raccourcis
   ↓
fermeture surfaces
   ↓
libération verrou d'instance
   ↓
fin processus
```

Les détails exacts d'ordonnancement relèvent de l'implémentation.

---

## 26. Flux principal — Capture rapide

```text
Global shortcut
      ↓
Desktop integration
      ↓
QuickCapture.show + focus
      ↓
texte utilisateur
      ↓
InputParser
      ↓
CreateTaskInput
      ↓
mise à jour UI optimiste
      ↓
Application::CreateTask
      ↓
Repository
      ↓
vérification revision + fingerprint
      ↓
serialization Markdown
      ↓
écriture atomique
      ↓
commit cache
      ↓
nouvelle revision
      ↓
snapshot confirmé
```

L'objectif UX est :

```text
raccourci → saisie → Entrée → disparition immédiate
```

---

## 27. Flux principal — Édition extérieure

```text
édition extérieure de tasks.md
          ↓
filesystem event(s)
          ↓
coalescence
          ↓
fingerprint différent
          ↓
lecture complète
          ↓
MarkdownTaskCodec
       /        \
    valide      invalide
      │            │
      ▼            ▼
nouveau cache   conserver
revision + 1    ancien cache
      │
      ▼
nouveau snapshot AppState
      │
      ▼
Iced redessine les vues
```

Aucune fusion ligne par ligne n'est tentée.

---

## 28. Flux principal — Completion

```text
TaskPanel
   ↓
CompleteSelected
   ↓
AppState optimiste :
task.completed = true
   ↓
Application::CompleteTask
   ↓
Repository
   ↓
validation revision
   ↓
validation fingerprint
   ↓
écriture :
[x] ... @completed(YYYY-MM-DD)
   ↓
commit
```

---

## 29. Flux principal — Réordonnancement

```text
TaskPanel
   ↓
ReorderTask
   ↓
réorganisation optimiste du snapshot
   ↓
Application
   ↓
Repository
   ↓
réorganisation du cache proposé
   ↓
serialization dans le nouvel ordre
   ↓
commit atomique
```

L'ordre des lignes est l'ordre métier relatif.

---

## 30. Structure de projet proposée

La structure exacte peut évoluer pendant l'implémentation, mais les frontières doivent rester visibles.

```text
src/
├── domain/
│   └── task.rs
│
├── application/
│   ├── create_task.rs
│   ├── update_task.rs
│   ├── complete_task.rs
│   ├── reorder_task.rs
│   └── purge_tasks.rs
│
├── repository/
│   ├── task_repository.rs
│   └── markdown_codec.rs
│
├── desktop/
│   ├── shortcuts.rs
│   ├── autostart.rs
│   ├── tray.rs
│   ├── single_instance.rs
│   └── file_watcher.rs
│
├── config/
│   └── config.rs
│
└── ui/
    ├── app.rs
    ├── message.rs
    ├── quick_capture.rs
    ├── task_panel.rs
    ├── history.rs
    ├── settings.rs
    ├── input_parser.rs
    └── components/
```

Cette arborescence n'impose pas un fichier par cas d'usage si cela produit trop de granularité.
Le principe important est la séparation des responsabilités, pas le nombre de fichiers.

---

## 31. Ce qui reste hors architecture v1

Les sujets suivants relèvent de la conception détaillée ou de l'implémentation :

- crates Rust exactes ;
- version exacte d'Iced ;
- format précis des erreurs UI ;
- délai de coalescence du watcher ;
- stratégie UX détaillée de purge ;
- widgets et bibliothèque visuelle ;
- styles, couleurs et composants ;
- seuils chiffrés CPU/RAM/latence ;
- format détaillé des logs ;
- packaging Windows/Linux ;
- mécanisme OS exact pour l'autostart ;
- mécanisme OS exact pour l'instance unique.

Ils ne doivent pas remettre en cause les frontières définies ici sans nouvelle décision architecturale.

---

## 32. Résumé des décisions

| Sujet | Décision |
|---|---|
| Langage | Rust |
| UI | Iced |
| Processus | Unique et résident |
| Instance | Une seule par session |
| Stockage | `tasks.md` |
| Format | Markdown structuré |
| Historique | Vue calculée |
| Recherche | Locale côté présentation |
| Filtres | Côté présentation |
| Sections du panneau | Côté présentation |
| Cache | Repository |
| Snapshot UI | AppState |
| UI | Optimiste |
| Persistance | Immédiate et atomique |
| Éditions extérieures | Temps réel via file watcher |
| Polling | Aucun |
| Identifiant de tâche persistant | Aucun |
| Protection concurrence | Revision runtime + fingerprint |
| Configuration | TOML |
| Emplacement `tasks.md` | Configurable |
| Autostart | Activé par défaut, désactivable |
| Raccourcis globaux | Configurables |
| Conflit raccourci | Config conservée, raccourci inactif |
| Retry raccourci | Démarrage ou modification uniquement |
| Tray | Minimal |
| Fermeture fenêtre | Masquage |
| Suppression | Purge de maintenance |
| Reopen | Hors MVP |
| Cloud | Aucun |
| Compte | Aucun |
| Dépendance Internet | Aucune |

---

## 33. Principe directeur

Lorsqu'une nouvelle décision technique apparaît, vérifier :

> **Est-ce que cette complexité améliore réellement la capture, la consultation, la robustesse ou la maintenabilité de l'application ?**

Si la réponse est non, ne pas l'introduire.
