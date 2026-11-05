//! A version's Markdown as blocks: headings, list items, quotes, code,
//! rules, table rows, and paragraphs, their inline marks dropped. Enough for
//! what cleanup and requests write, without a Markdown parser.

use rune::{El, col, row, text};

use super::super::controls::Paint;
use crate::files::text::{inline, is_rule, table_row};

enum Block {
    Heading(usize, String),
    Item {
        indent: usize,
        mark: String,
        text: String,
    },
    Quote(String),
    Code(String),
    Rule,
    Paragraph(String),
}

fn blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut code: Option<Vec<String>> = None;
    let flush = |paragraph: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            match code.take() {
                Some(lines) => blocks.push(Block::Code(lines.join("\n"))),
                None => {
                    flush(&mut paragraph, &mut blocks);
                    code = Some(Vec::new());
                }
            }
            continue;
        }
        if let Some(lines) = &mut code {
            lines.push(line.to_string());
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        let indent = (line.len() - trimmed.len()) / 2;
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading(hashes, inline(trimmed[hashes..].trim())));
        } else if is_rule(trimmed) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
        } else if let Some(item) = ["- [ ] ", "- [x] ", "- ", "* ", "+ "]
            .iter()
            .find_map(|mark| trimmed.strip_prefix(mark).map(|rest| (*mark, rest)))
        {
            flush(&mut paragraph, &mut blocks);
            let mark = match item.0 {
                "- [ ] " => "☐",
                "- [x] " => "☑",
                _ => "•",
            };
            blocks.push(Block::Item {
                indent,
                mark: mark.into(),
                text: inline(item.1),
            });
        } else if let Some((number, rest)) = trimmed.split_once(". ")
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item {
                indent,
                mark: format!("{number}."),
                text: inline(rest),
            });
        } else if let Some(quote) = trimmed.strip_prefix('>') {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Quote(inline(quote.trim())));
        } else if trimmed.starts_with('|') {
            flush(&mut paragraph, &mut blocks);
            let cells = table_row(trimmed);
            if !cells.is_empty() {
                blocks.push(Block::Paragraph(cells));
            }
        } else {
            paragraph.push(inline(trimmed));
        }
    }
    if let Some(lines) = code {
        blocks.push(Block::Code(lines.join("\n")));
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

pub(super) fn view(p: Paint, markdown: &str) -> El {
    let mut column = col().key("markdown").gap(8.0);
    for (index, block) in blocks(markdown).into_iter().enumerate() {
        let key = format!("block{index}");
        let words = |copy: String| {
            text(copy)
                .key("text")
                .font(p.k.ty.body)
                .color(p.text())
                .grow(1.0)
                .shrink()
                .wrap()
        };
        column = column.child(match block {
            Block::Heading(level, copy) => text(copy)
                .key(key)
                .font(if level <= 2 {
                    p.k.ty.body_medium
                } else {
                    p.k.ty.ui_medium
                })
                .color(p.text())
                .pt(if index == 0 { 0.0 } else { 8.0 })
                .wrap(),
            Block::Item {
                indent,
                mark,
                text: copy,
            } => row()
                .key(key)
                .gap(8.0)
                .pl(indent as f32 * 16.0)
                .child(
                    text(mark)
                        .key("mark")
                        .font(p.k.ty.body)
                        .color(p.muted())
                        .w(18.0),
                )
                .child(words(copy)),
            Block::Quote(copy) => row()
                .key(key)
                .pl(10.0)
                .border(p.hair, p.k.c.border_base)
                .child(
                    text(copy)
                        .key("text")
                        .font(p.k.ty.body)
                        .color(p.muted())
                        .wrap(),
                ),
            Block::Code(copy) => text(copy)
                .key(key)
                .font(p.k.ty.mono)
                .color(p.text())
                .px(10.0)
                .py(8.0)
                .radius(p.k.radius.sm)
                .fill(p.k.c.layer_02)
                .wrap(),
            Block::Rule => rune::el()
                .key(key)
                .h(p.hair)
                .w_full()
                .fill(p.k.c.border_weak),
            Block::Paragraph(copy) => row().key(key).child(words(copy)),
        });
    }
    column
}
