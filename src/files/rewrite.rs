//! Cleanup and requests for a file's text. Cleanup runs in pieces of about
//! `cleanup_pieces::PIECE_WORDS` words, as dictation's does, so it fits the
//! built-in engine's usual window at any length. A request reads the whole
//! text when it fits a window: the built-in engine counts its tokens and
//! takes the usual 8K window, else the wide 32K one; past that the text is
//! worked in three steps (summaries of its parts, an outline from them, then
//! each part with the outline) and the parts joined.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use color_eyre::eyre::{Result, eyre};

use crate::app_settings::{CleanupRunsOn, CleanupServerSettings, DictationMode};
use crate::dictation::cleanup_pieces::{PIECE_WORDS, split};
use crate::dictation::cleanup_server::{self, Provider, Server, engine, local_models::LocalModel};
use crate::dictation::vocabulary::VocabularyEntry;

/// One request's longest wait, a whole wide window's answer included.
const DEADLINE: Duration = Duration::from_secs(240);
/// The share of a window a request may fill, leaving room for the template
/// and rounding.
const FILL: f32 = 0.9;
/// A part's size in the three steps, in words: about 7K tokens of Russian
/// in the wide window.
const STEP_PART_WORDS: usize = 2_500;
/// Parts for a server whose window isn't known.
const SMALL_STEP_PART_WORDS: usize = 1_000;

/// Where cleanup and requests run.
#[derive(Clone, Debug)]
pub enum Engine {
    Server(Server),
    /// OpenCode with this mode's model.
    OpenCode(DictationMode),
}

impl Engine {
    /// `choice` as Files' Settings saves it (`local:<id>`, `openrouter`,
    /// `custom`, `opencode`), or dictation's AI cleanup when it is `None`.
    pub fn resolve(
        choice: Option<&str>,
        server: &CleanupServerSettings,
        server_on: bool,
        mode: &DictationMode,
    ) -> Self {
        let runs_on = |runs_on: CleanupRunsOn| CleanupServerSettings {
            runs_on,
            ..server.clone()
        };
        let chosen = match choice {
            None if server_on => Server::from_settings(server),
            None | Some("opencode") => None,
            Some("openrouter") => Server::from_settings(&runs_on(CleanupRunsOn::Openrouter)),
            Some("custom") => Server::from_settings(&runs_on(CleanupRunsOn::Custom)),
            Some(other) => {
                let id = other.strip_prefix("local:").unwrap_or(other);
                Server::from_settings(&CleanupServerSettings {
                    runs_on: CleanupRunsOn::Local,
                    local_model: id.into(),
                    ..server.clone()
                })
            }
        };
        match chosen {
            Some(server) => Self::Server(server),
            None => Self::OpenCode(mode.clone()),
        }
    }

    /// The model a version names: Qwen3 4B, claude-sonnet-4.6, OpenCode's.
    pub fn model_name(&self) -> String {
        match self {
            Self::Server(server) if server.provider == Provider::Local => {
                cleanup_server::local_models::chosen(&server.model)
                    .name
                    .into()
            }
            Self::Server(server) => short_model(&server.model),
            Self::OpenCode(mode) => mode
                .post_processing
                .model
                .as_deref()
                .map(short_model)
                .unwrap_or_else(|| "OpenCode".into()),
        }
    }

    fn local(&self) -> Option<&'static LocalModel> {
        match self {
            Self::Server(server) if server.provider == Provider::Local => {
                Some(cleanup_server::local_models::chosen(&server.model))
            }
            _ => None,
        }
    }

    /// The window a whole text is judged against, in tokens, when no
    /// tokenizer is at hand: OpenRouter's and OpenCode's models read far
    /// more than a file's text, an unknown server about 8K.
    fn assumed_window(&self) -> usize {
        match self {
            Self::Server(server) if server.provider == Provider::Custom => 8_192,
            _ => 128_000,
        }
    }

    fn generate(
        &self,
        context: u32,
        instructions: &str,
        input: &str,
        cancelled: &AtomicBool,
    ) -> Result<String> {
        if cancelled.load(Ordering::Acquire) {
            return Err(eyre!("stopped"));
        }
        match self {
            Self::Server(server) => cleanup_server::generate_in_window(
                server,
                context,
                instructions,
                input,
                DEADLINE,
                cancelled,
            ),
            Self::OpenCode(mode) => crate::dictation::processor::generate_for_mode(
                &format!("{instructions}\n\n{input}"),
                mode,
                DEADLINE,
                cancelled,
            ),
        }
        .map(|text| text.trim().to_string())
    }
}

/// `anthropic/claude-sonnet-4.6` → `claude-sonnet-4.6`.
fn short_model(model: &str) -> String {
    model.rsplit('/').next().unwrap_or(model).to_string()
}

/// What a cleanup or request made.
#[derive(Debug, Default, PartialEq)]
pub struct Made {
    pub text: String,
    /// The built-in engine's window it ran in, in thousands of tokens.
    pub window_k: Option<u32>,
    /// Worked in three steps over this many parts.
    pub parts: Option<usize>,
    /// Why some of it kept its input.
    pub failed: Option<String>,
}

/// Cleanup's instructions: the prompt (the Global mode's or Files' own), the
/// vocabulary, and the answer's form.
pub fn cleanup_instructions(prompt: &str, vocabulary: &[VocabularyEntry]) -> String {
    format!(
        "You clean up a transcript of a recording, one part at a time. Follow these \
         instructions:\n{}\n\n{}Return only the cleaned text of this part, in its language, \
         without an explanation, a label, or a Markdown fence.",
        prompt.trim(),
        crate::dictation::processor::vocabulary_lines(vocabulary)
    )
}

/// Cleans `text` piece by piece; a piece that fails keeps its text.
/// `progress` hears the fraction done.
pub fn clean(
    engine: &Engine,
    instructions: &str,
    text: &str,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<Made> {
    let pieces = split(text, PIECE_WORDS);
    let mut made = String::new();
    let mut failures = Vec::new();
    for (index, piece) in pieces.iter().enumerate() {
        let body = piece.trim_end();
        let tail = &piece[body.len()..];
        let input = format!("Transcript:\n<transcript>\n{body}\n</transcript>");
        match engine.generate(engine::CONTEXT_TOKENS, instructions, &input, cancelled) {
            Ok(cleaned) if !cleaned.is_empty() => {
                made.push_str(&cleaned);
                made.push_str(if tail.is_empty() { "" } else { "\n\n" });
            }
            _ if cancelled.load(Ordering::Acquire) => return Err(eyre!("stopped")),
            Ok(_) => {
                failures.push("the model returned empty text".to_string());
                made.push_str(piece);
            }
            Err(error) => {
                failures.push(error.to_string());
                made.push_str(piece);
            }
        }
        progress((index + 1) as f32 / pieces.len().max(1) as f32);
    }
    if failures.len() == pieces.len() {
        return Err(eyre!("{}", failures[0]));
    }
    Ok(Made {
        text: made.trim().to_string(),
        window_k: cleanup_window_k(engine),
        parts: None,
        failed: (!failures.is_empty()).then(|| {
            format!(
                "{} of {} parts kept as transcribed: {}",
                failures.len(),
                pieces.len(),
                failures[0]
            )
        }),
    })
}

fn request_instructions(vocabulary: &[VocabularyEntry]) -> String {
    format!(
        "You rework a text as the user asks. Do exactly what the request says, keep the facts \
         and numbers of the text, and answer in the text's language unless the request says \
         otherwise. Use Markdown where structure helps (headings, lists, tables). Return only \
         the result, without a preamble or a closing remark.\n\n{}",
        crate::dictation::processor::vocabulary_lines(vocabulary)
    )
    .trim_end()
    .to_string()
}

fn request_input(prompt: &str, text: &str) -> String {
    format!("Request: {prompt}\n\nText:\n<text>\n{text}\n</text>")
}

/// How a request reads its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fit {
    /// Whole, in a window of this many tokens.
    Whole(u32),
    /// In three steps, with parts of this many words in this window.
    Steps(u32, usize),
}

/// The window a request needs: its instructions, the text, and room for an
/// answer as long as the text (at least 1K tokens, at most 8K).
fn fit(prompt_tokens: usize, text_tokens: usize, local: bool, assumed: usize) -> Fit {
    let need = prompt_tokens + text_tokens + text_tokens.clamp(1_024, 8_192);
    let fits = |window: u32| need as f32 <= window as f32 * FILL;
    if local {
        if fits(engine::CONTEXT_TOKENS) {
            Fit::Whole(engine::CONTEXT_TOKENS)
        } else if fits(engine::WIDE_CONTEXT_TOKENS) {
            Fit::Whole(engine::WIDE_CONTEXT_TOKENS)
        } else {
            Fit::Steps(engine::WIDE_CONTEXT_TOKENS, STEP_PART_WORDS)
        }
    } else if need as f32 <= assumed as f32 * FILL {
        Fit::Whole(engine::CONTEXT_TOKENS)
    } else if assumed <= 8_192 {
        Fit::Steps(engine::CONTEXT_TOKENS, SMALL_STEP_PART_WORDS)
    } else {
        Fit::Steps(engine::CONTEXT_TOKENS, STEP_PART_WORDS)
    }
}

/// About how many tokens `text` is without a tokenizer: three characters a
/// token, which overcounts English and fits Russian.
fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 3 + 1
}

/// The built-in engine's window for cleanup, in K tokens: always the usual
/// one, since cleanup goes in pieces.
pub fn cleanup_window_k(engine: &Engine) -> Option<u32> {
    engine.local().map(|_| window_k(engine::CONTEXT_TOKENS))
}

/// Applies `prompt` to `text`: whole when it fits a window, else in three
/// steps. `progress` hears the fraction done and, once known, the parts and
/// the built-in engine's window in K tokens.
pub fn request(
    engine: &Engine,
    prompt: &str,
    text: &str,
    vocabulary: &[VocabularyEntry],
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32, Option<usize>, Option<u32>),
) -> Result<Made> {
    let instructions = request_instructions(vocabulary);
    let input = request_input(prompt, text);
    let local = engine.local();
    let fit = match local {
        Some(model) => {
            let whole = cleanup_server::count_tokens(
                model,
                &format!("{instructions}\n{input}"),
                DEADLINE,
                cancelled,
            )?;
            let text_tokens = cleanup_server::count_tokens(model, text, DEADLINE, cancelled)?;
            fit(whole - text_tokens.min(whole), text_tokens, true, 0)
        }
        None => fit(
            estimate_tokens(&instructions) + estimate_tokens(prompt) + 40,
            estimate_tokens(text),
            false,
            engine.assumed_window(),
        ),
    };
    match fit {
        Fit::Whole(window) => {
            let shown = local.map(|_| ran_window_k(window));
            progress(0.05, None, shown);
            let text = engine.generate(window, &instructions, &input, cancelled)?;
            if text.is_empty() {
                return Err(eyre!("the model returned empty text"));
            }
            Ok(Made {
                text,
                window_k: local.map(|_| ran_window_k(window)),
                parts: None,
                failed: None,
            })
        }
        Fit::Steps(window, part_words) => {
            let generate = |instructions: &str, input: &str| {
                engine.generate(window, instructions, input, cancelled)
            };
            let shown = local.map(|_| ran_window_k(window));
            let made = in_steps(
                &generate,
                part_words,
                prompt,
                text,
                &instructions,
                cancelled,
                &mut |fraction, parts| progress(fraction, parts, shown),
            )?;
            Ok(Made {
                window_k: shown,
                ..made
            })
        }
    }
}

/// The window a run used: the one asked for, or the wider one a running
/// server already had.
fn ran_window_k(asked: u32) -> u32 {
    window_k(engine::running_context().unwrap_or(asked).max(asked))
}

fn window_k(tokens: u32) -> u32 {
    tokens / 1_024
}

/// The three steps: a summary of each part, an outline of the whole from
/// them, then each part with the outline, joined.
fn in_steps(
    generate: &dyn Fn(&str, &str) -> Result<String>,
    part_words: usize,
    prompt: &str,
    text: &str,
    instructions: &str,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32, Option<usize>),
) -> Result<Made> {
    let parts = split(text, part_words);
    let count = parts.len();
    let calls = (count * 2 + 1) as f32;
    let mut done = 0.0;
    let mut step = |progress: &mut dyn FnMut(f32, Option<usize>)| {
        done += 1.0;
        progress(done / calls, Some(count));
    };
    progress(0.0, Some(count));
    let summarize = "Summarize this part of a longer text in 5 to 10 short bullet points, in \
                     its language, keeping names, numbers, and decisions. Return only the \
                     bullet points.";
    let mut summaries = Vec::with_capacity(count);
    for part in &parts {
        let input = format!("Part of the text:\n<text>\n{}\n</text>", part.trim());
        summaries.push(generate(summarize, &input)?);
        step(progress);
    }
    let joined = summaries
        .iter()
        .enumerate()
        .map(|(index, summary)| format!("Part {} of {count}:\n{summary}", index + 1))
        .collect::<Vec<_>>()
        .join("\n\n");
    let outline_instructions = "You get the summaries of the parts of one long text, in order. \
         Write a short outline of the whole text, in its language: its topics in order and \
         what matters for the user's request. Return only the outline.";
    let outline = generate(
        outline_instructions,
        &format!("User's request: {prompt}\n\nSummaries:\n{joined}"),
    )?;
    step(progress);
    let mut results = Vec::with_capacity(count);
    let mut failures = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        let input = format!(
            "Request: {prompt}\n\nThis is part {} of {count} of a longer text. Outline of the \
             whole text:\n{outline}\n\nApply the request to this part only, so the parts' \
             results join into one; don't repeat an introduction or headings another part \
             has.\n\nText:\n<text>\n{}\n</text>",
            index + 1,
            part.trim()
        );
        match generate(instructions, &input) {
            Ok(result) if !result.is_empty() => results.push(result),
            _ if cancelled.load(Ordering::Acquire) => return Err(eyre!("stopped")),
            Ok(_) => failures.push("the model returned empty text".to_string()),
            Err(error) => failures.push(error.to_string()),
        }
        step(progress);
    }
    if results.is_empty() {
        return Err(eyre!(
            "{}",
            failures.first().map_or("nothing came back", |f| f.as_str())
        ));
    }
    Ok(Made {
        text: results.join("\n\n"),
        window_k: None,
        parts: Some(count),
        failed: (!failures.is_empty()).then(|| {
            format!(
                "{} of {count} parts failed: {}",
                failures.len(),
                failures[0]
            )
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_take_the_usual_window_then_the_wide_one_then_three_steps() {
        assert_eq!(fit(300, 2_000, true, 0), Fit::Whole(8_192));
        assert_eq!(fit(300, 6_000, true, 0), Fit::Whole(32_768));
        assert_eq!(
            fit(300, 30_000, true, 0),
            Fit::Steps(32_768, STEP_PART_WORDS)
        );
        // A short text still leaves an answer room.
        assert_eq!(fit(300, 10, true, 0), Fit::Whole(8_192));
    }

    #[test]
    fn other_servers_read_the_whole_text_unless_an_unknown_one_cannot() {
        assert_eq!(fit(300, 60_000, false, 128_000), Fit::Whole(8_192));
        assert!(matches!(
            fit(300, 9_000, false, 8_192),
            Fit::Steps(_, SMALL_STEP_PART_WORDS)
        ));
    }

    #[test]
    fn engines_name_their_model() {
        let mode = DictationMode::default();
        let server = CleanupServerSettings {
            openrouter_model: "anthropic/claude-sonnet-4.6".into(),
            ..Default::default()
        };
        let openrouter = Engine::resolve(Some("openrouter"), &server, false, &mode);
        assert_eq!(openrouter.model_name(), "claude-sonnet-4.6");
        let opencode = Engine::resolve(None, &server, false, &mode);
        assert!(matches!(opencode, Engine::OpenCode(_)));
        assert_eq!(opencode.model_name(), "OpenCode");
        let local = Engine::resolve(Some("local:nope"), &server, false, &mode);
        assert!(
            local.local().is_some(),
            "an unknown id is the recommended model"
        );
    }

    #[test]
    fn three_steps_summarize_outline_and_join_the_parts() {
        let text = (0..6_000).map(|n| format!("word{n}. ")).collect::<String>();
        let parts = split(&text, STEP_PART_WORDS).len();
        assert!(parts > 1);
        let calls = std::sync::Mutex::new(0);
        let fake = |instructions: &str, input: &str| -> Result<String> {
            *calls.lock().unwrap() += 1;
            Ok(if instructions.starts_with("Summarize") {
                "- point".into()
            } else if instructions.starts_with("You get the summaries") {
                assert_eq!(input.matches("- point").count(), parts);
                assert!(input.contains("User's request: list the tasks"));
                "OUTLINE".into()
            } else {
                assert!(input.contains("OUTLINE"), "each part sees the outline");
                "result".into()
            })
        };
        let mut seen = Vec::new();
        let made = in_steps(
            &fake,
            STEP_PART_WORDS,
            "list the tasks",
            &text,
            "rework",
            &AtomicBool::new(false),
            &mut |fraction, count| seen.push((fraction, count)),
        )
        .unwrap();
        assert_eq!(made.text, vec!["result"; parts].join("\n\n"));
        assert_eq!(made.parts, Some(parts));
        assert_eq!(*calls.lock().unwrap(), parts * 2 + 1);
        assert_eq!(seen.last(), Some(&(1.0, Some(parts))));
    }
}
