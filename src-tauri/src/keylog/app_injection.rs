//! Injection `SSLKEYLOGFILE` pour les apps ciblées (story 3.2) — wrapper de lancement
//! `$XDG_DATA_HOME/vitrail/keylog-wrapper.sh` + copie utilisateur du `.desktop`
//! (`$XDG_DATA_HOME/applications/<basename>.desktop`, mécanisme XDG standard de surcharge,
//! jamais le `.desktop` système touché). Toute surcharge PRÉEXISTANTE à ce chemin est
//! sauvegardée avant écrasement (`$XDG_DATA_HOME/vitrail/keylog-backups/`) pour restauration
//! exacte à la désactivation — jamais une suppression aveugle (PLAN.md §6octies).

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::attribution::find_desktop_file;
use crate::storage::keylog::KeylogAppRow;
use crate::storage::{self, StorageHandle};

use super::{vitrail_data_dir, xdg_data_home};

const WRAPPER_NAME: &str = "keylog-wrapper.sh";
const BACKUP_DIR: &str = "keylog-backups";

/// Résultat d'une tentative d'injection pour une app — jamais fatal individuellement (3.2 est
/// best-effort par app : une app sans `.desktop` résolvable reste simplement non couverte,
/// visible côté UI story 3.5).
pub enum InjectionOutcome {
    Injected {
        desktop_path: PathBuf,
        #[allow(dead_code)] // conservé pour un futur affichage détaillé (EPIC 8), pas de panne
        backup_path: Option<PathBuf>,
    },
    NoDesktopFile,
    AlreadyInjected,
}

pub fn wrapper_path() -> PathBuf {
    vitrail_data_dir().join(WRAPPER_NAME)
}

/// Écrit (ou réécrit, idempotent) le script wrapper — pose `SSLKEYLOGFILE` puis `exec "$@"`.
pub fn ensure_wrapper(keyfile: &Path) -> io::Result<PathBuf> {
    let path = wrapper_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = format!(
        "#!/bin/sh\nexport SSLKEYLOGFILE={}\nexec \"$@\"\n",
        shell_quote(&keyfile.to_string_lossy())
    );
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o700)
        .open(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(path)
}

/// Échappement minimal shell (guillemets simples) — le chemin de clés vient de `XDG_DATA_HOME`
/// (potentiellement porteur d'espaces), jamais interpolé sans échappement dans le wrapper.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Injecte une app ciblée — résout son `.desktop`, sauvegarde une éventuelle surcharge
/// préexistante, réécrit `Exec=` pour passer par le wrapper. Ignore silencieusement (log
/// warn/error, jamais de panic) une app déjà marquée injectée (résidu d'un arrêt non propre) :
/// re-snapshotter le `.desktop` déjà réécrit par Vitrail comme s'il s'agissait de l'original
/// casserait la garantie de réversibilité (PLAN.md §6octies).
pub fn inject_app(storage: &StorageHandle, row: &KeylogAppRow, wrapper: &Path) -> InjectionOutcome {
    if row.desktop_path.is_some() {
        tracing::warn!(
            binary = %row.binary_path,
            "keylog: app déjà marquée injectée (résidu d'un arrêt non propre), injection ignorée"
        );
        return InjectionOutcome::AlreadyInjected;
    }

    let Some(desktop_source) = find_desktop_file(&row.binary_path) else {
        return InjectionOutcome::NoDesktopFile;
    };

    let override_path = user_override_path(&desktop_source);
    let backup_path = match backup_existing_override(&override_path) {
        Ok(path) => path,
        Err(error) => {
            tracing::error!(
                error = %error, path = %override_path.display(),
                "keylog: sauvegarde de la surcharge .desktop préexistante échouée, injection annulée"
            );
            return InjectionOutcome::NoDesktopFile;
        }
    };

    if let Err(error) = write_override(&desktop_source, &override_path, wrapper) {
        tracing::error!(error = %error, path = %override_path.display(), "keylog: écriture de la surcharge .desktop échouée");
        return InjectionOutcome::NoDesktopFile;
    }

    persist_injection(storage, row, &override_path, backup_path.as_deref());
    InjectionOutcome::Injected {
        desktop_path: override_path,
        backup_path,
    }
}

fn persist_injection(
    storage: &StorageHandle,
    row: &KeylogAppRow,
    override_path: &Path,
    backup_path: Option<&Path>,
) {
    let backup = backup_path.map(|p| p.to_string_lossy().to_string());
    if let Err(error) = storage::keylog::record_injection(
        storage,
        &row.binary_path,
        &override_path.to_string_lossy(),
        backup.as_deref(),
    ) {
        tracing::error!(error = %error, "keylog: persistance de l'état d'injection échouée");
    }
}

/// Restaure une app à son état d'origine (`stop()`, story 3.2) : copie sa sauvegarde si une
/// surcharge préexistait, supprime l'override sinon (Vitrail l'avait créé de zéro) — jamais une
/// simple suppression aveugle qui effacerait une personnalisation non liée à Vitrail.
pub fn restore_app(storage: &StorageHandle, row: &KeylogAppRow) {
    let Some(desktop_path) = &row.desktop_path else {
        return;
    };
    let path = PathBuf::from(desktop_path);

    let result = match &row.backup_path {
        Some(backup) => fs::copy(backup, &path).map(|_| ()),
        None => remove_if_present(&path),
    };
    if let Err(error) = result {
        tracing::error!(
            error = %error, path = %path.display(),
            "keylog: restauration de la surcharge .desktop échouée — divergence potentielle"
        );
    }

    if let Err(error) = storage::keylog::clear_injection(storage, &row.binary_path) {
        tracing::error!(error = %error, "keylog: effacement de l'état d'injection échoué");
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn user_override_path(desktop_source: &Path) -> PathBuf {
    let basename = desktop_source.file_name().unwrap_or_default();
    xdg_data_home().join("applications").join(basename)
}

fn backup_existing_override(override_path: &Path) -> io::Result<Option<PathBuf>> {
    if !override_path.exists() {
        return Ok(None);
    }
    let backup_dir = vitrail_data_dir().join(BACKUP_DIR);
    fs::create_dir_all(&backup_dir)?;
    let basename = override_path.file_name().unwrap_or_default();
    let backup_path = backup_dir.join(basename);
    fs::copy(override_path, &backup_path)?;
    Ok(Some(backup_path))
}

fn write_override(source: &Path, override_path: &Path, wrapper: &Path) -> io::Result<()> {
    let content = fs::read_to_string(source)?;
    let rewritten = rewrite_exec_lines(&content, wrapper);
    if let Some(parent) = override_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(override_path, rewritten)
}

/// Réécrit chaque ligne `Exec=` pour passer par le wrapper — les `.desktop` valides n'en ont
/// normalement qu'une, mais toutes sont réécrites par prudence (jamais une supposition sur le
/// nombre d'occurrences).
fn rewrite_exec_lines(content: &str, wrapper: &Path) -> String {
    let wrapper = wrapper.to_string_lossy();
    let mut out = content
        .lines()
        .map(|line| match line.trim_start().strip_prefix("Exec=") {
            Some(rest) => format!("Exec={wrapper} {rest}"),
            None => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::shared::ENV_GUARD;

    fn isolated_env(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "vitrail-app-injection-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::env::set_var("XDG_DATA_HOME", base.join("data"));
        std::env::set_var("XDG_DATA_DIRS", base.join("system"));
        base
    }

    fn cleanup(base: &Path) {
        let _ = fs::remove_dir_all(base);
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_DATA_DIRS");
    }

    fn write_system_desktop(base: &Path, basename: &str) -> PathBuf {
        let dir = base.join("system").join("applications");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(basename);
        fs::write(&path, "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n").unwrap();
        path
    }

    fn row(binary_path: &str) -> KeylogAppRow {
        KeylogAppRow {
            binary_path: binary_path.to_string(),
            desktop_path: None,
            backup_path: None,
        }
    }

    #[test]
    fn inject_puis_restore_sans_surcharge_preexistante_supprime_l_override() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("no-prior");
        write_system_desktop(&base, "firefox.desktop");
        let wrapper = base.join("wrapper.sh");
        fs::write(&wrapper, "#!/bin/sh\n").unwrap();

        let storage = StorageHandle::open_in_memory().unwrap();
        let app = row("/usr/lib/firefox/firefox");
        storage::keylog::add_app(&storage, &app.binary_path).unwrap();

        let outcome = inject_app(&storage, &app, &wrapper);
        let InjectionOutcome::Injected {
            desktop_path,
            backup_path,
        } = outcome
        else {
            panic!("injection attendue");
        };
        assert!(
            backup_path.is_none(),
            "aucune surcharge préexistante à sauvegarder"
        );
        let content = fs::read_to_string(&desktop_path).unwrap();
        assert!(content.contains(&format!("Exec={}", wrapper.display())));

        let persisted = storage::keylog::list_apps(&storage).unwrap();
        assert_eq!(
            persisted[0].desktop_path.as_deref(),
            Some(desktop_path.to_str().unwrap())
        );

        restore_app(&storage, &persisted[0]);
        assert!(
            !desktop_path.exists(),
            "override créé par Vitrail doit être supprimé"
        );
        let cleared = storage::keylog::list_apps(&storage).unwrap();
        assert_eq!(cleared[0].desktop_path, None);

        cleanup(&base);
    }

    #[test]
    fn inject_sauvegarde_une_surcharge_preexistante_et_la_restaure_exactement() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("prior-override");
        write_system_desktop(&base, "firefox.desktop");
        let wrapper = base.join("wrapper.sh");
        fs::write(&wrapper, "#!/bin/sh\n").unwrap();

        // Surcharge utilisateur préexistante, non liée à Vitrail (ex: personnalisation locale).
        let override_dir = base.join("data").join("applications");
        fs::create_dir_all(&override_dir).unwrap();
        let override_path = override_dir.join("firefox.desktop");
        let original_content = "[Desktop Entry]\nName=Firefox (perso)\nExec=firefox --private\n";
        fs::write(&override_path, original_content).unwrap();

        let storage = StorageHandle::open_in_memory().unwrap();
        let app = row("/usr/lib/firefox/firefox");
        storage::keylog::add_app(&storage, &app.binary_path).unwrap();

        let outcome = inject_app(&storage, &app, &wrapper);
        let InjectionOutcome::Injected { backup_path, .. } = outcome else {
            panic!("injection attendue");
        };
        assert!(
            backup_path.is_some(),
            "la surcharge préexistante doit être sauvegardée"
        );

        let persisted = storage::keylog::list_apps(&storage).unwrap();
        restore_app(&storage, &persisted[0]);

        assert_eq!(
            fs::read_to_string(&override_path).unwrap(),
            original_content,
            "le contenu original doit être restauré exactement"
        );

        cleanup(&base);
    }

    #[test]
    fn inject_sans_desktop_resolvable_renvoie_no_desktop_file() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("no-desktop");
        let wrapper = base.join("wrapper.sh");
        fs::write(&wrapper, "#!/bin/sh\n").unwrap();

        let storage = StorageHandle::open_in_memory().unwrap();
        let app = row("/usr/bin/totally-unknown-binary");

        assert!(matches!(
            inject_app(&storage, &app, &wrapper),
            InjectionOutcome::NoDesktopFile
        ));

        cleanup(&base);
    }

    #[test]
    fn inject_deux_fois_de_suite_sans_restore_est_ignore() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("double-inject");
        write_system_desktop(&base, "firefox.desktop");
        let wrapper = base.join("wrapper.sh");
        fs::write(&wrapper, "#!/bin/sh\n").unwrap();

        let storage = StorageHandle::open_in_memory().unwrap();
        let app = row("/usr/lib/firefox/firefox");
        storage::keylog::add_app(&storage, &app.binary_path).unwrap();

        inject_app(&storage, &app, &wrapper);
        let already_injected = storage::keylog::list_apps(&storage).unwrap().remove(0);

        assert!(matches!(
            inject_app(&storage, &already_injected, &wrapper),
            InjectionOutcome::AlreadyInjected
        ));

        cleanup(&base);
    }
}
