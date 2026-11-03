//! The models a cleanup server offers, for the model field's list:
//! `GET {base}/models`, the OpenAI-compatible listing LM Studio (its
//! downloaded models), Ollama (its pulled models), and OpenRouter (its
//! catalog, which needs no key) all serve. Embedding models and models that
//! can't answer with text are left out, since a rewrite can't use them.

use std::time::Duration;

use color_eyre::eyre::{Result, eyre};
use serde::Deserialize;

use super::{Exchange, Provider, Server, api_base, is_loopback};

/// How long the list waits: a local server answers at once, OpenRouter's
/// catalog is a few megabytes.
pub const LIST_DEADLINE: Duration = Duration::from_secs(15);
/// OpenRouter's catalog with descriptions runs to a few megabytes.
const MAX_LIST_BYTES: u64 = 32 * 1024 * 1024;

/// One model a server offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerModel {
    /// What a request names: `gemma-4-e4b-it-mlx@4bit`, `google/gemma-3-4b-it`.
    pub id: String,
    /// A display name when the server gives one (OpenRouter's "Google: Gemma 3 4B").
    pub name: String,
}

/// The models `server` offers, in its own order. Blocks for at most
/// [`LIST_DEADLINE`]; the window calls it on its own thread.
pub fn list(server: &Server, key: Option<&str>) -> Result<Vec<ServerModel>> {
    let name = server.name();
    if server.url.trim().is_empty() {
        return Err(eyre!("{name} has no URL yet"));
    }
    let url = format!("{}/models", api_base(&server.url));
    let parsed = url::Url::parse(&url).map_err(|_| eyre!("{name}'s URL isn't valid: {url}"))?;
    let body = Exchange {
        loopback: is_loopback(&parsed),
        url,
        key: key
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned),
        openrouter: server.provider == Provider::OpenRouter,
        body: None,
        timeout: LIST_DEADLINE,
        limit: MAX_LIST_BYTES,
        name: name.clone(),
    }
    .run()?;
    parse(&name, &body)
}

#[derive(Deserialize)]
struct Listing {
    #[serde(default)]
    data: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    architecture: Option<Architecture>,
}

#[derive(Deserialize)]
struct Architecture {
    #[serde(default)]
    output_modalities: Option<Vec<String>>,
}

fn parse(name: &str, body: &[u8]) -> Result<Vec<ServerModel>> {
    let listing: Listing = serde_json::from_slice(body)
        .map_err(|error| eyre!("{name} sent a model list Hex can't read: {error}"))?;
    let mut seen = std::collections::HashSet::new();
    Ok(listing
        .data
        .into_iter()
        .filter(|entry| !entry.id.trim().is_empty())
        .filter(|entry| !entry.id.to_ascii_lowercase().contains("embed"))
        .filter(|entry| {
            entry
                .architecture
                .as_ref()
                .and_then(|architecture| architecture.output_modalities.as_ref())
                .is_none_or(|outputs| outputs.iter().any(|output| output == "text"))
        })
        .filter(|entry| seen.insert(entry.id.clone()))
        .map(|entry| ServerModel {
            name: entry
                .name
                .filter(|shown| shown.trim() != entry.id.trim())
                .unwrap_or_default(),
            id: entry.id.trim().to_owned(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_keeps_chat_models_in_order_and_names_them() {
        // LM Studio: its downloaded models, an embedding model among them.
        let lm_studio = br#"{"object":"list","data":[
            {"id":"google/gemma-4-e4b","object":"model"},
            {"id":"gemma-4-e4b-it-mlx@4bit","object":"model"},
            {"id":"text-embedding-nomic-embed-text-v1.5","object":"model"},
            {"id":"gemma-4-e4b-it-mlx@4bit","object":"model"}]}"#;
        let ids: Vec<_> = parse("LM Studio", lm_studio)
            .unwrap()
            .into_iter()
            .map(|model| model.id)
            .collect();
        assert_eq!(ids, ["google/gemma-4-e4b", "gemma-4-e4b-it-mlx@4bit"]);

        // OpenRouter: display names, and an image model left out.
        let openrouter = br#"{"data":[
            {"id":"google/gemma-3-4b-it","name":"Google: Gemma 3 4B",
             "architecture":{"output_modalities":["text"]}},
            {"id":"openai/gpt-image-1","name":"OpenAI: GPT Image",
             "architecture":{"output_modalities":["image"]}},
            {"id":"x/same","name":"x/same"}]}"#;
        assert_eq!(
            parse("OpenRouter", openrouter).unwrap(),
            [
                ServerModel {
                    id: "google/gemma-3-4b-it".into(),
                    name: "Google: Gemma 3 4B".into()
                },
                ServerModel {
                    id: "x/same".into(),
                    name: String::new()
                },
            ]
        );
        assert!(parse("Ollama", b"<html>").is_err());
        assert!(parse("Ollama", br#"{"object":"list"}"#).unwrap().is_empty());
    }
}
