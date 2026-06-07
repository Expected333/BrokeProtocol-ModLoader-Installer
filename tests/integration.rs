//! Test d'intégration : simule une installation BROKE PROTOCOL dans un dossier
//! temporaire et vérifie le cycle install → uninstall de bout en bout.

use std::fs;
use std::path::PathBuf;

use mlkit::installer;

/// Crée un faux dossier de jeu minimal et renvoie sa racine.
fn make_fake_game(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mlsetup_it_{tag}"));
    let _ = fs::remove_dir_all(&root);
    let data = root.join("BrokeProtocol_Data");
    fs::create_dir_all(data.join("Managed")).unwrap();

    // ScriptingAssemblies.json minimal (comme le vrai, en plus court).
    fs::write(
        data.join("ScriptingAssemblies.json"),
        r#"{"names":["UnityEngine.dll","Scripts.dll"],"types":[2,16]}"#,
    )
    .unwrap();

    // RuntimeInitializeOnLoads.json minimal.
    fs::write(
        data.join("RuntimeInitializeOnLoads.json"),
        r#"{"root":[{"assemblyName":"Unity.InputSystem","nameSpace":"UnityEngine.InputSystem","className":"InputSystem","methodName":"RunInitialUpdate","loadTypes":1,"isUnityClass":true}]}"#,
    )
    .unwrap();

    root
}

#[test]
fn cycle_install_puis_uninstall() {
    let game = make_fake_game("cycle");
    let data = game.join("BrokeProtocol_Data");
    let managed = data.join("Managed");
    let sa = data.join("ScriptingAssemblies.json");
    let ri = data.join("RuntimeInitializeOnLoads.json");

    // --- Installation ---
    assert!(!installer::is_installed(&game));
    let log = installer::install(&game).expect("install OK");
    assert!(log.iter().any(|l| l.contains("succès")), "log: {log:?}");

    // DLL copiées.
    assert!(managed.join("ModLoader.dll").is_file());
    assert!(managed.join("!_0Harmony.dll").is_file());
    assert!(installer::is_installed(&game));

    // Sauvegardes créées.
    assert!(data.join("ScriptingAssemblies.json.modloader.bak").is_file());
    assert!(data.join("RuntimeInitializeOnLoads.json.modloader.bak").is_file());

    // JSON patchés.
    let sa_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sa).unwrap()).unwrap();
    let names = sa_json["names"].as_array().unwrap();
    let types = sa_json["types"].as_array().unwrap();
    assert!(names.iter().any(|n| n == "ModLoader.dll"));
    assert!(names.iter().any(|n| n == "!_0Harmony.dll"));
    assert_eq!(names.len(), types.len(), "names/types désynchronisés");

    let ri_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&ri).unwrap()).unwrap();
    let count = ri_json["root"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["assemblyName"] == "ModLoader")
        .count();
    assert_eq!(count, 1);

    // --- Réinstallation : idempotent, aucun doublon ---
    installer::install(&game).expect("réinstall OK");
    let sa_json2: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sa).unwrap()).unwrap();
    assert_eq!(sa_json2["names"].as_array().unwrap().len(), names.len());

    // --- Désinstallation : retour à l'état vanilla ---
    installer::uninstall(&game).expect("uninstall OK");
    assert!(!managed.join("ModLoader.dll").is_file());
    assert!(!managed.join("!_0Harmony.dll").is_file());
    assert!(!installer::is_installed(&game));

    let sa_restored: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sa).unwrap()).unwrap();
    assert_eq!(sa_restored["names"].as_array().unwrap().len(), 2);
    assert!(!sa_restored["names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n == "ModLoader.dll"));

    fs::remove_dir_all(&game).ok();
}

#[test]
fn refuse_dossier_invalide() {
    let bad = std::env::temp_dir().join("mlsetup_it_bad_not_a_game");
    fs::create_dir_all(&bad).unwrap();
    assert!(installer::install(&bad).is_err());
    fs::remove_dir_all(&bad).ok();
}
