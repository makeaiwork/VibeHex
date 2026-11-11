//! What Hex keeps out of its own Codex app-server: nothing of the person's
//! Codex setup that could act, remember, read a project, make a sound, or
//! start a tool of theirs.
//!
//! Their MCP servers take two steps, because Codex merges `-c` overrides
//! into their config.toml rather than replacing it: `mcp_servers={}` adds
//! nothing and removes nothing. Each start turns every server Hex knows of
//! off by name ([`overrides`]); once it is up, Hex asks Codex for the
//! config it really runs with (`config/read`, every layer merged: system,
//! user, project, managed) and, while a server there is still on
//! ([`still_on`]), starts again with it off too ([`learn`]). MCP servers
//! start only with a thread, so none runs in between. Hex never passes
//! `--profile`, so no profile layer applies.

use std::sync::Mutex;

use serde_json::{Value, json};

/// Overrides for Hex's own app-server, besides the MCP servers. Feature
/// flags go through `-c` because `--disable` refuses a flag a Codex
/// version doesn't know.
pub(super) const ISOLATION: &[&str] = &[
    "notify=[]",
    "project_doc_max_bytes=0",
    "web_search=\"disabled\"",
    "features.hooks=false",
    "features.memories=false",
    "features.apps=false",
    "features.plugins=false",
    "features.multi_agent=false",
    "features.shell_tool=false",
    "features.unified_exec=false",
    "features.computer_use=false",
    "features.browser_use=false",
    "features.in_app_browser=false",
    "features.image_generation=false",
    "features.goals=false",
    "features.code_mode_host=false",
    "features.js_repl=false",
    "features.view_image=false",
];

/// What an Assistant thread with Web on sets for itself, over the
/// process's isolation: live web search, and code mode, which Codex 0.167
/// searches through. Code mode runs JavaScript in a V8 isolate with no
/// Node, file system, network, or console of its own; it reaches only the
/// thread's other tools, and with [`ISOLATION`] those are no shell, exec,
/// MCP server, app, plugin, browser, computer use, or image reading; a
/// patch can't write in the read-only sandbox, where no approval is ever
/// given. Dotted keys only: an object under `features` would replace the
/// process's whole `features` table.
pub(super) fn searching() -> Value {
    json!({ "web_search": "live", "features.code_mode_host": true })
}

/// The person's MCP servers Codex has reported on, by name, so the next
/// start turns them off from the outset.
static SERVERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The servers a start turns off.
pub(super) fn known() -> Vec<String> {
    SERVERS
        .lock()
        .map(|servers| servers.clone())
        .unwrap_or_default()
}

/// Adds servers Codex reported on; true when one was new, so another start
/// can turn it off.
pub(super) fn learn(on: &[String]) -> bool {
    let Ok(mut servers) = SERVERS.lock() else {
        return false;
    };
    let mut new = false;
    for name in on {
        if !servers.contains(name) {
            servers.push(name.clone());
            new = true;
        }
    }
    new
}

/// Forgets them all: Codex refuses to start over a server Hex names that
/// has since left the person's config.
pub(super) fn forget() {
    if let Ok(mut servers) = SERVERS.lock() {
        servers.clear();
    }
}

/// Every `-c` override of a start: [`ISOLATION`], then `servers` off.
pub(super) fn overrides(servers: &[String]) -> Vec<String> {
    let mut all: Vec<String> = ISOLATION
        .iter()
        .map(|&setting| setting.to_owned())
        .collect();
    all.push(servers_off(servers));
    all
}

/// `mcp_servers={"node_repl"={enabled=false}}`: an inline table merges
/// into the person's servers and takes any name, where Codex's dotted `-c`
/// paths split on every dot and keep quotes as part of the name.
fn servers_off(servers: &[String]) -> String {
    let entries: Vec<String> = servers
        .iter()
        .map(|name| format!("{}={{enabled=false}}", quoted(name)))
        .collect();
    format!("mcp_servers={{{}}}", entries.join(","))
}

/// A TOML basic string.
fn quoted(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    out.push('"');
    for c in name.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The MCP servers still on in `config/read`'s result: every one under
/// `config.mcp_servers` whose `enabled` isn't false.
pub(super) fn still_on(read: &Value) -> Vec<String> {
    let Some(servers) = read
        .pointer("/config/mcp_servers")
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    servers
        .iter()
        .filter(|(_, server)| server.get("enabled").and_then(Value::as_bool) != Some(false))
        .map(|(name, _)| name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `config/read` as Codex answers it for a person with Codex app's
    /// `node_repl` on, Computer Use off, and one more server written with
    /// a dot in its name and no `enabled` at all.
    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/codex/config-read-mcp.json"
        ))
        .unwrap()
    }

    #[test]
    fn a_server_left_on_in_the_persons_config_is_found() {
        let mut on = still_on(&fixture());
        on.sort();
        assert_eq!(on, ["node_repl", "odd.name"]);
        assert!(still_on(&serde_json::json!({ "config": {} })).is_empty());
        assert!(still_on(&Value::Null).is_empty());
    }

    /// node_repl from the fixture ends up off, by a name Codex reads whole.
    #[test]
    fn every_server_found_is_turned_off_by_name() {
        let on = still_on(&fixture());
        let all = overrides(&on);
        let last = all.last().unwrap();
        assert!(last.starts_with("mcp_servers={"), "{last}");
        assert!(last.contains(r#""node_repl"={enabled=false}"#), "{last}");
        assert!(last.contains(r#""odd.name"={enabled=false}"#), "{last}");
        assert_eq!(
            servers_off(&["with space".into(), "q\"uote\\d".into()]),
            r#"mcp_servers={"with space"={enabled=false},"q\"uote\\d"={enabled=false}}"#
        );
        assert_eq!(overrides(&[]).last().unwrap(), "mcp_servers={}");
        // The rest of the isolation comes first, unchanged.
        assert_eq!(&all[..ISOLATION.len()], ISOLATION);
        assert!(
            !ISOLATION
                .iter()
                .any(|setting| setting.starts_with("mcp_servers"))
        );
    }

    /// Only web search and code mode open up, each by its own key; the
    /// process keeps both off, and everything else off, for every thread.
    #[test]
    fn a_searching_thread_opens_web_search_and_code_mode_alone() {
        let config = searching();
        let keys: Vec<&String> = config.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["features.code_mode_host", "web_search"]);
        assert!(ISOLATION.contains(&"web_search=\"disabled\""));
        assert!(ISOLATION.contains(&"features.code_mode_host=false"));
        for off in [
            "features.shell_tool=false",
            "features.unified_exec=false",
            "features.js_repl=false",
            "features.apps=false",
            "features.plugins=false",
            "features.browser_use=false",
            "features.computer_use=false",
            "features.view_image=false",
        ] {
            assert!(ISOLATION.contains(&off), "{off}");
        }
    }

    #[test]
    fn known_servers_are_learned_once_and_forgotten_together() {
        forget();
        assert!(learn(&["node_repl".into()]));
        assert!(!learn(&["node_repl".into()]), "nothing new");
        assert!(learn(&["node_repl".into(), "other".into()]));
        assert_eq!(known(), ["node_repl", "other"]);
        forget();
        assert!(known().is_empty());
    }
}
