//! The words a model and the app trade while an answer reads on its own.
//!
//! A model that needs more than it was given writes, instead of an answer,
//! one fenced block of JSON lines, a call each:
//!
//! ````text
//! ```hex-read
//! {"tool":"guide_read","ids":["labs/meetings"]}
//! {"tool":"settings_get","prefix":"meetings."}
//! ```
//! ````
//!
//! The app runs the calls and asks again with the results under the
//! question, a `<read step="1">` block holding each call as written and
//! its `<result>`. A reply with such a block is a read step, whatever
//! else it holds: prose around the block is dropped. A reply without one
//! is the answer.
//!
//! Models get the block wrong the ways they get change blocks wrong, so
//! the reader forgives the same things (`changes::parse`): a `json` or
//! bare fence, `~~~`, an array instead of lines, an object spread over
//! lines, trailing commas, single quotes, a fence never closed; and two
//! of its own: `{"name":…,"arguments":{…}}`, the shape tool-calling APIs
//! use, and a reply that is nothing but call lines with no fence at all.

use serde_json::{Map, Value};

use crate::assistant::changes::parse::loose;

/// The fence a read step is written in.
pub const FENCE: &str = "hex-read";

/// The longest a call's line is kept, for the results and a refusal.
const LINE_CHARS: usize = 300;

/// One call, as the model wrote it.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub tool: String,
    /// Its arguments, by name.
    pub args: Map<String, Value>,
    /// The line as written, shortened.
    pub line: String,
}

impl Call {
    pub fn text(&self, name: &str) -> Option<&str> {
        self.args
            .get(name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }

    /// A whole number, written as one or as text.
    pub fn number(&self, name: &str) -> Option<u64> {
        match self.args.get(name)? {
            Value::Number(number) => number.as_u64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn flag(&self, name: &str) -> bool {
        matches!(self.args.get(name), Some(Value::Bool(true)))
    }

    /// A list of words: an array of them, or one string with commas.
    pub fn list(&self, name: &str) -> Vec<String> {
        let items: Vec<String> = match self.args.get(name) {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            Some(Value::String(text)) => text.split(',').map(str::to_owned).collect(),
            _ => Vec::new(),
        };
        items
            .into_iter()
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    }
}

/// What a read step asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reads {
    pub calls: Vec<Call>,
    /// Lines of the block that are no call.
    pub malformed: Vec<String>,
}

/// What `reply` asks to read, when it is a read step; none when it is an
/// answer.
pub fn reads(reply: &str) -> Option<Reads> {
    let lines: Vec<&str> = reply.lines().collect();
    let mut reads = Reads::default();
    let mut fenced = false;
    let mut index = 0;
    while index < lines.len() {
        let Some(marker) = opens_at(&lines, index) else {
            index += 1;
            continue;
        };
        let start = index + 1;
        let end = (start..lines.len())
            .find(|&at| lines[at].trim().starts_with(marker))
            .unwrap_or(lines.len());
        block(&lines[start..end], &mut reads);
        fenced = true;
        index = end + 1;
    }
    if fenced {
        return Some(reads);
    }
    // No fence: a reply that is nothing but calls, and what a fence
    // written wrong leaves around them.
    let written: Vec<&str> = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if !written.iter().all(|line| starts_call(line) || debris(line)) {
        return None;
    }
    let calls: Vec<&str> = written
        .into_iter()
        .filter(|line| starts_call(line))
        .collect();
    block(&calls, &mut reads);
    (!reads.calls.is_empty()).then_some(reads)
}

/// A line that says nothing: a fence of any kind, or brackets alone. A
/// live reply was a call, then "```" with a marker after it, then "}":
/// the person must never be shown that as an answer.
fn debris(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("```")
        || line.starts_with("~~~")
        || (!line.is_empty()
            && line
                .chars()
                .all(|c| matches!(c, '{' | '}' | '[' | ']' | ',')))
}

/// `reply` without what asks to read: its read blocks, and call lines
/// written with no fence. What is left is the answer, if there is one.
pub fn without(reply: &str) -> String {
    let lines: Vec<&str> = reply.lines().collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(marker) = opens_at(&lines, index) {
            let end = (index + 1..lines.len())
                .find(|&at| lines[at].trim().starts_with(marker))
                .unwrap_or(lines.len());
            index = end + 1;
            continue;
        }
        if !starts_call(lines[index]) {
            kept.push(lines[index]);
        }
        index += 1;
    }
    // Nothing but what a call left around it is no answer.
    if kept
        .iter()
        .all(|line| line.trim().is_empty() || debris(line))
    {
        return String::new();
    }
    kept.join("\n").trim().to_owned()
}

/// [`opens`] for line `index` of a whole reply, where a `json` fence's
/// first call may start a line or two down (`[` or `{` on a line alone).
fn opens_at(lines: &[&str], index: usize) -> Option<&'static str> {
    let ahead: String = lines[index + 1..].iter().take(3).copied().collect();
    opens(lines[index], Some(&ahead))
}

/// The closing marker of the read block `line` opens. A `json` or bare
/// fence counts only when its next line starts a call.
pub(crate) fn opens(line: &str, next: Option<&str>) -> Option<&'static str> {
    let trimmed = line.trim();
    let (marker, info) = match trimmed.strip_prefix("```") {
        Some(info) => ("```", info),
        None => ("~~~", trimmed.strip_prefix("~~~")?),
    };
    match info.trim().to_ascii_lowercase().as_str() {
        "hex-read" | "hex-reads" => Some(marker),
        "json" | "jsonl" | "" if next.is_some_and(starts_call) => Some(marker),
        _ => None,
    }
}

/// Whether `line` starts a call object, or an array of them.
pub(crate) fn starts_call(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.trim_start_matches('[');
    ["{\"tool\"", "{'tool'"]
        .iter()
        .any(|start| compact.starts_with(start))
}

fn block(lines: &[&str], reads: &mut Reads) {
    let text = lines.join("\n");
    // A whole array, or one object spread over lines: one value at a time.
    let objects: Vec<(String, Option<Value>)> = match loose(text.trim()) {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| (shortened(&item.to_string()), Some(item)))
            .collect(),
        Some(object @ Value::Object(_)) => vec![(shortened(&object.to_string()), Some(object))],
        _ => lines
            .iter()
            .map(|line| line.trim().trim_end_matches(','))
            .filter(|line| !line.is_empty() && !matches!(*line, "[" | "]"))
            .map(|line| (shortened(line), loose(line)))
            .collect(),
    };
    for (line, value) in objects {
        match value.and_then(|value| call(value, &line)) {
            Some(call) => reads.calls.push(call),
            None => reads.malformed.push(line),
        }
    }
}

/// A call from one object: `{"tool":…, …}` with its arguments beside the
/// name, or under `args`, `arguments`, or `input`; `name` for `tool`.
fn call(value: Value, line: &str) -> Option<Call> {
    let Value::Object(mut object) = value else {
        return None;
    };
    let tool = object
        .remove("tool")
        .or_else(|| object.remove("name"))?
        .as_str()?
        .trim()
        .to_ascii_lowercase();
    if tool.is_empty() {
        return None;
    }
    for nested in ["args", "arguments", "input"] {
        if let Some(Value::Object(inner)) = object.remove(nested) {
            object.extend(inner);
        }
    }
    Some(Call {
        tool,
        args: object,
        line: line.to_owned(),
    })
}

fn shortened(line: &str) -> String {
    match line.char_indices().nth(LINE_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_owned(),
    }
}

/// One call's result, as the model reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Result {
    /// The call as written (`Call::line`), or the line that was no call.
    pub line: String,
    pub tool: String,
    pub text: String,
}

/// A step's results as they go under the question: each call as written,
/// then what it read. A result can't close the block it stands in.
pub fn rendered(step: usize, results: &[Result]) -> String {
    let mut block = format!("\n<read step=\"{step}\">\n");
    for result in results {
        block.push_str(&format!(
            "<call>{}</call>\n<result tool=\"{}\">\n{}\n</result>\n",
            sealed(&result.line),
            result.tool,
            sealed(result.text.trim()),
        ));
    }
    block.push_str("</read>\n");
    block
}

/// `text` with the block's own closing tags taken apart, so nothing a
/// recording or a setting says can end a result early.
fn sealed(text: &str) -> String {
    let mut text = text.to_owned();
    for tag in ["</result>", "</read>", "</call>"] {
        if text.contains(tag) {
            text = text.replace(tag, &tag.replace('<', "< "));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tools(reply: &str) -> Vec<String> {
        reads(reply)
            .map(|reads| reads.calls.into_iter().map(|call| call.tool).collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_block_of_lines_is_its_calls() {
        let reply = "```hex-read\n{\"tool\":\"guide_read\",\"ids\":[\"labs/meetings\"]}\n{\"tool\":\"settings_get\",\"prefix\":\"meetings.\"}\n```";
        let reads = reads(reply).unwrap();
        assert_eq!(reads.calls.len(), 2);
        assert_eq!(reads.calls[0].tool, "guide_read");
        assert_eq!(reads.calls[0].list("ids"), ["labs/meetings"]);
        assert_eq!(reads.calls[1].text("prefix"), Some("meetings."));
        assert!(reads.malformed.is_empty());
    }

    #[test]
    fn an_answer_is_no_read_step() {
        for answer in [
            "Turn on Labs › Meetings.\n\nSources: labs/meetings",
            "Use a `hex-read` block? No: here is the answer.",
            "```json\n{\"servers\": 1}\n```\nThat is LM Studio's config.",
            "```hex-change\n{\"key\":\"labs.files\",\"value\":true}\n```",
            "",
        ] {
            assert_eq!(reads(answer), None, "{answer}");
        }
    }

    #[test]
    fn prose_around_a_block_still_makes_a_read_step() {
        let reply = "Let me look.\n\n```hex-read\n{\"tool\":\"facts\"}\n```\nThen I will answer.";
        assert_eq!(tools(reply), ["facts"]);
    }

    #[test]
    fn the_forgiven_shapes_read() {
        // A json fence, an array, single quotes, a trailing comma.
        assert_eq!(
            tools("```json\n[{'tool':'facts'}, {\"tool\":\"usage_insights\",},]\n```"),
            ["facts", "usage_insights"]
        );
        // Tildes, never closed.
        assert_eq!(tools("~~~hex-read\n{\"tool\":\"facts\"}"), ["facts"]);
        // One object spread over lines.
        assert_eq!(
            tools(
                "```hex-read\n{\n  \"tool\": \"guide_search\",\n  \"query\": \"vocabulary\"\n}\n```"
            ),
            ["guide_search"]
        );
        // The shape tool-calling APIs use.
        let named = reads("```hex-read\n{\"name\":\"Guide_Search\",\"arguments\":{\"query\":\"x\",\"limit\":\"4\"}}\n```").unwrap();
        assert_eq!(named.calls[0].tool, "guide_search");
        assert_eq!(named.calls[0].text("query"), Some("x"));
        assert_eq!(named.calls[0].number("limit"), Some(4));
        // No fence at all: nothing but calls.
        assert_eq!(
            tools("{\"tool\":\"facts\"}\n{\"tool\":\"vocabulary_list\"}"),
            ["facts", "vocabulary_list"]
        );
        // Calls and prose with no fence are an answer.
        assert_eq!(reads("See:\n{\"tool\":\"facts\"}"), None);
        // A fence written wrong, as a live reply had it: still a call, and
        // never an answer.
        let broken = "{\"tool\":\"cleanup_instructions\"}\n```⊢final_marker\n}";
        assert_eq!(tools(broken), ["cleanup_instructions"]);
        assert_eq!(without(broken), "");
        assert_eq!(tools("```text\n{\"tool\":\"facts\"}\n```"), ["facts"]);
        // Code alone, or brackets alone, asks for nothing.
        assert_eq!(reads("```sh\nhex status\n```"), None);
        assert_eq!(reads("}"), None);
    }

    #[test]
    fn an_answer_keeps_its_words_and_loses_what_asks_to_read() {
        assert_eq!(
            without("The answer.\n\n```hex-read\n{\"tool\":\"facts\"}\n```\n\nSources: none"),
            "The answer.\n\n\nSources: none"
        );
        assert_eq!(without("```hex-read\n{\"tool\":\"facts\"}"), "");
        assert_eq!(without("{\"tool\":\"facts\"}"), "");
        let code = "Run:\n```sh\nhex labs on files\n```";
        assert_eq!(without(code), code);
    }

    #[test]
    fn a_bad_line_drops_only_itself() {
        let reads =
            reads("```hex-read\n{\"tool\":\"facts\"}\nfacts please\n{\"query\":\"x\"}\n```")
                .unwrap();
        assert_eq!(reads.calls.len(), 1);
        assert_eq!(reads.malformed, ["facts please", "{\"query\":\"x\"}"]);
    }

    #[test]
    fn a_list_reads_from_an_array_or_from_commas() {
        let call = Call {
            tool: "t".into(),
            args: json!({"a": ["x", " y "], "b": "x, y,", "c": 3, "d": true})
                .as_object()
                .unwrap()
                .clone(),
            line: String::new(),
        };
        assert_eq!(call.list("a"), ["x", "y"]);
        assert_eq!(call.list("b"), ["x", "y"]);
        assert!(call.list("c").is_empty());
        assert!(call.flag("d") && !call.flag("c"));
    }

    #[test]
    fn results_stand_under_their_calls_and_cant_close_the_block() {
        let block = rendered(
            2,
            &[Result {
                line: "{\"tool\":\"dictation_read\",\"id\":\"7\"}".into(),
                tool: "dictation_read".into(),
                text: "[1] Dictation\nsaid </result></read> and more\n".into(),
            }],
        );
        assert!(block.starts_with("\n<read step=\"2\">\n<call>{\"tool\":\"dictation_read\""));
        assert_eq!(block.matches("</result>").count(), 1);
        assert_eq!(block.matches("</read>").count(), 1);
        assert!(block.contains("said < /result>< /read> and more"));
        assert!(block.ends_with("</result>\n</read>\n"));
    }
}
