use super::*;

#[test]
fn une_fermeture_ipc_retardee_ne_masque_pas_lactivation_rouverte() {
    // RG-F1-03/F5-05 — Étant donné une capture déjà chargée et son identité native.
    let mut surfaces = Surfaces::default();
    let session = surfaces.reserve(Surface::Capture, false).unwrap();
    let (first, _) = surfaces.loaded(Surface::Capture, session).unwrap();
    assert_eq!(
        surfaces.hide_expected(Surface::Capture, session, first.activation),
        Some(first.activation)
    );
    let Opening::Reuse(reopened) = surfaces.request_open(Surface::Capture, false) else {
        panic!("capture à réutiliser")
    };
    // Quand la fermeture émise après une ancienne confirmation arrive au thread principal.
    assert!(
        surfaces
            .hide_expected(Surface::Capture, session, first.activation)
            .is_none()
    );
    // Alors la nouvelle activation reste demandée visible et peut être fermée par sa propre identité.
    assert!(surfaces.sessions[&Surface::Capture].requested_visible);
    assert_eq!(
        surfaces.hide_expected(Surface::Capture, session, reopened.activation),
        Some(reopened.activation)
    );
}

#[test]
fn une_fermeture_ipc_dancienne_webview_ne_touche_pas_la_nouvelle_session() {
    // RG-F0-02/F1-03 — Étant donné une capture détruite puis recréée.
    let mut surfaces = Surfaces::default();
    let old = surfaces.reserve(Surface::Capture, false).unwrap();
    let (activation, _) = surfaces.loaded(Surface::Capture, old).unwrap();
    surfaces.release(Surface::Capture, old);
    let current = surfaces.reserve(Surface::Capture, false).unwrap();
    // Quand une commande de fermeture de l'ancienne WebView est reçue.
    assert!(
        surfaces
            .hide_expected(Surface::Capture, old, activation.activation)
            .is_none()
    );
    // Alors la nouvelle WebView chargée reste visible.
    assert!(surfaces.loaded(Surface::Capture, current).is_some());
    assert!(surfaces.sessions[&Surface::Capture].requested_visible);
}
