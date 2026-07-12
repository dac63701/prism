use std::path::PathBuf;

/// Install Prism's official CS2 Game State Integration config when the game is
/// installed in one of Steam's standard library locations.
pub fn ensure_gsi_config(port: u16) -> Result<Option<PathBuf>, String> {
    let contents = format!(
        "\"Prism Game State Integration\"\n{{\n  \"uri\" \"http://127.0.0.1:{port}\"\n  \"timeout\" \"5.0\"\n  \"buffer\" \"0.1\"\n  \"throttle\" \"0.1\"\n  \"heartbeat\" \"10.0\"\n  \"data\"\n  {{\n    \"provider\" \"1\"\n    \"map\" \"1\"\n    \"round\" \"1\"\n    \"player_id\" \"1\"\n    \"player_state\" \"1\"\n    \"player_match_stats\" \"1\"\n  }}\n}}\n"
    );

    for directory in cs2_config_directories() {
        if !directory.is_dir() {
            continue;
        }
        let path = directory.join("gamestate_integration_prism.cfg");
        if std::fs::read_to_string(&path).ok().as_deref() == Some(contents.as_str()) {
            return Ok(Some(path));
        }
        std::fs::write(&path, &contents)
            .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        return Ok(Some(path));
    }
    Ok(None)
}

fn cs2_config_directories() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for program_files in ["PROGRAMFILES(X86)", "PROGRAMFILES"] {
        if let Some(root) = std::env::var_os(program_files) {
            paths.push(
                PathBuf::from(root)
                    .join("Steam")
                    .join("steamapps/common/Counter-Strike Global Offensive/game/csgo/cfg"),
            );
        }
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(
            home.join(
                ".steam/steam/steamapps/common/Counter-Strike Global Offensive/game/csgo/cfg",
            ),
        );
    }
    paths
}
