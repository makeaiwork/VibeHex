//! Reads the change lines out of a reply. The model writes, after its
//! prose and before `Sources:`, fenced blocks of one JSON object per line:
//!
//! ````text
//! ```hex-change
//! {"key":"labs.smart_spacing","value":true,"why":"..."}
//! ```
//! ````
//!
//! Weak models get this wrong in every way, so the reader forgives what it
//! can: `json` and `~~~` fences, an array instead of lines, trailing commas,
//! `True`/`False`, and a tagged line, `CHANGE labs.smart_spacing = on: why`.
//! Each line is read on its own, so one bad line drops only itself. What a
//! value means for its key (`"on"`, a choice's display name) is `names`'.

use serde_json::{Map, Value};

/// The fences a reply's change lines come in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fence {
    /// Each line one card.
    Change,
    /// Every line one checklist.
    Plan,
    /// One question per line, with option chips (setup).
    Ask,
    /// One vocabulary term per line, all in one card.
    Vocab,
}

/// Every line a plan's reply holds, as a line of its one list.
pub fn as_plan(proposals: &mut [Proposal]) {
    for proposal in proposals {
        proposal.plan = true;
    }
}

/// One change line, as the model wrote it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proposal {
    pub key: String,
    pub value: Option<Value>,
    /// Values to choose from; `suggest` is the one it recommends.
    pub options: Vec<Value>,
    pub suggest: Option<Value>,
    pub why: String,
    /// A plan line that starts unchecked.
    pub optional: bool,
    /// From a `hex-plan` block.
    pub plan: bool,
    /// The line as written, shortened, for a refusal.
    pub line: String,
}

/// A reply without its change blocks, and what they held.
#[derive(Debug, Default, PartialEq)]
pub struct Parsed {
    /// What the pane shows.
    pub body: String,
    pub proposals: Vec<Proposal>,
    /// Lines in a change block that weren't a change.
    pub malformed: Vec<String>,
    /// `hex-ask` questions, as written (setup reads them).
    pub asks: Vec<Value>,
}

/// The longest a refused line is kept.
const LINE_CHARS: usize = 160;

/// Splits `reply` into what shows and the change lines it holds.
pub fn parse(reply: &str) -> Parsed {
    let lines: Vec<&str> = reply.lines().collect();
    let mut parsed = Parsed::default();
    let mut body: Vec<&str> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some((marker, fence)) = opens(line, lines.get(index + 1).copied()) {
            let start = index + 1;
            let end = (start..lines.len())
                .find(|&at| lines[at].trim() == marker || lines[at].trim().starts_with(marker))
                .unwrap_or(lines.len());
            read_block(&lines[start..end], fence, &mut parsed);
            index = end + 1;
            continue;
        }
        if let Some(proposal) = tagged(line) {
            parsed.proposals.push(proposal);
        } else {
            body.push(line);
        }
        index += 1;
    }
    parsed.body = tidy(&body.join("\n"));
    parsed
}

/// The closing marker and kind of the fence `line` opens, if it opens a
/// change block. A `json` (or bare) fence counts only when its next line
/// starts a change object.
fn opens(line: &str, next: Option<&str>) -> Option<(&'static str, Fence)> {
    let trimmed = line.trim();
    let (marker, info) = match trimmed.strip_prefix("```") {
        Some(info) => ("```", info),
        None => ("~~~", trimmed.strip_prefix("~~~")?),
    };
    let fence = match info.trim().to_ascii_lowercase().as_str() {
        "hex-change" | "hex-changes" => Fence::Change,
        "hex-plan" => Fence::Plan,
        "hex-ask" => Fence::Ask,
        "hex-vocab" | "hex-vocabulary" => Fence::Vocab,
        "json" | "jsonl" | "" if next.is_some_and(starts_change) => Fence::Change,
        _ => return None,
    };
    Some((marker, fence))
}

/// Whether `line` starts a change object, or an array of them.
pub(crate) fn starts_change(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.trim_start_matches('[');
    ["{\"key\"", "{'key'", "{\"term\"", "{\"vocabulary_add\""]
        .iter()
        .any(|start| compact.starts_with(start))
}

fn read_block(lines: &[&str], fence: Fence, parsed: &mut Parsed) {
    let text = lines.join("\n");
    // A whole array, or objects spread over lines: one value at a time.
    let objects: Vec<(String, Option<Value>)> = match loose(text.trim()) {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| (shortened(&item.to_string()), Some(item)))
            .collect(),
        Some(object @ Value::Object(_)) => vec![(shortened(text.trim()), Some(object))],
        _ => lines
            .iter()
            .map(|line| line.trim().trim_end_matches(','))
            .filter(|line| !line.is_empty() && !matches!(*line, "[" | "]"))
            .map(|line| (shortened(line), loose(line)))
            .collect(),
    };
    for (line, value) in objects {
        match (fence, value) {
            (Fence::Ask, Some(value @ Value::Object(_))) => parsed.asks.push(value),
            (_, Some(Value::Object(object))) if super::vocab_edit::is_edit(&object) => parsed
                .proposals
                .push(super::vocab_edit::proposal(&object, &line)),
            (_, Some(Value::Object(object))) if super::vocab::is_term(&object) => {
                match super::vocab::proposals(&object, fence == Fence::Plan, &line) {
                    Some(terms) => parsed.proposals.extend(terms),
                    None => parsed.malformed.push(line),
                }
            }
            (_, Some(Value::Object(object))) => match proposal(&object, fence, &line) {
                Some(proposal) => parsed.proposals.push(proposal),
                None => parsed.malformed.push(line),
            },
            _ => parsed.malformed.push(line),
        }
    }
}

fn proposal(object: &Map<String, Value>, fence: Fence, line: &str) -> Option<Proposal> {
    let text = |name: &str| object.get(name).and_then(Value::as_str).map(str::to_owned);
    let key = text("key")?.trim().to_owned();
    if key.is_empty() {
        return None;
    }
    let options = match object.get("options") {
        Some(Value::Array(options)) => options.clone(),
        _ => Vec::new(),
    };
    let value = object.get("value").cloned();
    if value.is_none() && options.is_empty() {
        return None;
    }
    Some(Proposal {
        key,
        value,
        options,
        suggest: object.get("suggest").cloned(),
        why: text("why").unwrap_or_default(),
        optional: object.get("optional").and_then(Value::as_bool) == Some(true),
        plan: fence == Fence::Plan,
        line: line.to_owned(),
    })
}

/// `CHANGE labs.smart_spacing = on: why`, a line weak models write when
/// they lose the fences.
fn tagged(line: &str) -> Option<Proposal> {
    let rest = line
        .trim()
        .trim_start_matches(['-', '*', ' '])
        .strip_prefix("CHANGE ")?;
    let (key, rest) = rest.split_once('=')?;
    let key = key.trim().trim_matches('`');
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    let (value, why) = rest.split_once(':').unwrap_or((rest, ""));
    let value = value.trim().trim_matches(['`', '"']);
    Some(Proposal {
        key: key.to_owned(),
        value: Some(loose(value).unwrap_or_else(|| Value::String(value.to_owned()))),
        why: why.trim().to_owned(),
        line: shortened(line.trim()),
        ..Proposal::default()
    })
}

/// JSON as weak models write it: trailing commas, single-quoted keys,
/// `True`, `False`, and `None` outside strings.
fn loose(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str(text) {
        return Some(value);
    }
    let mut fixed = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(open) = quote {
            match c {
                '\\' => {
                    fixed.push(c);
                    fixed.extend(chars.next());
                }
                c if c == open => {
                    quote = None;
                    fixed.push('"');
                }
                '"' => fixed.push_str("\\\""),
                c => fixed.push(c),
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                fixed.push('"');
            }
            ',' => {
                let rest: String = chars.clone().collect();
                if !rest.trim_start().starts_with(['}', ']']) {
                    fixed.push(',');
                }
            }
            c if c.is_ascii_alphabetic() => {
                let mut word = String::from(c);
                while let Some(&next) = chars.peek().filter(|next| next.is_ascii_alphanumeric()) {
                    word.push(next);
                    chars.next();
                }
                fixed.push_str(match word.as_str() {
                    "True" => "true",
                    "False" => "false",
                    "None" => "null",
                    word => word,
                });
            }
            c => fixed.push(c),
        }
    }
    serde_json::from_str(&fixed).ok()
}

fn shortened(line: &str) -> String {
    match line.char_indices().nth(LINE_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_owned(),
    }
}

/// The body with no run of blank lines left where a block was.
fn tidy(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut blank = false;
    for line in body.lines() {
        let empty = line.trim().is_empty();
        if empty && blank {
            continue;
        }
        blank = empty;
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

/// While a reply streams, where its visible part ends: before a change
/// block opens, a tagged line, or a fence still being written. `true`
/// when a change is on its way.
pub fn visible(draft: &str) -> (&str, bool) {
    let mut at = 0;
    // Inside a code block that isn't a change, its closing fence is just
    // that.
    let mut inside = false;
    let lines: Vec<&str> = draft.split_inclusive('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        let next = lines.get(index + 1).map(|next| next.trim_end());
        let trimmed = line.trim();
        let last = index + 1 == lines.len();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        let change = !inside
            && (opens(line, next).is_some()
                || trimmed.starts_with("CHANGE ")
                // A `json` fence whose next line hasn't come yet.
                || (next.is_none_or(str::is_empty)
                    && matches!(trimmed, "```json" | "```" | "~~~" | "~~~json"))
                // A fence still being written: ```, ``, `, ```he.
                || (last && !line.ends_with('\n') && unfinished_fence(trimmed)));
        if change {
            return (draft[..at].trim_end(), true);
        }
        if fence {
            inside = !inside;
        }
        at += line.len();
    }
    (draft, false)
}

fn unfinished_fence(line: &str) -> bool {
    let info = line.trim_start_matches(['`', '~']);
    let marks = line.len() - info.len();
    (1..=3).contains(&marks)
        && (info.is_empty()
            || "hex-change".starts_with(info)
            || "hex-plan".starts_with(info)
            || "hex-ask".starts_with(info)
            || "hex-vocab".starts_with(info)
            || "json".starts_with(info))
        || "CHANGE".starts_with(line)
            && !line.is_empty()
            && line.chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(parsed: &Parsed) -> Vec<&str> {
        parsed.proposals.iter().map(|p| p.key.as_str()).collect()
    }

    #[test]
    fn a_change_block_leaves_the_body_and_its_lines_become_proposals() {
        let parsed = parse(
            "Turn on Smart spacing.\n\n```hex-change\n{\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"spaces\"}\n{\"key\":\"meetings.notes.writer\",\"options\":[\"\",\"codex\"],\"suggest\":\"codex\",\"why\":\"Russian\"}\n```\n\nSources: labs/smart-spacing",
        );
        assert_eq!(
            parsed.body,
            "Turn on Smart spacing.\n\nSources: labs/smart-spacing"
        );
        assert_eq!(
            keys(&parsed),
            ["labs.smart_spacing", "meetings.notes.writer"]
        );
        assert_eq!(parsed.proposals[0].value, Some(json!(true)));
        assert_eq!(parsed.proposals[0].why, "spaces");
        assert_eq!(parsed.proposals[1].options, [json!(""), json!("codex")]);
        assert_eq!(parsed.proposals[1].suggest, Some(json!("codex")));
        assert!(parsed.malformed.is_empty());
    }

    #[test]
    fn json_and_tilde_fences_arrays_and_trailing_commas_are_read() {
        let json = parse("A.\n```json\n{\"key\":\"labs.smart_spacing\",\"value\":True,}\n```");
        assert_eq!(keys(&json), ["labs.smart_spacing"]);
        assert_eq!(json.proposals[0].value, Some(json!(true)));
        assert_eq!(json.body, "A.");
        let array = parse(
            "~~~hex-change\n[\n  {\"key\": \"a.b\", \"value\": 1},\n  {\"key\": \"c.d\", \"value\": \"on\"},\n]\n~~~",
        );
        assert_eq!(keys(&array), ["a.b", "c.d"]);
        // A json block that isn't a change stays in the answer.
        let code = parse("Like this:\n```json\n{\"name\": \"x\"}\n```");
        assert!(code.proposals.is_empty());
        assert!(code.body.contains("{\"name\": \"x\"}"));
    }

    #[test]
    fn one_bad_line_drops_only_itself() {
        let parsed = parse(
            "```hex-change\n{\"key\":\"labs.smart_spacing\",\"value\":true}\nturn on meetings please\n{\"value\":true}\n{\"key\":\"labs.files\",\"value\":false}\n```",
        );
        assert_eq!(keys(&parsed), ["labs.smart_spacing", "labs.files"]);
        assert_eq!(parsed.malformed.len(), 2);
    }

    #[test]
    fn a_tagged_line_is_a_change_and_leaves_the_body() {
        let parsed = parse("Do this.\nCHANGE labs.smart_spacing = on: adds the space\nThanks.");
        assert_eq!(keys(&parsed), ["labs.smart_spacing"]);
        assert_eq!(parsed.proposals[0].value, Some(json!("on")));
        assert_eq!(parsed.proposals[0].why, "adds the space");
        assert_eq!(parsed.body, "Do this.\nThanks.");
    }

    #[test]
    fn a_plan_and_an_ask_are_told_apart() {
        let parsed = parse(
            "```hex-plan\n{\"key\":\"labs.files\",\"value\":true,\"optional\":true}\n```\n```hex-ask\n{\"id\":\"use\",\"q\":\"What for?\",\"options\":[]}\n```",
        );
        assert!(parsed.proposals[0].plan && parsed.proposals[0].optional);
        assert_eq!(parsed.asks.len(), 1);
    }

    #[test]
    fn an_unclosed_block_runs_to_the_end() {
        let parsed = parse("A.\n```hex-change\n{\"key\":\"labs.files\",\"value\":true}");
        assert_eq!(keys(&parsed), ["labs.files"]);
        assert_eq!(parsed.body, "A.");
    }

    #[test]
    fn a_streaming_draft_stops_before_a_change() {
        assert_eq!(
            visible("Turn it on.\n```hex-change\n{\"ke"),
            ("Turn it on.", true)
        );
        assert_eq!(visible("Turn it on.\n```hex-"), ("Turn it on.", true));
        assert_eq!(visible("Turn it on.\n``"), ("Turn it on.", true));
        assert_eq!(visible("Turn it on.\n`"), ("Turn it on.", true));
        assert_eq!(visible("Turn it on.\n```json\n"), ("Turn it on.", true));
        assert_eq!(
            visible("Turn it on.\n```json\n{\"key\":\"labs.files\""),
            ("Turn it on.", true)
        );
        assert_eq!(
            visible("Turn it on.\nCHANGE labs.files"),
            ("Turn it on.", true)
        );
        assert_eq!(visible("Turn it on.\nCHA"), ("Turn it on.", true));
        // Code that isn't a change keeps streaming, and so does prose.
        let code = "Run:\n```sh\nhex labs on files\n";
        assert_eq!(visible(code), (code, false));
        let json = "Like:\n```json\n{\"name\": 1}\n";
        assert_eq!(visible(json), (json, false));
        let closed = "Run:\n```sh\nhex labs on files\n```\n";
        assert_eq!(visible(closed), (closed, false));
        assert_eq!(visible("Use `hex`"), ("Use `hex`", false));
        assert_eq!(visible("Answer"), ("Answer", false));
    }
}
