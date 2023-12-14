use pulldown_cmark::{
    html, Alignment, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Options, Parser, Tag,
};
use tealr::{
    mlu::{mlua::FromLua, FromToLua},
    ToTypename,
};

#[derive(Clone, Debug, ToTypename, FromToLua)]
///What kind of codeblock it is
pub enum MarkdownCodeBlockKind {
    Indented,
    /// The value contained in the tag describes the language of the code, which may be empty.
    Fenced(String),
}

impl<'a> From<CodeBlockKind<'a>> for MarkdownCodeBlockKind {
    fn from(x: CodeBlockKind) -> Self {
        match x {
            CodeBlockKind::Indented => Self::Indented,
            CodeBlockKind::Fenced(x) => Self::Fenced(x.to_string()),
        }
    }
}

impl From<MarkdownCodeBlockKind> for CodeBlockKind<'static> {
    fn from(x: MarkdownCodeBlockKind) -> Self {
        match x {
            MarkdownCodeBlockKind::Indented => Self::Indented,
            MarkdownCodeBlockKind::Fenced(x) => Self::Fenced(x.into()),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ToTypename, FromToLua)]
pub enum MarkdownHeadingLevel {
    H1 = 1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl From<HeadingLevel> for MarkdownHeadingLevel {
    fn from(x: HeadingLevel) -> Self {
        match x {
            HeadingLevel::H1 => Self::H1,
            HeadingLevel::H2 => Self::H2,
            HeadingLevel::H3 => Self::H3,
            HeadingLevel::H4 => Self::H4,
            HeadingLevel::H5 => Self::H5,
            HeadingLevel::H6 => Self::H6,
        }
    }
}
impl From<MarkdownHeadingLevel> for HeadingLevel {
    fn from(x: MarkdownHeadingLevel) -> Self {
        match x {
            MarkdownHeadingLevel::H1 => Self::H1,
            MarkdownHeadingLevel::H2 => Self::H2,
            MarkdownHeadingLevel::H3 => Self::H3,
            MarkdownHeadingLevel::H4 => Self::H4,
            MarkdownHeadingLevel::H5 => Self::H5,
            MarkdownHeadingLevel::H6 => Self::H6,
        }
    }
}
/// Text alignment in tables
#[derive(ToTypename, Debug, FromToLua, Clone)]
pub enum MarkdownAlignment {
    /// Default text alignment.
    None,
    Left,
    Center,
    Right,
}
impl From<Alignment> for MarkdownAlignment {
    fn from(x: Alignment) -> Self {
        match x {
            Alignment::None => Self::None,
            Alignment::Left => Self::Left,
            Alignment::Center => Self::Center,
            Alignment::Right => Self::Right,
        }
    }
}

impl From<MarkdownAlignment> for Alignment {
    fn from(x: MarkdownAlignment) -> Self {
        match x {
            MarkdownAlignment::None => Self::None,
            MarkdownAlignment::Left => Self::Left,
            MarkdownAlignment::Center => Self::Center,
            MarkdownAlignment::Right => Self::Right,
        }
    }
}

#[derive(ToTypename, Debug, FromToLua, Clone)]
/// Type specifier for inline links
pub enum MarkdownLinkType {
    /// Inline link like `[foo](bar)`
    Inline,
    /// Reference link like `[foo][bar]`
    Reference,
    /// Reference without destination in the document, but resolved by the broken_link_callback
    ReferenceUnknown,
    /// Collapsed link like `[foo][]`
    Collapsed,
    /// Collapsed link without destination in the document, but resolved by the broken_link_callback
    CollapsedUnknown,
    /// Shortcut link like `[foo]`
    Shortcut,
    /// Shortcut without destination in the document, but resolved by the broken_link_callback
    ShortcutUnknown,
    /// Autolink like `<http://foo.bar/baz>`
    Autolink,
    /// Email address in autolink like `<john@example.org>`
    Email,
}

impl From<LinkType> for MarkdownLinkType {
    fn from(x: LinkType) -> Self {
        match x {
            LinkType::Inline => Self::Inline,
            LinkType::Reference => Self::Reference,
            LinkType::ReferenceUnknown => Self::ReferenceUnknown,
            LinkType::Collapsed => Self::Collapsed,
            LinkType::CollapsedUnknown => Self::CollapsedUnknown,
            LinkType::Shortcut => Self::Shortcut,
            LinkType::ShortcutUnknown => Self::ShortcutUnknown,
            LinkType::Autolink => Self::Autolink,
            LinkType::Email => Self::Email,
        }
    }
}

impl From<MarkdownLinkType> for LinkType {
    fn from(x: MarkdownLinkType) -> Self {
        match x {
            MarkdownLinkType::Inline => Self::Inline,
            MarkdownLinkType::Reference => Self::Reference,
            MarkdownLinkType::ReferenceUnknown => Self::ReferenceUnknown,
            MarkdownLinkType::Collapsed => Self::Collapsed,
            MarkdownLinkType::CollapsedUnknown => Self::CollapsedUnknown,
            MarkdownLinkType::Shortcut => Self::Shortcut,
            MarkdownLinkType::ShortcutUnknown => Self::ShortcutUnknown,
            MarkdownLinkType::Autolink => Self::Autolink,
            MarkdownLinkType::Email => Self::Email,
        }
    }
}

#[derive(Clone, Debug, FromToLua, ToTypename)]
/// Tags containing other elements
pub enum MarkdownTag {
    /// A paragraph of text and other inline elements.
    Paragraph,

    /// A heading. The first field indicates the level of the heading,
    /// the second the fragment identifier, and the third the classes.
    Heading(MarkdownHeadingLevel, Option<String>, Vec<String>),

    BlockQuote,
    /// A code block.
    CodeBlock(MarkdownCodeBlockKind),

    /// A list. If the list is ordered the field indicates the number of the first item.
    /// Contains only list items.
    List(Option<u64>), // TODO: add delim and tight for ast (not needed for html)
    /// A list item.
    Item,
    /// A footnote definition. The value contained is the footnote's label by which it can
    /// be referred to.
    FootnoteDefinition(String),

    /// A table. Contains a vector describing the text-alignment for each of its columns.
    Table(Vec<MarkdownAlignment>),
    /// A table header. Contains only `TableRow`s. Note that the table body starts immediately
    /// after the closure of the `TableHead` tag. There is no `TableBody` tag.
    TableHead,
    /// A table row. Is used both for header rows as body rows. Contains only `TableCell`s.
    TableRow,
    TableCell,

    // span-level tags
    Emphasis,
    Strong,
    Strikethrough,

    /// A link. The first field is the link type, the second the destination URL and the third is a title.
    Link(MarkdownLinkType, String, String),

    /// An image. The first field is the link type, the second the destination URL and the third is a title.
    Image(MarkdownLinkType, String, String),
}

impl<'a> From<Tag<'a>> for MarkdownTag {
    fn from(x: Tag<'a>) -> Self {
        match x {
            Tag::Paragraph => Self::Paragraph,
            Tag::Heading(x, y, z) => Self::Heading(
                x.into(),
                y.map(Into::into),
                z.into_iter().map(Into::into).collect(),
            ),
            Tag::BlockQuote => Self::BlockQuote,
            Tag::CodeBlock(x) => Self::CodeBlock(x.into()),
            Tag::List(x) => Self::List(x),
            Tag::Item => Self::Item,
            Tag::FootnoteDefinition(x) => Self::FootnoteDefinition(x.to_string()),
            Tag::Table(x) => Self::Table(x.into_iter().map(Into::into).collect()),
            Tag::TableHead => Self::TableHead,
            Tag::TableRow => Self::TableRow,
            Tag::TableCell => Self::TableCell,
            Tag::Emphasis => Self::Emphasis,
            Tag::Strong => Self::Strong,
            Tag::Strikethrough => Self::Strikethrough,
            Tag::Link(x, y, z) => Self::Link(x.into(), y.to_string(), z.to_string()),
            Tag::Image(x, y, z) => Self::Image(x.into(), y.to_string(), z.to_string()),
        }
    }
}

impl From<MarkdownTag> for Tag<'static> {
    fn from(x: MarkdownTag) -> Self {
        match x {
            MarkdownTag::Paragraph => Self::Paragraph,
            MarkdownTag::Heading(x, _, _) => Self::Heading(x.into(), None, Vec::new()),
            MarkdownTag::BlockQuote => Self::BlockQuote,
            MarkdownTag::CodeBlock(x) => Self::CodeBlock(x.into()),
            MarkdownTag::List(x) => Self::List(x),
            MarkdownTag::Item => Self::Item,
            MarkdownTag::FootnoteDefinition(x) => Self::FootnoteDefinition(x.into()),
            MarkdownTag::Table(x) => Self::Table(x.into_iter().map(Into::into).collect()),
            MarkdownTag::TableHead => Self::TableHead,
            MarkdownTag::TableRow => Self::TableRow,
            MarkdownTag::TableCell => Self::TableCell,
            MarkdownTag::Emphasis => Self::Emphasis,
            MarkdownTag::Strong => Self::Strong,
            MarkdownTag::Strikethrough => Self::Strikethrough,
            MarkdownTag::Link(x, y, z) => Self::Link(x.into(), y.into(), z.into()),
            MarkdownTag::Image(x, y, z) => Self::Image(x.into(), y.into(), z.into()),
        }
    }
}

#[derive(Clone, FromToLua, Debug, ToTypename)]
/// Markdown events that are generated in a preorder traversal of the document
/// tree, with additional `End` events whenever all of an inner node's children
/// have been visited.
pub enum MarkdownEvent {
    Start(MarkdownTag),
    End(MarkdownTag),
    Text(String),
    Code(String),
    Html(String),
    FootnoteReference(String),
    SoftBreak,
    HardBreak,
    Rule,
    TaskListMarker(bool),
}

impl<'lua> tealr::mlu::FromLuaExact<'lua> for MarkdownEvent {
    fn from_lua_exact(
        value: tealr::mlu::mlua::Value<'lua>,
        lua: &'lua tealr::mlu::mlua::Lua,
    ) -> tealr::mlu::mlua::Result<Self> {
        MarkdownEvent::from_lua(value, lua)
    }
}

impl<'a> From<Event<'a>> for MarkdownEvent {
    fn from(x: Event) -> Self {
        match x {
            Event::Start(x) => Self::Start(x.into()),
            Event::End(x) => Self::End(x.into()),
            Event::Text(x) => Self::Text(x.to_string()),
            Event::Code(x) => Self::Code(x.to_string()),
            Event::Html(x) => Self::Html(x.to_string()),
            Event::FootnoteReference(x) => Self::FootnoteReference(x.to_string()),
            Event::SoftBreak => Self::SoftBreak,
            Event::HardBreak => Self::HardBreak,
            Event::Rule => Self::Rule,
            Event::TaskListMarker(x) => Self::TaskListMarker(x),
        }
    }
}
impl From<MarkdownEvent> for Event<'static> {
    fn from(x: MarkdownEvent) -> Self {
        match x {
            MarkdownEvent::Start(x) => Self::Start(x.into()),
            MarkdownEvent::End(x) => Self::End(x.into()),
            MarkdownEvent::Text(x) => Self::Text(CowStr::from(x)),
            MarkdownEvent::Code(x) => Self::Code(CowStr::from(x)),
            MarkdownEvent::Html(x) => Self::Html(CowStr::from(x)),
            MarkdownEvent::FootnoteReference(x) => Self::FootnoteReference(x.into()),
            MarkdownEvent::SoftBreak => Self::SoftBreak,
            MarkdownEvent::HardBreak => Self::HardBreak,
            MarkdownEvent::Rule => Self::Rule,
            MarkdownEvent::TaskListMarker(x) => Self::TaskListMarker(x),
        }
    }
}

pub(crate) fn parse_markdown_lua(
    to_parse: String,
    func: impl Fn(MarkdownEvent) -> Result<Vec<MarkdownEvent>, tealr::mlu::mlua::Error>,
) -> Result<String, tealr::mlu::mlua::Error> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(&to_parse, options);
    let injected = parser
        .flat_map(|x| {
            let z = func(x.into());
            match z {
                Ok(x) => x.into_iter().map(Ok).collect::<Vec<_>>(),
                Err(x) => vec![Err(x)],
            }
        })
        .collect::<Result<Vec<_>, tealr::mlu::mlua::Error>>()?;
    let transformed = injected.iter().map(|v| match v {
        MarkdownEvent::Start(MarkdownTag::Heading(x, y, z)) => Event::Start(Tag::Heading(
            (*x).into(),
            y.as_deref(),
            z.iter().map(|x| x.as_str()).collect(),
        )),
        MarkdownEvent::End(MarkdownTag::Heading(x, y, z)) => Event::End(Tag::Heading(
            (*x).into(),
            y.as_deref(),
            z.iter().map(|x| x.as_str()).collect(),
        )),
        x => x.to_owned().into(),
    });
    let mut html_output = Default::default();
    html::push_html(&mut html_output, transformed);
    Ok(html_output)
}
