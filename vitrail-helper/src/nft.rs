//! Gestion de la chaîne unique `VITRAIL_REDIRECT` (`inet vitrail`) — création/destruction
//! (EPIC 7) + règles de redirection MITM et exclusions (EPIC 4).
//!
//! CORRECTION DE TYPE (découverte EPIC 4, à signaler) : la chaîne créée en EPIC 7 était
//! déclarée `type filter` — valide pour un simple marqueur de présence, mais les instructions
//! `redirect`/règles référençant des sets `ip daddr` exigent une chaîne `type nat`. Aucune
//! nouvelle chaîne n'est créée : seul le type déclaré de LA MÊME chaîne `VITRAIL_REDIRECT`
//! passe de `filter` à `nat` (jamais utilisée avec de vraies règles avant cette passe, donc
//! aucune régression possible sur le comportement déjà audité d'EPIC 7 — "chaîne présente =
//! kill switch actif" reste vrai à l'identique).

use std::process::Command;

const NFT_BIN: &str = "nft";
const NFT_FAMILY: &str = "inet";
const NFT_TABLE: &str = "vitrail";
const NFT_CHAIN: &str = "VITRAIL_REDIRECT";
const SET_V4: &str = "vitrail_exclude_v4";
const SET_V6: &str = "vitrail_exclude_v6";

/// Crée la table `inet vitrail`, la chaîne `VITRAIL_REDIRECT` (vide, hook output, table nat)
/// et les deux sets d'exclusion (v4/v6, vides) si elles n'existent pas déjà. `nft add` est
/// idempotent (contrairement à `nft create`).
pub fn nft_apply() -> Result<(), String> {
    run_nft(&["add", "table", NFT_FAMILY, NFT_TABLE])?;
    run_nft(&[
        "add", "chain", NFT_FAMILY, NFT_TABLE, NFT_CHAIN, "{", "type", "nat", "hook", "output",
        "priority", "-100", ";", "}",
    ])?;
    run_nft(&[
        "add",
        "set",
        NFT_FAMILY,
        NFT_TABLE,
        SET_V4,
        "{",
        "type",
        "ipv4_addr",
        ";",
        "}",
    ])?;
    run_nft(&[
        "add",
        "set",
        NFT_FAMILY,
        NFT_TABLE,
        SET_V6,
        "{",
        "type",
        "ipv6_addr",
        ";",
        "}",
    ])?;
    Ok(())
}

/// Détruit la table `inet vitrail` (chaîne + sets + règles) si elle existe. Idempotent.
pub fn nft_flush() -> Result<(), String> {
    if !table_exists()? {
        return Ok(());
    }
    run_nft(&["delete", "table", NFT_FAMILY, NFT_TABLE])
}

/// Ajoute les règles d'exclusion (accept si IP dans les sets, ajoutées AVANT la redirection —
/// ordre garanti par l'ordre d'appel dans cette fonction) PUIS la règle de redirection DNAT
/// locale `redirect to :<port>` (PLAN.md §6nonies 4.3, port déjà validé `u16 > 1024` par
/// l'appelant). Table/chaîne/sets doivent déjà exister (créés par `nft-apply`) — jamais créés
/// ici, cohérent avec "jamais une nouvelle chaîne, jamais de règle en dehors". Découpée en 3
/// sous-fonctions (audit EPIC 4, point 6 — limite de taille de fonction).
pub fn nft_redirect(port: u16) -> Result<(), String> {
    add_exclude_accept_rule(&format!("@{SET_V4}"), "ip")?;
    add_exclude_accept_rule(&format!("@{SET_V6}"), "ip6")?;
    add_redirect_rule(port)
}

/// Règle `accept` pour une famille d'adresse donnée (`ip`/`ip6`) référençant le set d'exclusion
/// correspondant (`@vitrail_exclude_v4`/`@vitrail_exclude_v6`).
fn add_exclude_accept_rule(set_ref: &str, addr_family_keyword: &str) -> Result<(), String> {
    run_nft(&[
        "add",
        "rule",
        NFT_FAMILY,
        NFT_TABLE,
        NFT_CHAIN,
        "tcp",
        "dport",
        "{",
        "80",
        ",",
        "443",
        "}",
        addr_family_keyword,
        "daddr",
        set_ref,
        "accept",
    ])
}

/// Règle de redirection DNAT locale vers `port` — ajoutée en dernier, après les deux règles
/// `accept` d'exclusion (ordre garanti par l'appelant `nft_redirect`).
fn add_redirect_rule(port: u16) -> Result<(), String> {
    run_nft(&[
        "add",
        "rule",
        NFT_FAMILY,
        NFT_TABLE,
        NFT_CHAIN,
        "tcp",
        "dport",
        "{",
        "80",
        ",",
        "443",
        "}",
        "redirect",
        "to",
        &format!(":{port}"),
    ])
}

/// Retire UNIQUEMENT les règles ajoutées par `nft_redirect` (accept + redirect), laisse la
/// chaîne et les sets intacts (marqueur "kill switch actif" préservé, PLAN.md §6nonies 4.3).
/// nftables ne supporte la suppression que par handle numérique — liste les règles avec leur
/// handle (`-a`) et supprime chacune. Idempotent : chaîne/table absente ou déjà vide = no-op.
pub fn nft_clear_redirect() -> Result<(), String> {
    if !table_exists()? {
        return Ok(());
    }
    for handle in list_rule_handles()? {
        run_nft(&[
            "delete", "rule", NFT_FAMILY, NFT_TABLE, NFT_CHAIN, "handle", &handle,
        ])?;
    }
    Ok(())
}

/// Remplace intégralement le contenu des deux sets d'exclusion (v4/v6) — `flush set` puis
/// `add element` si la liste correspondante est non vide. Séparé en deux listes par
/// l'appelant (`main.rs`) selon `IpAddr::is_ipv4()`.
pub fn nft_set_exclusions(v4: &[String], v6: &[String]) -> Result<(), String> {
    run_nft(&["flush", "set", NFT_FAMILY, NFT_TABLE, SET_V4])?;
    run_nft(&["flush", "set", NFT_FAMILY, NFT_TABLE, SET_V6])?;
    if !v4.is_empty() {
        run_nft(&[
            "add",
            "element",
            NFT_FAMILY,
            NFT_TABLE,
            SET_V4,
            "{",
            &v4.join(","),
            "}",
        ])?;
    }
    if !v6.is_empty() {
        run_nft(&[
            "add",
            "element",
            NFT_FAMILY,
            NFT_TABLE,
            SET_V6,
            "{",
            &v6.join(","),
            "}",
        ])?;
    }
    Ok(())
}

fn table_exists() -> Result<bool, String> {
    let output = Command::new(NFT_BIN)
        .args(["list", "table", NFT_FAMILY, NFT_TABLE])
        .output()
        .map_err(|error| format!("échec d'exécution de `nft list table`: {error}"))?;
    Ok(output.status.success())
}

/// Parse la sortie de `nft -a list chain ...` pour extraire chaque `# handle <N>` de fin de
/// ligne de règle — best-effort, une ligne sans handle exploitable est ignorée plutôt que de
/// faire échouer tout le nettoyage.
fn list_rule_handles() -> Result<Vec<String>, String> {
    let output = Command::new(NFT_BIN)
        .args(["-a", "list", "chain", NFT_FAMILY, NFT_TABLE, NFT_CHAIN])
        .output()
        .map_err(|error| format!("échec d'exécution de `nft -a list chain`: {error}"))?;
    if !output.status.success() {
        // Chaîne absente malgré la table présente : rien à nettoyer, pas une erreur fatale.
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(|line| line.rsplit_once("# handle "))
        .map(|(_, handle)| handle.trim().to_string())
        .collect())
}

fn run_nft(args: &[&str]) -> Result<(), String> {
    let output = Command::new(NFT_BIN)
        .args(args)
        .output()
        .map_err(|error| format!("échec d'exécution de `nft {}`: {error}", args.join(" ")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`nft {}` a échoué (code {}): {}",
        args.join(" "),
        output.status.code().unwrap_or(-1),
        stderr.trim()
    ))
}
