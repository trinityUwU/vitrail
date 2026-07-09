//! Résolution nom d'application depuis un `exe_path` — heuristique `.desktop` (story 1.4).
//! AFFICHAGE UNIQUEMENT : jamais utilisé pour la logique de corrélation, qui reste sur le
//! pid/exe_path exact (PLAN.md §6quinquies).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Cherche un `.desktop` dans `$XDG_DATA_DIRS/applications/` dont la ligne `Exec=` référence
/// le basename du binaire résolu. Fallback : nom de binaire brut.
pub fn resolve_app_name(exe_path: &str) -> String {
    let basename = Path::new(exe_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| exe_path.to_string());

    if basename.is_empty() {
        return exe_path.to_string();
    }

    for dir in applications_dirs() {
        if let Some(name) = search_dir(&dir, &basename) {
            return name;
        }
    }
    basename
}

/// Cache mémoire du nom d'app déjà résolu, clé = `exe_path` — `resolve_app_name` fait de l'I/O
/// disque (`fs::read_dir`/`fs::read_to_string` sur les `.desktop`) qui NE DOIT JAMAIS s'exécuter
/// sur le chemin synchrone de la RPC `AskRule` (server.rs, contexte critique du projet : le
/// daemon `opensnitchd` bloque tout le trafic réseau en attendant la réponse). Rempli
/// exclusivement en tâche de fond (`tokio::task::spawn_blocking`).
pub struct AppNameCache {
    entries: Mutex<HashMap<String, String>>,
}

impl AppNameCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, exe_path: &str) -> Option<String> {
        self.entries
            .lock()
            .expect("mutex cache noms d'app empoisonné")
            .get(exe_path)
            .cloned()
    }

    pub fn insert(&self, exe_path: String, name: String) {
        self.entries
            .lock()
            .expect("mutex cache noms d'app empoisonné")
            .insert(exe_path, name);
    }
}

impl Default for AppNameCache {
    fn default() -> Self {
        Self::new()
    }
}

fn applications_dirs() -> Vec<PathBuf> {
    let raw = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    let mut dirs: Vec<PathBuf> = raw
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join("applications"))
        .collect();
    if let Ok(home_data) = std::env::var("XDG_DATA_HOME") {
        dirs.insert(0, PathBuf::from(home_data).join("applications"));
    }
    dirs
}

/// Cherche le fichier `.desktop` (chemin complet, pas seulement `Name=`) dont `Exec=`
/// référence le basename de `exe_path` — réutilisé par `keylog::app_injection` (EPIC 3,
/// PLAN.md §6octies) pour localiser le `.desktop` à surcharger avant l'injection
/// `SSLKEYLOGFILE`. Même recherche que `resolve_app_name`, extraite ici plutôt que dupliquée
/// pour ne jamais faire diverger les deux heuristiques.
pub fn find_desktop_file(exe_path: &str) -> Option<PathBuf> {
    let basename = Path::new(exe_path)
        .file_name()?
        .to_string_lossy()
        .to_string();
    if basename.is_empty() {
        return None;
    }
    applications_dirs()
        .into_iter()
        .find_map(|dir| search_dir_for_path(&dir, &basename))
}

fn search_dir_for_path(dir: &Path, basename: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if desktop_file_exec_matches(&path, basename) {
            return Some(path);
        }
    }
    None
}

fn desktop_file_exec_matches(path: &Path, basename: &str) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    content.lines().any(|line| {
        line.trim_start()
            .strip_prefix("Exec=")
            .map(|exec| exec.contains(basename))
            .unwrap_or(false)
    })
}

fn search_dir(dir: &Path, basename: &str) -> Option<String> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(name) = check_desktop_file(&path, basename) {
            return Some(name);
        }
    }
    None
}

fn check_desktop_file(path: &Path, basename: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let exec_matches = content.lines().any(|line| {
        line.trim_start()
            .strip_prefix("Exec=")
            .map(|exec| exec.contains(basename))
            .unwrap_or(false)
    });
    if !exec_matches {
        return None;
    }
    content.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("Name=")
            .map(|n| n.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // `XDG_DATA_DIRS`/`XDG_DATA_HOME` sont des variables d'environnement globales au process :
    // sans ce verrou partagé (`shared::ENV_GUARD`), des tests de modules différents s'exécutant
    // en parallèle (comportement par défaut de `cargo test`) pourraient se marcher dessus de
    // façon non déterministe.
    use crate::shared::ENV_GUARD;

    #[test]
    fn fallback_sur_basename_si_aucun_desktop_ne_matche() {
        let _guard = ENV_GUARD.lock().unwrap();
        // XDG_DATA_DIRS pointé vers un dossier vide et isolé : garantit l'absence de faux
        // positif quel que soit l'environnement d'exécution du test.
        let tmp = std::env::temp_dir().join(format!("vitrail-test-desktop-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_DATA_DIRS", &tmp);
        std::env::remove_var("XDG_DATA_HOME");

        assert_eq!(
            resolve_app_name("/usr/bin/totally-unknown-binary"),
            "totally-unknown-binary"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn trouve_le_nom_via_exec_matchant() {
        let _guard = ENV_GUARD.lock().unwrap();
        let tmp =
            std::env::temp_dir().join(format!("vitrail-test-desktop2-{}", std::process::id()));
        let apps_dir = tmp.join("applications");
        std::fs::create_dir_all(&apps_dir).unwrap();
        let mut file = std::fs::File::create(apps_dir.join("firefox.desktop")).unwrap();
        writeln!(file, "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n").unwrap();

        std::env::set_var("XDG_DATA_DIRS", &tmp);
        std::env::remove_var("XDG_DATA_HOME");

        assert_eq!(resolve_app_name("/usr/lib/firefox/firefox"), "Firefox");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_desktop_file_retourne_le_chemin_complet_du_fichier_matchant() {
        let _guard = ENV_GUARD.lock().unwrap();
        let tmp =
            std::env::temp_dir().join(format!("vitrail-test-desktop3-{}", std::process::id()));
        let apps_dir = tmp.join("applications");
        std::fs::create_dir_all(&apps_dir).unwrap();
        let desktop_path = apps_dir.join("firefox.desktop");
        let mut file = std::fs::File::create(&desktop_path).unwrap();
        writeln!(file, "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n").unwrap();

        std::env::set_var("XDG_DATA_DIRS", &tmp);
        std::env::remove_var("XDG_DATA_HOME");

        assert_eq!(
            find_desktop_file("/usr/lib/firefox/firefox"),
            Some(desktop_path)
        );
        assert_eq!(find_desktop_file("/usr/bin/totally-unknown"), None);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn app_name_cache_absent_puis_present_apres_insert() {
        let cache = AppNameCache::new();
        assert!(cache.get("/usr/bin/firefox").is_none());
        cache.insert("/usr/bin/firefox".to_string(), "Firefox".to_string());
        assert_eq!(cache.get("/usr/bin/firefox").unwrap(), "Firefox");
    }
}
