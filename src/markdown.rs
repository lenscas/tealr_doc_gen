use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag};

use crate::compile_teal::compile_teal;

pub(crate) fn parse_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown, options);
    let mut teal_block = None;
    let injected = parser.flat_map(move |v| {
        //println!("{:?}",v);
        match (v, teal_block.as_ref()) {
            (Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(x))), None) => {
                if x.eq_ignore_ascii_case("teal_lua") {
                    teal_block = Some(String::new());
                    return Vec::new();
                };
                vec![pulldown_cmark::Event::Start(Tag::CodeBlock(
                    CodeBlockKind::Fenced(x),
                ))]
            }
            (Event::Text(x), Some(old)) => {
                teal_block = Some(old.to_string() + "\n" + &x);
                Vec::new()
            }
            (Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(x))), Some(y)) => {
                if x.eq_ignore_ascii_case("teal_lua") {
                    let retrn = vec![
                        Event::Html(
                            CowStr::from(
                                "<div class=\"tabs\"><ul><li class=\"select-teal\"><a>Teal</a></li><li class=\"select-lua\"><a>Lua</a></li></ul></div>"
                            )
                        ),
                        Event::Html(
                            CowStr::from("<div class=\"code-block-teal\">")
                        ),
                        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from("lua")))),
                        Event::Text(CowStr::from(y.clone())),
                        Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from("lua")))),
                        Event::Html(CowStr::from("</div>")),
                        Event::Html(
                            CowStr::from("<div class=\"code-block-lua\">")
                        ),
                        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from("lua")))),
                        Event::Text(CowStr::from(compile_teal(y.clone()))),
                        Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from("lua")))),
                        Event::Html(CowStr::from("</div>")),
                    ];
                    teal_block = None;
                    retrn
                } else {
                    vec![Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(x)))]
                }
            }

            (e, _) => vec![e],
        }
    });
    let mut html_output = String::new();
    html::push_html(&mut html_output, injected);
    html_output
}
