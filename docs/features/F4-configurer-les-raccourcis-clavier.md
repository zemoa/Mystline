# F4-configurer-les-raccourcis-clavier — Configurer les raccourcis clavier

## Objectif et périmètre

Permettre de choisir les deux raccourcis globaux qui déclenchent la capture rapide et l'affichage/masquage du panneau. F0 fournit les raccourcis initiaux, leur enregistrement système et la surface de paramètres ; F4 définit leur modification par l'utilisateur. Les raccourcis internes au panneau ou à la saisie restent définis dans leurs features respectives.

Références : [SFG, §4, §6 et §17](../SFG.md) ; [architecture, §19, §22–24](../ARCHITECTURE.md).

## Parcours couvert

Depuis « Paramètres » dans le tray, l'utilisateur visualise les raccourcis de capture et de panneau, en modifie un, valide, puis utilise sa nouvelle combinaison globale sans redémarrer l'application.

## Règles de gestion

- **RG-F4-01 — Deux actions distinctes.** Les paramètres proposent exactement les deux raccourcis globaux du MVP : « Capture rapide » et « Ouvrir / masquer le panneau ». Les valeurs initiales définies par F0 sont `Ctrl+Shift+Espace` et `Ctrl+Shift+T`. Le changement d'un raccourci ne modifie pas l'autre.
- **RG-F4-02 — Configuration persistante.** Une combinaison validée devient la valeur configurée pour l'action et reste enregistrée pour les lancements suivants. L'ancienne combinaison de cette action cesse de la déclencher. La combinaison nouvelle est tentée immédiatement auprès du système, sans nécessiter de redémarrage.
- **RG-F4-03 — Collision interne.** L'utilisateur ne peut pas assigner la même combinaison aux deux actions ; la modification est refusée dans les paramètres et la configuration antérieure reste inchangée.
- **RG-F4-04 — Refus par le système.** Si le système d'exploitation refuse l'enregistrement d'un raccourci configuré, la valeur demandée est néanmoins conservée dans la configuration ; son état courant est indiqué comme « inactif » et l'utilisateur peut toujours accéder à l'action par le tray. Aucune notification de tâche n'est émise.
- **RG-F4-05 — Nouvelles tentatives.** L'enregistrement d'un raccourci inactif est retenté au démarrage de l'application ou lorsqu'il est modifié par l'utilisateur, et non à intervalles automatiques.
- **RG-F4-06 — Utilisation au clavier.** L'accès aux paramètres depuis le tray, le choix de l'action à modifier, la saisie de la combinaison et la validation peuvent être réalisés au clavier.

## Vérifications fonctionnelles

- Après avoir changé le raccourci de capture, seule la nouvelle combinaison ouvre la capture ; le raccourci du panneau fonctionne toujours, et le réglage survit à un redémarrage.
- Attribuer la même combinaison aux deux actions est refusé sans altérer leurs valeurs précédentes.
- Si une combinaison est indisponible au niveau de l'OS, les paramètres montrent la valeur enregistrée et son état inactif ; le tray garde l'action accessible. Le système ne retente pas l'enregistrement en boucle.

## Frontière avec les autres features

F4 règle uniquement les deux raccourcis globaux. Leur comportement une fois déclenchés est défini en F1 ; leur enregistrement initial et le tray relèvent de F0. La navigation, la recherche, l'aide et les actions au clavier à l'intérieur des surfaces relèvent de F1, F2 et F3.
