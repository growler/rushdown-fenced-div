#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;

use core::any::TypeId;
use core::fmt;
use core::fmt::Write;
use core::str;

use rushdown::{
    ast::{
        pp_indent, Arena, Attributes, KindData, NodeKind, NodeRef, NodeType, PrettyPrint,
        WalkStatus,
    },
    context::{ContextKey, ContextKeyRegistry, UsizeValue},
    parser::{
        self, parse_attributes, AnyBlockParser, BlockParser, NoParserOptions, Parser,
        ParserExtension, ParserExtensionFn, PRIORITY_LIST,
    },
    renderer::{
        self,
        html::{self, Renderer, RendererExtension, RendererExtensionFn},
        BoxRenderNode, NodeRenderer, NodeRendererRegistry, RenderNode, RendererOptions, TextWrite,
    },
    text::{self, BlockReader, Reader as _, EOS},
    util::{is_punct, is_space},
    Result,
};

// AST {{{

const OPEN_DIV_DEPTH: &str = "rushdown-fenced-div-depth";

/// AST node for a fenced div container.
#[derive(Debug)]
pub struct FencedDiv {
    depth: usize,
}

impl FencedDiv {
    fn new(depth: usize) -> Self {
        Self { depth }
    }
}

impl NodeKind for FencedDiv {
    fn typ(&self) -> NodeType {
        NodeType::ContainerBlock
    }

    fn kind_name(&self) -> &'static str {
        "FencedDiv"
    }
}

impl PrettyPrint for FencedDiv {
    fn pretty_print(&self, w: &mut dyn Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(w, "{}FencedDiv", pp_indent(level))
    }
}

impl From<FencedDiv> for KindData {
    fn from(e: FencedDiv) -> Self {
        KindData::Extension(Box::new(e))
    }
}

// }}} AST

// Parser {{{

#[derive(Debug)]
struct FencedDivBlockParser {
    open_div_depth: ContextKey<UsizeValue>,
}

impl FencedDivBlockParser {
    fn new(reg: alloc::rc::Rc<core::cell::RefCell<ContextKeyRegistry>>) -> Self {
        let open_div_depth = reg.borrow_mut().get_or_create::<UsizeValue>(OPEN_DIV_DEPTH);
        Self { open_div_depth }
    }
}

impl BlockParser for FencedDivBlockParser {
    fn trigger(&self) -> &[u8] {
        b":"
    }

    fn open(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        ctx: &mut parser::Context,
    ) -> Option<(NodeRef, parser::State)> {
        let segment = reader.peek_line_segment()?;
        let blk = [segment];
        let mut blk_reader = BlockReader::new(reader.source(), &blk);
        let fence_length = blk_reader.skip_while(|b| b == b':');
        if fence_length < 3 {
            return None;
        }
        let depth = ctx.get(self.open_div_depth).copied().unwrap_or(0) + 1;
        let node_ref = parse_opening_fence(arena, &mut blk_reader, depth)?;
        ctx.insert(self.open_div_depth, depth);
        reader.advance_to_eol();
        Some((node_ref, parser::State::HAS_CHILDREN))
    }

    fn cont(
        &self,
        arena: &mut Arena,
        node_ref: NodeRef,
        reader: &mut text::BasicReader,
        ctx: &mut parser::Context,
    ) -> Option<parser::State> {
        if let Some(last_opened_block) = ctx.last_opened_block() {
            // CodeBlock is a special case
            if last_opened_block != node_ref
                && matches!(arena[last_opened_block].kind_data(), KindData::CodeBlock(_))
            {
                return Some(parser::State::HAS_CHILDREN);
            }
        }
        let (line, _) = reader.peek_line_bytes()?;
        let fence_length = line.iter().take_while(|&&b| b == b':').count();
        if fence_length < 3 {
            return Some(parser::State::HAS_CHILDREN);
        }
        let rest = &line[fence_length..];
        if rest
            .iter()
            .take_while(|&&b| b.is_ascii_whitespace())
            .count()
            < rest.len()
        {
            return Some(parser::State::HAS_CHILDREN);
        }
        let fenced_div = rushdown::as_extension_data!(arena, node_ref, FencedDiv);
        let open_depth = ctx.get(self.open_div_depth).copied().unwrap_or(0);
        // apparently, rushdown calls blocks from outermost to innermost, so
        // we have to check the open_depth
        if fenced_div.depth == open_depth {
            reader.advance_to_eol();
            return None;
        }
        Some(parser::State::HAS_CHILDREN)
    }

    fn close(
        &self,
        _arena: &mut Arena,
        _node_ref: NodeRef,
        _reader: &mut text::BasicReader,
        ctx: &mut parser::Context,
    ) {
        if let Some(depth) = ctx.get_mut(self.open_div_depth) {
            *depth = depth.saturating_sub(1);
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

fn parse_opening_fence(
    arena: &mut Arena,
    reader: &mut text::BlockReader,
    depth: usize,
) -> Option<NodeRef> {
    reader.skip_spaces();
    let b = reader.peek_byte();
    if b == EOS {
        return None;
    }
    let attributes = if b == b'{' {
        parse_attributes(reader)?
    } else {
        let (line, seg) = reader.peek_line_bytes()?;
        let i = line
            .iter()
            .take_while(|&&b| {
                !is_space(b) && (!is_punct(b) || b == b'_' || b == b'-' || b == b':' || b == b'.')
            })
            .count();
        if i == 0 {
            return None;
        }
        let mut attributes = Attributes::new();
        attributes.insert("class", seg.with_stop(seg.start() + i).into());
        reader.advance(i);
        attributes
    };
    reader.skip_spaces();
    reader.skip_while(|b| b == b':');
    reader.skip_spaces();
    if reader.peek_byte() != EOS {
        return None;
    }
    let node_ref = arena.new_node(FencedDiv::new(depth));
    arena[node_ref].attributes_mut().extend(attributes);
    Some(node_ref)
}

impl From<FencedDivBlockParser> for AnyBlockParser {
    fn from(p: FencedDivBlockParser) -> Self {
        AnyBlockParser::Extension(Box::new(p))
    }
}

/// Returns a parser extension that parses fenced div blocks.
pub fn fenced_div_parser_extension() -> impl ParserExtension {
    ParserExtensionFn::new(|p: &mut Parser| {
        p.add_block_parser(
            FencedDivBlockParser::new,
            NoParserOptions,
            PRIORITY_LIST + 100,
        );
    })
}

// }}} Parser

// Renderer {{{

#[derive(Debug, Clone, Default)]
pub struct FencedDivHtmlRendererOptions;

impl RendererOptions for FencedDivHtmlRendererOptions {}

struct FencedDivHtmlRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
}

impl<W: TextWrite> FencedDivHtmlRenderer<W> {
    fn with_options(html_opts: html::Options, _options: FencedDivHtmlRendererOptions) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
        }
    }
}

impl<W: TextWrite> RenderNode<W> for FencedDivHtmlRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _ctx: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            self.writer.write_safe_str(w, "<div")?;
            html::render_attributes(w, source, arena[node_ref].attributes(), None)?;
            self.writer.write_safe_str(w, ">")?;
        } else {
            self.writer.write_safe_str(w, "</div>")?;
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for FencedDivHtmlRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<FencedDiv>(), BoxRenderNode::new(self));
    }
}

/// Returns a renderer extension that renders fenced div blocks in HTML.
pub fn fenced_div_html_renderer_extension<'cb, W>(
    options: impl Into<FencedDivHtmlRendererOptions>,
) -> impl RendererExtension<'cb, W>
where
    W: TextWrite + 'cb,
{
    RendererExtensionFn::new(move |r: &mut Renderer<'cb, W>| {
        r.add_node_renderer(FencedDivHtmlRenderer::with_options, options.into());
    })
}

// }}} Renderer
