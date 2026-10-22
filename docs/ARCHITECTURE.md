# Architecture — Gestionnaire de tâches local

> **Statut : Architecture Tauri/Vue validée — migration en cours**
>
> Ce document décrit la cible architecturale validée pour le MVP. L'état de son implémentation et les lots autorisés figurent dans [le suivi de migration](migration-tauri-vue.md).
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
- ni de Tauri ou Vue ;
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

La vérification et le téléchargement des mises à jour applicatives utilisent GitHub quand
une connexion est disponible. Cette dépendance est isolée de la gestion locale des tâches.

---

### 2.4 Séparation domaine / présentation

Les concepts intrinsèques d'une tâche appartiennent au domaine.

Les notions suivantes sont des projections de présentation :

- En retard ;
- En cours ;
- Prochain jour ouvré ;
- Plus tard ;
- Historique ;
- recherche ;
- filtre par tag.

Ces catégories ne sont jamais persistées.

« En cours » regroupe les tâches sans date prévue et celles prévues aujourd'hui ou
auparavant. Le retard de deadline prime sur ce regroupement. Les états du domaine restent
« À faire » et « Terminée » ; le classement n'ajoute ni état ni date persistée.

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

### Mises à jour applicatives

`src-tauri/src/updates.rs` porte la vérification et le téléchargement signés via le plugin
Tauri updater, sans dépendance du domaine ou du repository. Un réveil toutes les 24 heures
et une vérification au démarrage consultent le manifeste `latest.json` de la dernière release
stable GitHub `zemoa/Mystline`. La version SemVer courante provient de la configuration Tauri ;
seule une version supérieure est proposée. La clé publique embarquée valide la signature et
la version signée de l’installateur. La clé privée reste dans les secrets de publication.

Le téléchargement est automatique et conservé dans un fichier temporaire privé. Vue expose
l’état via un snapshot et l’événement séquencé `update-status`. L’installation et le
redémarrage sont une intention explicite depuis les paramètres ; le backend refuse une
capture ouverte ou une opération de persistance en cours et verrouille le runtime pendant
l’installation. Les erreurs réseau restent séparées des diagnostics du repository.

L’updater est actif dans les distributions Windows et Linux AppImage en release. Les paquets
`.deb` et les builds debug n’effectuent pas ces vérifications. La pipeline signe les artefacts,
fusionne les entrées des deux plateformes dans `latest.json`, puis publie la release complète.

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

### 2.10 Instance applicative unique

Une seule instance applicative est autorisée par session utilisateur. Un second lancement quitte immédiatement et silencieusement, via le plugin Tauri `single-instance`.

Un cœur Rust résident détient la configuration, l'application, le domaine, le repository et le watcher. Il orchestre les intégrations desktop et les fenêtres Tauri. Les WebViews peuvent utiliser des processus supplémentaires : l'unicité applicative ne signifie pas un seul processus OS.

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

### UI et hôte desktop

**Tauri 2 + Vue 3 + Vite + TypeScript**, avec pnpm, CSS simple et état réactif Vue.

Une application Vue commune rend la capture rapide, le panneau quotidien, l'historique et les paramètres. Le frontend porte l'affichage, les brouillons et les interactions ; le domaine, les commandes, le parsing et les projections restent en Rust.

La présentation partage une palette claire, des métriques d'espacement, des contrôles compacts et des icônes SVG embarquées. Cette couche visuelle ne parse pas le Markdown et ne porte aucune règle du domaine. La direction graphique reste celle de la maquette et de F5.

Le lot 1 a fourni trois surfaces prototypes. Le portage des fonctionnalités F0–F5 et le contrat de commandes de tâches ont été autorisés le 13 octobre 2026 ; leur avancement est suivi dans le document de migration.

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
│                 PRÉSENTATION — RUST / VUE                     │
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
Vue ──→ Commandes Tauri ──→ Runtime Rust ──→ Application
                                │                 │
Intégration desktop ────────────┘          ┌──────┴──────┐
                                          ▼             ▼
                                        Domaine     Repository
```

Interdictions importantes :

```text
UI      ✗→ Markdown directement
Domain  ✗→ Tauri / Vue
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
   ├── réouverture depuis le panneau quotidien
   ↓
Historique calculé
   ↓
Purge éventuelle
```

Hors MVP :

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
ReopenTask
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

L'implémentation F2 regroupe le parseur dans `presentation::input` et les projections dans `presentation::panel`. `parse_input(text, today)` reçoit une date locale explicite, évaluée lors de la validation ; `format_edit(task)` réaffiche les dates absolues et échappe les tokens littéraux du titre. Ces fonctions sont testables sans fenêtre ni horloge système. L'égalité des tags sans casse et leur grammaire appartiennent au domaine et sont partagées avec le codec Markdown.

L'entrée structurée `application::TaskInput` contient `title`, `planned`, `deadline` et `tags`. `create_task_from_input` crée une tâche ; `update_task` remplace ces quatre données sans modifier son état ; `reorder_tasks` permute deux positions. L'action `create_task` par titre littéral est conservée. Les actions portant sur une tâche existante reçoivent la révision du snapshot et ses positions physiques, puis passent par le commit atomique du repository. La présentation choisit les voisins visibles dans la même section ; le repository ne connaît ni section ni filtre.

---

## 9. Présentation Rust, runtime et UI Vue

### 9.1 Frontières

`presentation::input` et `presentation::panel` conservent en Rust le parsing, les dates, les sections, l'historique, la recherche et les filtres. Les règles sont testables sans fenêtre, avec des dates injectées.

`src/runtime/task_store.rs` détient le snapshot optimiste et le snapshot confirmé. `src/runtime/coordinator.rs` sérialise les opérations et conserve les créations en FIFO pendant une écriture ou un rechargement. Les protections de génération, de séquence et de révision sont indépendantes de Tauri.

`src-tauri/` orchestre les fenêtres, l'intégration système et le runtime. `frontend/` contient les composants Vue et le style partagé. Les surfaces ne détiennent jamais l'unique copie de l'état confirmé.

### 9.2 Échanges entre les surfaces et le cœur

Le portage fonctionnel utilise `src-tauri/src/tasks.rs` pour les DTO et les intentions de présentation. `task_view(request)` reçoit la recherche, le tag et le mode quotidien/historique, et renvoie les lignes projetées par Rust : position physique, section, titre, tags, dates, état, tonalité de deadline et brouillon d'édition échappé. La réponse contient aussi la date locale, le prochain jour ouvré, l'occupation du stockage, la génération de source, la révision et une version d'observation. Vue ne recalcule ni les sections ni les filtres métier.

`task_create`, `task_update`, `task_toggle` et `task_reorder` attendent la confirmation de persistance ou retournent une erreur lisible. Les actions sur une tâche existante fournissent la génération et la révision observées. Le backend calcule les voisins visibles pour le réordonnancement et vérifie que les intentions de complétion correspondent à la vue quotidienne. `task_reorder` renvoie la position cible après confirmation afin que la sélection suive la tâche déplacée ; l'absence de voisin ne provoque aucune écriture. Vue réserve l'édition aux tâches actives du quotidien. La création associe son attente à la session et à l'activation de la surface d'origine.

L'événement `task-changed` demande une relecture de la projection, y compris lors d'une proposition optimiste, de sa confirmation, d'un retour à l'état confirmé ou d'un changement de jour. Le frontend s'abonne avant sa lecture initiale et ignore les réponses de lectures dépassées. Une surface conservée relit l'état à chaque réouverture. `desktop-status`, également versionné, reste le canal des diagnostics et de la configuration ; les réglages passent par les commandes dédiées `settings_source`, `settings_autostart` et `settings_shortcut`. `settings_choose(create)` ouvre un dialogue natif via le plugin Tauri `dialog` et renvoie le chemin choisi ou l'annulation ; la validation et l'activation restent dans `settings_source`.

La cible conserve une révision runtime et la génération de source. Une action obsolète ne peut pas être appliquée à une autre ligne ou à un autre fichier. Chaque nouvelle fenêtre doit obtenir l'état courant et recevoir les mises à jour, sans dépendre d'événements émis avant son chargement.

### 9.3 Mise à jour optimiste

L'affichage des modifications reste optimiste. Le snapshot confirmé et le fichier ne changent qu'après réussite du commit atomique. Une erreur resynchronise l'affichage sur l'état confirmé et conserve les brouillons non enregistrés.

Une seule opération de repository est exécutée à la fois. Une création reçue pendant une écriture ou un rechargement est conservée en FIFO, puis rejouée sur le snapshot confirmé suivant. Elle n'est jamais refusée uniquement parce que le stockage est occupé. Un changement de source attend que les créations reçues soient traitées.

La capture disparaît seulement après confirmation de son enregistrement. Les décisions de fermeture et les retours d'opération distinguent chaque session et activation de fenêtre pour ne pas affecter une surface rouverte.

### 9.4 Navigation et journée locale

Vue rend les interactions clavier documentées dans F1–F3 : sélection, recherche, édition, complétion, historique, aide et réordonnancement. Une touche traitée par un champ ou un bouton ne déclenche pas une seconde action de liste. Les scénarios DOM montent le composant et vérifient ces interactions avec un adaptateur IPC contrôlé ; la validation native et les limites par plateforme figurent dans le suivi de migration.

Le cœur attend le prochain début de journée locale avec `next_day_delay`, puis émet la date et réarme son délai, sans polling permanent. Les projections se recalculent à partir de cette date ; le changement de jour ne modifie pas le fichier. La date est aussi revérifiée lors de la consultation de l'état d'une nouvelle surface.

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

Dans `tasks.md`, `#` et `@` en début de token (début du titre ou après un blanc) désignent respectivement un tag et une métadonnée, sauf s'ils sont précédés d'un antislash sur une ligne portant `@title-escaped(1)`. Un `#` non échappé sans nom de tag est invalide. Le codec échappe ces caractères lorsqu'ils appartiennent littéralement au titre et double les antislashs du titre ; il ajoute alors `@title-escaped(1)` à la ligne. Exemples de titres et de leur écriture dans le fichier :

| Titre de la tâche | Fragment Markdown du titre |
| --- | --- |
| `#architecture` | `\#architecture` |
| `#` | `\#` |
| `\#mission` (avec un antislash dans le titre) | `\\#mission` |
| `Relire @deadline(2026-10-05)` | `Relire \@deadline(2026-10-05)` |

Pour ces exemples, la ligne complète contient aussi `@title-escaped(1)`, par exemple `- [ ] \#architecture @title-escaped(1)`. Une ligne existante sans ce marqueur conserve ses antislashs littéraux (y compris `\\serveur` et `\#mission`) ; à la prochaine écriture applicative, elle reçoit si nécessaire le nouveau codage et son marqueur sans changer le titre. Un `#mission` non échappé dans un fichier externe reste un vrai tag. Les espaces internes du titre sont conservés ; lors de la création par titre seul, seuls les espaces aux deux extrémités sont retirés. Cette convention concerne le **fichier Markdown** : elle ne préjuge pas de l'interprétation future des commandes saisies dans l'interface (F2).

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

Il reconnaît les tokens `#tag` et `@...` non échappés comme métadonnées. Sur une ligne marquée `@title-escaped(1)`, il décode `\#` et `\@` en début de mot du titre ainsi que `\\` en antislash littéral ; sans ce marqueur, les antislashs du titre sont conservés tels quels. La sérialisation échappe les caractères réservés du titre et ajoute ce marqueur si un échappement est nécessaire. La relecture d'un fichier écrit par l'application retrouve donc le titre original sans transformer un fragment littéral en tag ni modifier les antislashs d'un ancien fichier.

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

Les transitions du watcher et du repository sont journalisées dans le sous-dossier `logs/` du
répertoire de configuration, que le watcher de `tasks.md` ne surveille pas. Les événements contiennent le type d'opération, la génération, la
séquence, la révision, les durées d'attente et d'exécution et l'erreur éventuelle, mais jamais le
titre ni les métadonnées d'une tâche.

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

Le chemin choisi est résolu et canonisé après ouverture valide. Le repository actif, le watcher et la configuration utilisent le même chemin absolu, y compris lorsqu'un lien symbolique a été choisi. Au démarrage, la source configurée est également canonisée après sa création ou sa validation, avant l'ouverture du runtime et du watcher.

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

L'autostart est confié au plugin Tauri `autostart` dans `src-tauri/`. Le nom `mystline` reprend l'entrée Linux existante ; sous Windows le nom de valeur du registre est insensible à la casse. Le réglage dans `config.toml` reste la référence.

---

## 22. Raccourcis globaux

Deux raccourcis globaux sont configurables :

- capture rapide ;
- affichage/masquage du panneau.

Leurs combinaisons par défaut sont `Ctrl+Shift+Espace` et `Ctrl+Shift+T`.

Sous Wayland, `Ctrl+!` et `Ctrl+ù` sont acceptés : la validation et l'enregistrement
utilisent les keysyms XKB `exclam` et `ugrave`, sans déduire de position physique
sur le clavier. Les noms `exclam` et `ugrave` sont également acceptés et traités
comme équivalents pour refuser une attribution identique aux deux actions.
La configuration conserve la saisie utilisateur. X11 et Windows conservent la
syntaxe du parseur natif `global-hotkey`.

Sur Linux Wayland, l'enregistrement passe par le portail XDG Global Shortcuts : le compositeur peut demander une autorisation, refuser un raccourci ou proposer une combinaison différente. L'état actif reflète les raccourcis effectivement accordés. Sur X11 et Windows, l'enregistrement utilise le plugin Tauri `global-shortcut`. Sous Wayland, l'adaptateur `ashpd` existant est conservé. Le tray reste l'accès de repli si le portail ou l'un des raccourcis est indisponible.

L'adaptateur Wayland déclare l'identifiant applicatif `local.mystline`, dérivé de l'identifiant Tauri en retirant son suffixe `.desktop` réservé aux lanceurs dans KGlobalAccel, auprès du registre du portail, sur une connexion D-Bus dédiée, avant toute création de session. Hors sandbox applicatif, il crée si nécessaire une entrée `.desktop` locale avec `NoDisplay=true` dans le dossier XDG des applications : le portail exige cette métadonnée pour identifier les lancements depuis les sources. Une entrée existante est conservée. Les deux actions sont liées ensemble dans une seule session ; sa fermeture précède tout renouvellement. Une modification invalide les événements de l'ancienne session pour les deux actions. Les identifiants d'actions incluent la combinaison normalisée afin qu'une nouvelle combinaison fasse l'objet d'une nouvelle demande, plutôt que de restaurer une ancienne attribution du compositeur. Une action présente dans la réponse mais sans `trigger_description` reste inactive.

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

Le clic principal sur son icône déclenche une demande d'ouverture du panneau, distincte de l'affichage/masquage demandé par le raccourci ou l'entrée du menu. L'UI ouvre le panneau ou redonne le focus à sa fenêtre existante. Le clic secondaire ouvre le menu. L'API tray Tauri est utilisée sous Windows ; Linux conserve le backend KSNI, car l'API tray standard Tauri n'y fournit pas les clics. Les deux adaptateurs transmettent leurs actions au même canal d'événements desktop, sans polling.

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

Le cœur Tauri démarre sans fenêtre visible. La capture et le panneau sont créés à leur première ouverture, puis conservés et masqués à la fermeture. Les paramètres et l'historique sont des vues du panneau : les paramètres remplacent son contenu dans la même fenêtre native.

La stratégie de conservation et masquage après première ouverture a été validée pour D7 après le test comparatif du lot 1. Elle privilégie les réouvertures et ne crée pas de WebView avant le premier appel de sa surface.

Une réservation empêche plusieurs ouvertures concurrentes de créer des fenêtres identiques. La session identifie la fenêtre native ; une activation distincte identifie chaque réouverture après masquage. Une demande sur une fenêtre déjà visible lui redonne le focus sans effacer la saisie. Le frontend signale son chargement initial ; une fermeture reçue avant ce signal empêche son affichage tardif. Les opérations de visibilité sont ordonnées sur le thread principal.

Le bouton de fermeture, Échap et une fermeture native masquent la fenêtre. Le frontend abandonne les états transitoires non enregistrés et redonne le focus à la capture lors de sa réouverture. Les activations permettent d'ignorer une fermeture retardée provenant d'une ancienne ouverture. Le basculement du panneau dépend de sa visibilité demandée, pas de l'existence de sa fenêtre.

La fermeture de la dernière surface ne quitte jamais l'application. Le plugin d'instance unique et les intégrations desktop restent actifs. Seule l'action explicite « Quitter » arrête le cœur et détruit les fenêtres conservées.

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
libération de l'instance Tauri
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
raccourci → saisie → Entrée → disparition dès confirmation de l'enregistrement
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
nouveau snapshot runtime
      │
      ▼
Vue actualise les vues
```

Aucune fusion ligne par ligne n'est tentée.

---

## 28. Flux principal — Completion

```text
TaskPanel
   ↓
CompleteSelected
   ↓
Snapshot runtime optimiste :
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

## 30. Structure de projet

```text
Cargo.toml                 bibliothèque Rust existante
package.json               commandes frontend et desktop depuis la racine
pnpm-workspace.yaml        workspace incluant frontend/
pnpm-lock.yaml             dépendances JavaScript verrouillées
src/
├── domain.rs
├── application.rs
├── config.rs
├── repository/
├── presentation/
└── runtime/
    ├── coordinator.rs
    └── task_store.rs
frontend/
├── package.json
├── vite.config.ts
└── src/                   Vue / TypeScript / CSS
src-tauri/
├── Cargo.toml             hôte desktop, dépendant de la bibliothèque racine
├── tauri.conf.json
├── capabilities/
└── src/
    ├── main.rs
    ├── desktop.rs
    ├── linux.rs
    ├── surfaces.rs
    └── tasks.rs            projections et intentions IPC
tests/                     parcours persistés F2/F3
```

Les manifests Rust ont chacun leur lockfile. Aucun workspace Cargo ni crate de domaine supplémentaire n'est introduit. Le contrat de tâches est dans `src-tauri/src/tasks.rs`. Le frontend partage `Icon.vue` et `SyntaxHelp.vue` entre ses surfaces. `frontend/tests/` vérifie les interactions DOM et `tests/migration_runtime.rs` les protections de coordination.

---

## 31. Ce qui reste hors architecture v1

Les sujets suivants relèvent de la conception détaillée ou de l'implémentation :

- crates Rust exactes ;
- versions mineures exactes de Tauri/Vue ;
- format précis des erreurs UI ;
- délai de coalescence du watcher ;
- stratégie UX détaillée de purge ;
- composants visuels détaillés ;
- styles, couleurs et composants ;
- seuils chiffrés CPU/RAM/latence ;
- format détaillé des logs ;
- formats de distribution et installation des prérequis WebView ;
- mesures complémentaires sur les interfaces fonctionnelles et les autres plateformes.

Ils ne doivent pas remettre en cause les frontières définies ici sans nouvelle décision architecturale.

---

## 32. Résumé des décisions

| Sujet | Décision |
|---|---|
| Langage | Rust |
| UI | Vue 3 / TypeScript, hôte Tauri 2 |
| Distribution | Releases GitHub créées sur les tags `X.Y.Z`, avec `.deb` et `.AppImage` Linux, `.msi` et NSIS Windows |
| Processus | Cœur Rust résident et processus WebView possibles |
| Instance | Une seule par session |
| Stockage | `tasks.md` |
| Format | Markdown structuré |
| Historique | Vue calculée |
| Recherche | Locale côté présentation |
| Filtres | Côté présentation |
| Sections du panneau | Côté présentation |
| Cache | Repository |
| Snapshot UI | Runtime Rust, rendu Vue |
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
| Fermeture fenêtre | Masquage, fenêtre conservée après première ouverture |
| Suppression | Purge de maintenance |
| Reopen | Depuis le panneau quotidien uniquement |
| Cloud | Aucun |
| Compte | Aucun |
| Dépendance Internet | Aucune pour les tâches ; GitHub pour les mises à jour applicatives |

---

## 33. Principe directeur

Lorsqu'une nouvelle décision technique apparaît, vérifier :

> **Est-ce que cette complexité améliore réellement la capture, la consultation, la robustesse ou la maintenabilité de l'application ?**

Si la réponse est non, ne pas l'introduire.
