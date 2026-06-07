//! Détection automatique du dossier d'installation de BROKE PROTOCOL via Steam.
//!
//! Stratégie :
//!   1. Lire le chemin d'installation de Steam dans le registre Windows.
//!   2. Analyser `steamapps/libraryfolders.vdf` pour lister toutes les
//!      bibliothèques Steam (le jeu peut être sur un autre disque).
//!   3. Chercher `steamapps/common/BROKE PROTOCOL` dans chacune.
//!   4. Vérifier la présence de `BrokeProtocol_Data` pour valider.

use std::path::{Path, PathBuf};

/// Nom du dossier du jeu sous `steamapps/common`.
const GAME_FOLDER: &str = "BROKE PROTOCOL";

/// Renvoie le chemin racine du jeu si une installation valide est trouvée.
pub fn detect_game_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    for lib in steam_libraries() {
        candidates.push(lib.join("steamapps").join("common").join(GAME_FOLDER));
    }

    // Filets de sécurité : emplacements par défaut courants.
    for fallback in [
        r"C:\Program Files (x86)\Steam\steamapps\common\BROKE PROTOCOL",
        r"C:\Program Files\Steam\steamapps\common\BROKE PROTOCOL",
    ] {
        candidates.push(PathBuf::from(fallback));
    }

    candidates.into_iter().find(|p| is_valid_game_dir(p))
}

/// Vérifie qu'un dossier ressemble bien à une installation de BROKE PROTOCOL.
pub fn is_valid_game_dir(dir: &Path) -> bool {
    dir.join("BrokeProtocol_Data").is_dir()
}

/// Liste les racines de bibliothèques Steam (chaque entrée contient un sous-dossier `steamapps`).
fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();

    let Some(steam_root) = steam_root() else {
        return libs;
    };
    libs.push(steam_root.clone());

    // libraryfolders.vdf liste les bibliothèques additionnelles.
    let vdf = steam_root
        .join("steamapps")
        .join("libraryfolders.vdf");
    if let Ok(content) = std::fs::read_to_string(&vdf) {
        for path in parse_vdf_paths(&content) {
            let p = PathBuf::from(path);
            if !libs.contains(&p) {
                libs.push(p);
            }
        }
    }

    libs
}

/// Récupère le dossier d'installation de Steam depuis le registre Windows.
#[cfg(windows)]
fn steam_root() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    // HKCU\Software\Valve\Steam\SteamPath (chemin de l'utilisateur courant).
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(path) = key.get_value::<String, _>("SteamPath") {
            let p = PathBuf::from(path.replace('/', "\\"));
            if p.is_dir() {
                return Some(p);
            }
        }
    }

    // HKLM\SOFTWARE\WOW6432Node\Valve\Steam\InstallPath (machine, Steam 32 bits).
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for sub in [
        r"SOFTWARE\WOW6432Node\Valve\Steam",
        r"SOFTWARE\Valve\Steam",
    ] {
        if let Ok(key) = hklm.open_subkey(sub) {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(path);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn steam_root() -> Option<PathBuf> {
    None
}

/// Extrait les valeurs `"path"` d'un contenu VDF (libraryfolders.vdf).
///
/// On reste volontairement permissif : on prend toute paire dont la clé est
/// `path` et on déséchappe les `\\`.
fn parse_vdf_paths(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        // Format : "path"		"D:\\SteamLibrary"
        let mut parts = line.split('"').filter(|s| !s.trim().is_empty());
        if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
            if key.eq_ignore_ascii_case("path") {
                out.push(val.replace("\\\\", "\\"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vdf_extrait_les_chemins() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;
        let paths = parse_vdf_paths(vdf);
        assert_eq!(
            paths,
            vec![
                r"C:\Program Files (x86)\Steam".to_string(),
                r"D:\SteamLibrary".to_string()
            ]
        );
    }
}
