# Découpage des features du MVP

Ce découpage définit le périmètre des features avant la rédaction de leurs règles de gestion. Les règles détaillées devront préciser la [spécification fonctionnelle](../SFG.md) sans la contredire. Les choix techniques déjà validés relèvent de l'[architecture](../ARCHITECTURE.md).

| ID | Feature | Périmètre |
| --- | --- | --- |
| [**F0-mise-en-place**](F0-mise-en-place.md) | **Mise en place** | Mettre en place l'application locale résidente sur Windows et Linux, le stockage des tâches, le démarrage en arrière-plan, les deux raccourcis globaux initiaux et l'accès au panneau. |
| [**F1-capturer-et-consulter-ses-taches**](F1-capturer-et-consulter-ses-taches.md) | **Capturer et consulter ses tâches** | Créer rapidement une tâche avec un simple titre, la retrouver immédiatement dans le panneau et consulter les tâches au clavier. |
| [**F2-organiser-et-retrouver-ses-taches**](F2-organiser-et-retrouver-ses-taches.md) | **Organiser et retrouver ses tâches** | Modifier une tâche, lui attribuer une date prévue, une deadline ou des tags, la réordonner et la retrouver par section, recherche ou filtre. Comprend la préparation du prochain jour ouvré et l'aide aux commandes de saisie. |
| [**F3-terminer-et-suivre-ses-taches**](F3-terminer-et-suivre-ses-taches.md) | **Terminer et suivre ses tâches** | Cocher une tâche, la voir accomplie pendant la journée et la retrouver dans l'historique, y compris les jours suivants. |
| [**F4-configurer-les-raccourcis-clavier**](F4-configurer-les-raccourcis-clavier.md) | **Configurer les raccourcis clavier** | Choisir les raccourcis globaux de capture rapide et d'affichage du panneau, puis utiliser les nouvelles valeurs. |

Les exigences transversales, notamment l'utilisation principalement au clavier, l'absence de doublons entre sections et l'absence de notifications, seront précisées dans les features concernées.
