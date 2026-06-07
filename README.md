# ModLoader Setup — BROKE PROTOCOL

Installeur graphique (un seul `.exe` autonome) qui injecte le **ModLoader** et
**Harmony** dans une installation de BROKE PROTOCOL.

## Ce que fait l'installeur

1. **Détecte le jeu** automatiquement via Steam (registre Windows +
   `libraryfolders.vdf`, donc même si le jeu est sur un autre disque). Bouton
   *Parcourir…* en secours.
2. **Modifie `BrokeProtocol_Data/ScriptingAssemblies.json`** : ajoute
   `!_0Harmony.dll` et `ModLoader.dll` (type `16`) à la liste, sans toucher au
   reste (résiste aux mises à jour du jeu).
3. **Modifie `BrokeProtocol_Data/RuntimeInitializeOnLoads.json`** : enregistre le
   point d'entrée `ModLoader.Core.Start`.
4. **Copie `ModLoader.dll` et `!_0Harmony.dll`** dans `BrokeProtocol_Data/Managed`.

Les deux DLL sont **embarquées dans le `.exe`** : aucun fichier externe requis
pour distribuer l'installeur.

### Sécurité / robustesse

- **Sauvegarde** : chaque JSON est copié en `*.modloader.bak` avant modification.
- **Idempotent** : relancer l'installeur ne crée aucun doublon.
- **Désinstallation** : un bouton restaure les `.bak` et supprime les DLL ajoutées.
- **Élévation UAC** : l'installeur demande les droits administrateur au lancement
  (nécessaire pour écrire dans `C:\Program Files (x86)\Steam\...`).

## Compiler

```sh
cargo build --release
```

Le binaire est généré dans `target/release/ModLoaderSetup.exe`.

À chaque build, `build.rs` recopie automatiquement les dernières DLL depuis
`../ModLoader/ModLoader/bin/Debug` vers `assets/` avant de les embarquer. Si ce
dossier n'existe pas (build sur une autre machine), les copies présentes dans
`assets/` sont conservées.

## Tester

```sh
cargo test
```

- Tests unitaires (`src/`) : parsing du `.vdf` Steam, fusion JSON idempotente.
- Test d'intégration (`tests/integration.rs`) : cycle complet install → réinstall
  → désinstall sur un faux dossier de jeu temporaire.

## Architecture

| Fichier              | Rôle                                                        |
| -------------------- | ----------------------------------------------------------- |
| `src/main.rs`        | Interface graphique (egui/eframe).                          |
| `src/lib.rs`         | Point d'entrée de la bibliothèque (logique testable).       |
| `src/steam.rs`       | Détection du dossier du jeu via Steam.                      |
| `src/installer.rs`   | Backup, fusion des JSON, copie des DLL, désinstallation.    |
| `src/assets.rs`      | DLL embarquées (`include_bytes!`).                          |
| `build.rs`           | Rafraîchit les DLL + embarque le manifest Windows (UAC/DPI).|
