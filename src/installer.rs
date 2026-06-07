//! Logique d'installation / désinstallation du ModLoader.
//!
//! L'installation est *idempotente* et *non destructive* :
//!   - les fichiers JSON Unity sont fusionnés (on n'écrase pas la liste du jeu,
//!     ce qui survit aux mises à jour de BROKE PROTOCOL) ;
//!   - une sauvegarde `.modloader.bak` est créée avant toute modification ;
//!   - relancer l'installeur ne crée pas de doublons.

use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::assets;

/// Type des DLL non-Unity dans ScriptingAssemblies (les modules UnityEngine sont 2).
const ASSEMBLY_TYPE_MANAGED: i64 = 16;

/// Suffixe des fichiers de sauvegarde.
const BACKUP_SUFFIX: &str = ".modloader.bak";

/// Erreur d'installation avec message lisible (affiché tel quel dans la GUI).
#[derive(Debug)]
pub struct InstallError(pub String);

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InstallError {}

type Result<T> = std::result::Result<T, InstallError>;

/// Raccourci pour construire une erreur formatée.
macro_rules! err {
    ($($arg:tt)*) => { InstallError(format!($($arg)*)) };
}

/// Chemins importants déduits du dossier racine du jeu.
struct GamePaths {
    scripting_assemblies: PathBuf,
    runtime_init: PathBuf,
    managed: PathBuf,
}

impl GamePaths {
    fn from_game_dir(game_dir: &Path) -> Result<Self> {
        let data = game_dir.join("BrokeProtocol_Data");
        if !data.is_dir() {
            return Err(err!(
                "Dossier introuvable : {}\nCe n'est pas une installation BROKE PROTOCOL valide.",
                data.display()
            ));
        }

        let scripting_assemblies = find_unity_file(&data, "ScriptingAssemblies")?;
        let runtime_init = find_unity_file(&data, "RuntimeInitializeOnLoads")?;
        let managed = data.join("Managed");
        if !managed.is_dir() {
            return Err(err!("Dossier introuvable : {}", managed.display()));
        }

        Ok(Self {
            scripting_assemblies,
            runtime_init,
            managed,
        })
    }
}

/// Localise un fichier Unity en tolérant la présence ou non de l'extension `.json`.
fn find_unity_file(data_dir: &Path, base_name: &str) -> Result<PathBuf> {
    for candidate in [format!("{base_name}.json"), base_name.to_string()] {
        let p = data_dir.join(&candidate);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(err!(
        "Fichier introuvable : {}(.json) dans {}",
        base_name,
        data_dir.display()
    ))
}

/// Lance l'installation complète. Renvoie la liste des actions effectuées (pour la GUI).
pub fn install(game_dir: &Path) -> Result<Vec<String>> {
    let paths = GamePaths::from_game_dir(game_dir)?;
    let mut log = Vec::new();

    // 1. ScriptingAssemblies : déclarer les nouvelles DLL.
    patch_scripting_assemblies(&paths.scripting_assemblies, &mut log)?;

    // 2. RuntimeInitializeOnLoads : enregistrer le point d'entrée du ModLoader.
    patch_runtime_init(&paths.runtime_init, &mut log)?;

    // 3. Copier les DLL dans Managed/.
    copy_dlls(&paths.managed, &mut log)?;

    log.push("Installation terminée avec succès.".to_string());
    Ok(log)
}

/// Restaure les fichiers d'origine et supprime les DLL ajoutées.
pub fn uninstall(game_dir: &Path) -> Result<Vec<String>> {
    let paths = GamePaths::from_game_dir(game_dir)?;
    let mut log = Vec::new();

    for file in [&paths.scripting_assemblies, &paths.runtime_init] {
        let backup = backup_path(file);
        if backup.is_file() {
            std::fs::copy(&backup, file)
                .map_err(|e| err!("Restauration impossible de {} : {e}", file.display()))?;
            std::fs::remove_file(&backup).ok();
            log.push(format!("Restauré : {}", file.display()));
        } else {
            log.push(format!(
                "Aucune sauvegarde pour {} (ignoré).",
                file.display()
            ));
        }
    }

    for (name, _) in assets::MANAGED_DLLS {
        let dll = paths.managed.join(name);
        if dll.is_file() {
            std::fs::remove_file(&dll)
                .map_err(|e| err!("Suppression impossible de {} : {e}", dll.display()))?;
            log.push(format!("Supprimé : {}", dll.display()));
        }
    }

    log.push("Désinstallation terminée.".to_string());
    Ok(log)
}

/// Indique si le ModLoader semble déjà installé (DLL présentes).
pub fn is_installed(game_dir: &Path) -> bool {
    let managed = game_dir.join("BrokeProtocol_Data").join("Managed");
    assets::MANAGED_DLLS
        .iter()
        .all(|(name, _)| managed.join(name).is_file())
}

// --- Détails d'implémentation ---------------------------------------------

fn backup_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_os_string();
    s.push(BACKUP_SUFFIX);
    PathBuf::from(s)
}

/// Crée la sauvegarde `.modloader.bak` si elle n'existe pas déjà.
fn ensure_backup(file: &Path, log: &mut Vec<String>) -> Result<()> {
    let backup = backup_path(file);
    if !backup.exists() {
        std::fs::copy(file, &backup)
            .map_err(|e| err!("Sauvegarde impossible de {} : {e}", file.display()))?;
        log.push(format!("Sauvegarde créée : {}", backup.display()));
    }
    Ok(())
}

fn read_json(file: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(file)
        .map_err(|e| err!("Lecture impossible de {} : {e}", file.display()))?;
    serde_json::from_str(&raw).map_err(|e| err!("JSON invalide dans {} : {e}", file.display()))
}

fn write_json(file: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| err!("Sérialisation JSON impossible : {e}"))?;
    std::fs::write(file, text)
        .map_err(|e| err!("Écriture impossible de {} : {e}\n(Essayez de lancer en administrateur.)", file.display()))
}

/// Ajoute `!_0Harmony.dll` et `ModLoader.dll` aux tableaux `names`/`types`.
fn patch_scripting_assemblies(file: &Path, log: &mut Vec<String>) -> Result<()> {
    ensure_backup(file, log)?;
    let mut root = read_json(file)?;

    let names = root
        .get_mut("names")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| err!("Champ \"names\" absent ou invalide dans {}", file.display()))?;

    // Quelles DLL manquent ?
    let existing: Vec<String> = names
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let to_add: Vec<&str> = assets::MANAGED_DLLS
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| !existing.iter().any(|e| e == n))
        .collect();

    if to_add.is_empty() {
        log.push("ScriptingAssemblies : déjà à jour.".to_string());
        return Ok(());
    }

    for n in &to_add {
        names.push(json!(n));
    }

    let types = root
        .get_mut("types")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| err!("Champ \"types\" absent ou invalide dans {}", file.display()))?;
    for _ in &to_add {
        types.push(json!(ASSEMBLY_TYPE_MANAGED));
    }

    write_json(file, &root)?;
    log.push(format!(
        "ScriptingAssemblies : ajouté {}.",
        to_add.join(", ")
    ));
    Ok(())
}

/// Construit l'entrée d'initialisation du ModLoader.
fn modloader_init_entry() -> Value {
    json!({
        "assemblyName": "ModLoader",
        "nameSpace": "ModLoader",
        "className": "Core",
        "methodName": "Start",
        "loadTypes": 0,
        "isUnityClass": false
    })
}

/// Vérifie si une entrée correspond déjà au point d'entrée du ModLoader.
fn is_modloader_entry(v: &Value) -> bool {
    v.get("assemblyName").and_then(Value::as_str) == Some("ModLoader")
        && v.get("className").and_then(Value::as_str) == Some("Core")
        && v.get("methodName").and_then(Value::as_str) == Some("Start")
}

/// Enregistre `ModLoader.Core.Start` dans le tableau `root`.
fn patch_runtime_init(file: &Path, log: &mut Vec<String>) -> Result<()> {
    ensure_backup(file, log)?;
    let mut root = read_json(file)?;

    let entries = root
        .get_mut("root")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| err!("Champ \"root\" absent ou invalide dans {}", file.display()))?;

    if entries.iter().any(is_modloader_entry) {
        log.push("RuntimeInitializeOnLoads : déjà à jour.".to_string());
        return Ok(());
    }

    entries.push(modloader_init_entry());
    write_json(file, &root)?;
    log.push("RuntimeInitializeOnLoads : point d'entrée ModLoader ajouté.".to_string());
    Ok(())
}

/// Copie les DLL embarquées dans le dossier Managed du jeu.
fn copy_dlls(managed: &Path, log: &mut Vec<String>) -> Result<()> {
    for (name, bytes) in assets::MANAGED_DLLS {
        let dst = managed.join(name);
        std::fs::write(&dst, bytes).map_err(|e| {
            err!(
                "Copie impossible de {} : {e}\n(Essayez de lancer en administrateur.)",
                dst.display()
            )
        })?;
        log.push(format!("Copié : {} ({} octets)", name, bytes.len()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripting_assemblies_idempotent() {
        let dir = std::env::temp_dir().join("mlsetup_test_sa");
        let data = dir.join("BrokeProtocol_Data");
        std::fs::create_dir_all(data.join("Managed")).unwrap();
        let file = data.join("ScriptingAssemblies.json");
        std::fs::write(
            &file,
            r#"{"names":["UnityEngine.dll"],"types":[2]}"#,
        )
        .unwrap();

        let mut log = Vec::new();
        patch_scripting_assemblies(&file, &mut log).unwrap();
        let v = read_json(&file).unwrap();
        let names = v["names"].as_array().unwrap();
        let types = v["types"].as_array().unwrap();
        assert_eq!(names.len(), 3);
        assert_eq!(types.len(), 3);
        assert!(names.iter().any(|n| n == "ModLoader.dll"));
        assert!(names.iter().any(|n| n == "!_0Harmony.dll"));

        // Deuxième passe : aucun doublon.
        patch_scripting_assemblies(&file, &mut log).unwrap();
        let v = read_json(&file).unwrap();
        assert_eq!(v["names"].as_array().unwrap().len(), 3);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn runtime_init_idempotent() {
        let dir = std::env::temp_dir().join("mlsetup_test_ri");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("RuntimeInitializeOnLoads.json");
        std::fs::write(&file, r#"{"root":[]}"#).unwrap();

        let mut log = Vec::new();
        patch_runtime_init(&file, &mut log).unwrap();
        patch_runtime_init(&file, &mut log).unwrap();
        let v = read_json(&file).unwrap();
        let count = v["root"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| is_modloader_entry(e))
            .count();
        assert_eq!(count, 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
