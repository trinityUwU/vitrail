//! Écriture JSON Lines sur stdout — un enregistrement par paquet retenu, flush immédiat pour
//! ne pas retarder la visibilité côté process parent (PLAN.md §6quater).

use std::io::Write;
use std::sync::Mutex;

use crate::packet::CapturedPacket;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

pub fn write_record(record: &CapturedPacket) {
    let line = match serde_json::to_string(record) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("vitrail-capture-helper: sérialisation JSON échouée: {error}");
            return;
        }
    };

    let _guard = STDOUT_LOCK.lock().expect("mutex stdout empoisonné");
    let mut stdout = std::io::stdout();
    if let Err(error) = writeln!(stdout, "{line}") {
        eprintln!("vitrail-capture-helper: écriture stdout échouée: {error}");
        return;
    }
    let _ = stdout.flush();
}
