//! Cache pid→exe_path avec clé composite `(pid, start_time)` — story 1.3. `start_time` est lu
//! depuis `/proc/<pid>/stat` (champ 22, ticks depuis le boot) pour ne jamais confondre un pid
//! recyclé avec l'ancien process qui l'occupait (PLAN.md §6quinquies).

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProcessKey {
    pid: u32,
    start_time: u64,
}

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    // Lu par `get()` (tests story 1.3) et par le futur consommateur EPIC 5 (corrélation) —
    // pas encore de lecteur en dehors des tests tant que `correlation/` n'existe pas.
    #[allow(dead_code)]
    pub exe_path: String,
}

pub struct ProcessCache {
    entries: Mutex<HashMap<ProcessKey, ProcessEntry>>,
}

impl ProcessCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Lit `/proc/<pid>/stat`, extrait le champ 22 (`starttime`, ticks depuis le boot).
    /// Retourne `None` si le process est déjà mort ou illisible — jamais de panic.
    pub fn read_start_time(pid: u32) -> Option<u64> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // Le champ `comm` (2e champ) est entre parenthèses et peut contenir espaces/parenthèses
        // imbriquées — on repart après la DERNIÈRE `)` pour une tokenisation fiable du reste.
        let after_comm = stat.rfind(')')?;
        let rest = &stat[after_comm + 1..];
        let fields: Vec<&str> = rest.split_whitespace().collect();
        // `state` est le 1er champ après `comm` (index 0 dans `fields`) ; `starttime` est le
        // champ global 22, donc l'index (22 - 3) = 19 dans `fields` (pid + comm déjà consommés).
        fields.get(19)?.parse::<u64>().ok()
    }

    pub fn insert(&self, pid: u32, start_time: u64, exe_path: String) {
        let key = ProcessKey { pid, start_time };
        self.entries
            .lock()
            .expect("mutex cache attribution empoisonné")
            .insert(key, ProcessEntry { exe_path });
    }

    #[cfg(test)]
    pub fn get(&self, pid: u32, start_time: u64) -> Option<ProcessEntry> {
        let key = ProcessKey { pid, start_time };
        self.entries
            .lock()
            .expect("mutex cache attribution empoisonné")
            .get(&key)
            .cloned()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("mutex cache attribution empoisonné")
            .len()
    }

    /// Éviction des entrées dont le process n'existe plus (`/proc/<pid>` absent) — appelée
    /// périodiquement, jamais depuis le chemin chaud de réception d'un événement gRPC.
    pub fn evict_dead(&self) {
        let mut entries = self
            .entries
            .lock()
            .expect("mutex cache attribution empoisonné");
        entries.retain(|key, _| std::path::Path::new(&format!("/proc/{}", key.pid)).exists());
    }
}

impl Default for ProcessCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_recycle_ne_confond_pas_deux_process() {
        let cache = ProcessCache::new();
        cache.insert(1234, 1000, "/usr/bin/firefox".to_string());
        cache.insert(1234, 2000, "/usr/bin/curl".to_string());

        // Même pid, deux `start_time` différents : deux entrées distinctes, aucune écrasée
        // par erreur (ce serait le bug si la clé était juste `pid`).
        assert_eq!(cache.get(1234, 1000).unwrap().exe_path, "/usr/bin/firefox");
        assert_eq!(cache.get(1234, 2000).unwrap().exe_path, "/usr/bin/curl");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn pid_inconnu_retourne_none() {
        let cache = ProcessCache::new();
        assert!(cache.get(9999, 1).is_none());
    }
}
