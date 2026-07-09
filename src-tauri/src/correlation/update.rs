//! Enrichissement a posteriori d'un `Flow` déjà émis (fix audit 5.2) — un fragment
//! `Decryption` peut arriver après que `capture`/`attribution` aient déjà fermé et vidé le
//! buffer actif : `tshark` reconstruit le contenu HTTP après la fin du handshake TLS, souvent
//! après expiration/émission côté capture+attribution (ordre chronologique réaliste, pas un
//! cas limite). Sans ce module, `engine::ingest` créerait un second `Flow` `Fully` pour la
//! même connexion 5-tuple — viole EPICS.md 5.2 ("une même connexion vue par plusieurs sources
//! doit produire UN SEUL enregistrement"). Recherche le flow déjà persisté par 4-tuple
//! (ip/port/source_ip/source_port — `Flow.protocol` porte le protocole applicatif détecté par
//! `capture/`, pas le protocole transport brut du 5-tuple, donc exclu de la recherche,
//! cf. `storage::flows::find_recent_by_five_tuple`) dans une fenêtre récente, le met à jour en
//! place (contenu déchiffré + upgrade visibilité `Fully`) plutôt que d'en créer un second.

use crate::attribution::FiveTuple;
use crate::keylog::DecryptedFragment;
use crate::shared::{CorrelationSource, Flow, FlowVisibility};
use crate::storage::{self, StorageHandle};

use super::builder::DecryptionContent;

/// Fenêtre de recherche du flow déjà émis à enrichir — volontairement plus large que
/// `CORRELATION_WINDOW` (5s) : le fragment déchiffré tardif peut arriver après plusieurs
/// cycles de sweep, pas seulement après une seule fenêtre de fusion ratée.
const RECENT_FLOW_WINDOW_SECS: i64 = 30;

/// Tente d'enrichir un flow déjà émis pour ce 5-tuple avec un fragment déchiffré tardif.
/// Retourne `true` si un flow a été trouvé et mis à jour (l'appelant ne doit alors PAS créer
/// de nouvelle entrée dans le buffer actif) — `false` si rien n'a été trouvé (première
/// apparition réelle de ce 5-tuple, ou erreur storage), l'appelant continue le chemin normal
/// `ingest` (fusion immédiate en `Fully`, comme avant ce fix).
pub fn try_enrich_already_emitted(
    five_tuple: &FiveTuple,
    fragment: &DecryptedFragment,
    storage: &StorageHandle,
    emit: &impl Fn(&Flow),
) -> bool {
    let existing = storage::flows::find_recent_by_five_tuple(
        storage,
        &five_tuple.dst_ip,
        five_tuple.dst_port,
        &five_tuple.src_ip,
        five_tuple.src_port,
        RECENT_FLOW_WINDOW_SECS,
    );
    let Ok(Some(mut flow)) = existing else {
        if let Err(error) = existing {
            tracing::error!(error = %error, "recherche d'un flow déjà émis à enrichir échouée");
        }
        return false;
    };

    apply_decryption(&mut flow, fragment);
    if let Err(error) = storage::flows::update_flow(storage, &flow) {
        tracing::error!(error = %error, flow_id = %flow.id, "mise à jour d'un flow enrichi échouée");
    }
    emit(&flow);
    true
}

fn apply_decryption(flow: &mut Flow, fragment: &DecryptedFragment) {
    let content = DecryptionContent::from_fragment(fragment);
    flow.visibility = FlowVisibility::Fully;
    if let Some(host) = content.host {
        flow.destination = host;
    }
    flow.method = content.method;
    flow.path = content.path;
    flow.status = content.status;
    flow.request_headers = content.request_headers;
    flow.response_headers = content.response_headers;
    flow.body_preview = content.body_preview;
    flow.content_type = content.content_type;
    flow.certificate = content.certificate;
    mark_keylog_source_ok(&mut flow.sources);
}

/// Marque la source "Keylog" comme `ok` dans la liste déjà persistée (Capture/Attribution/
/// Décryptage/Keylog, cf. `builder::build_sources`) — les 3 autres restent inchangées, seule
/// la provenance du contenu enrichi évolue.
fn mark_keylog_source_ok(sources: &mut [CorrelationSource]) {
    for source in sources.iter_mut() {
        if source.name == "Keylog" {
            source.status = "ok".into();
            source.detail =
                "Fragment déchiffré reçu via tshark (SSLKEYLOGFILE), enrichi a posteriori".into();
        }
    }
}
