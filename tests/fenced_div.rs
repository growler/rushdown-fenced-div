use rushdown::new_markdown_to_html;
use rushdown::parser::{self, Parser};
use rushdown::renderer::html;
use rushdown::text::BasicReader;
use rushdown_fenced_div::{
    fenced_div_html_renderer_extension, fenced_div_parser_extension, FencedDivHtmlRendererOptions,
};

fn render(input: &str) -> String {
    let mut output = String::new();
    let markdown_to_html = new_markdown_to_html(
        parser::Options::default(),
        html::Options::default(),
        fenced_div_parser_extension(),
        fenced_div_html_renderer_extension(FencedDivHtmlRendererOptions),
    );
    markdown_to_html(&mut output, input).unwrap();
    output
}

fn parse_first_fenced_div(input: &str) -> (rushdown::ast::Arena, rushdown::ast::NodeRef) {
    let parser = Parser::with_extensions(parser::Options::default(), fenced_div_parser_extension());
    let mut reader = BasicReader::new(input);
    let (arena, document_ref) = parser.parse(&mut reader);
    let node_ref = arena[document_ref]
        .first_child()
        .expect("expected a fenced div node");
    (arena, node_ref)
}

fn parse_document(input: &str) -> (rushdown::ast::Arena, rushdown::ast::NodeRef) {
    let parser = Parser::with_extensions(parser::Options::default(), fenced_div_parser_extension());
    let mut reader = BasicReader::new(input);
    parser.parse(&mut reader)
}

fn child_kind_names(
    arena: &rushdown::ast::Arena,
    node_ref: rushdown::ast::NodeRef,
) -> Vec<&'static str> {
    arena[node_ref]
        .children(arena)
        .map(|child_ref| arena[child_ref].kind_data().kind_name())
        .collect()
}

#[test]
fn renders_shorthand_class_div() {
    let input = "\
::: note
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>inside</p>\n</div>"
    );
}

#[test]
fn renders_attached_shorthand_class_div() {
    let input = "\
:::note
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>inside</p>\n</div>"
    );
}

#[test]
fn renders_attribute_list_div() {
    let input = "\
::: {.note #tip data-kind=\"callout\" data-level=2}
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\" id=\"tip\" data-kind=\"callout\" data-level=\"2\"><p>inside</p>\n</div>"
    );
}

#[test]
fn renders_shorthand_div_with_trailing_colons() {
    let input = "\
::: note ::::::
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>inside</p>\n</div>"
    );
}

#[test]
fn renders_attribute_list_div_with_trailing_colons() {
    let input = "\
::: {.note #tip data-kind=\"callout\"} :::
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\" id=\"tip\" data-kind=\"callout\"><p>inside</p>\n</div>"
    );
}

#[test]
fn stores_attributes_on_the_rushdown_node() {
    let input = "\
::: {.note .warning #tip data-kind=\"callout\" title=\"Fish &amp; Chips\"}
inside
:::
";
    let (arena, node_ref) = parse_first_fenced_div(input);
    let attrs = arena[node_ref].attributes();

    assert_eq!(attrs.get("id").unwrap().str(input), "tip");
    assert_eq!(attrs.get("class").unwrap().str(input), "note warning");
    assert_eq!(attrs.get("data-kind").unwrap().str(input), "callout");
    assert_eq!(attrs.get("title").unwrap().str(input), "Fish & Chips");
}

#[test]
fn attached_trailing_colons_are_part_of_shorthand_class() {
    let input = "\
:::note:::::
inside
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note:::::\"><p>inside</p>\n</div>"
    );
}

#[test]
fn renders_nested_divs() {
    let input = "\
::: Warning ::::::
This is a warning.

::: Danger
This is a warning within a warning.
:::
::::::::::::::::::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"Warning\"><p>This is a warning.</p>\n<div class=\"Danger\"><p>This is a warning within a warning.</p>\n</div></div>"
    );
}

#[test]
fn same_length_nested_closers_close_innermost_div_first() {
    let input = "\
::: outer
before

::: inner
inside
:::

after
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"outer\"><p>before</p>\n<div class=\"inner\"><p>inside</p>\n</div><p>after</p>\n</div>"
    );
}

#[test]
fn renders_markdown_inside_div() {
    let input = "\
::: note
*emphasis*
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p><em>emphasis</em></p>\n</div>"
    );
}

#[test]
fn invalid_opener_falls_back_to_plain_markdown() {
    let input = "\
::: {not valid
text
";

    assert_eq!(render(input).trim(), "<p>::: {not valid\ntext</p>");
}

#[test]
fn indented_opener_falls_back_to_plain_markdown() {
    let input = " ::: note\ninside\n:::\n";

    assert_eq!(render(input).trim(), "<p>::: note\ninside\n:::</p>");
}

#[test]
fn mixed_shorthand_and_attribute_list_falls_back_to_plain_markdown() {
    let input = "\
::: note {.tip #abc data-kind=\"callout\"}
text
:::
";

    assert_eq!(
        render(input).trim(),
        "<p>::: note {.tip #abc data-kind=&quot;callout&quot;}\ntext\n:::</p>"
    );
}

#[test]
fn attached_shorthand_and_attribute_list_falls_back_to_plain_markdown() {
    let input = "\
:::node {#abc}
text
:::
";

    assert_eq!(render(input).trim(), "<p>:::node {#abc}\ntext\n:::</p>");
}

#[test]
fn multiple_shorthand_classes_fall_back_to_plain_markdown() {
    let input = "\
::: note warning
text
";

    assert_eq!(render(input).trim(), "<p>::: note warning\ntext</p>");
}

#[test]
fn shorter_closing_fence_closes_outer_div() {
    let input = "\
:::: note
alpha
:::
beta
::::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>alpha</p>\n</div><p>beta\n::::</p>"
    );
}

#[test]
fn indented_closing_fence_is_treated_as_content() {
    let input = "::: note\nalpha\n :::\nbeta\n";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>alpha\n:::\nbeta</p>\n</div>"
    );
}

#[test]
fn unclosed_div_consumes_to_eof() {
    let input = "\
::: note
alpha
beta
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>alpha\nbeta</p>\n</div>"
    );
}

#[test]
fn closing_fence_with_trailing_text_is_not_a_closer() {
    let input = "\
::: note
alpha
::: nope
:::
";

    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><p>alpha</p>\n<div class=\"nope\"></div></div>"
    );
}

#[test]
fn fenced_code_block_contents_do_not_close_the_div() {
    let input = "\
::: note
```rust
:::
let x = 1;
```

after
:::
";

    let (arena, node_ref) = parse_first_fenced_div(input);

    assert_eq!(
        child_kind_names(&arena, node_ref),
        vec!["CodeBlock", "Paragraph"]
    );
    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><pre><code class=\"language-rust\">:::\nlet x = 1;\n</code></pre>\n<p>after</p>\n</div>"
    );
}

#[test]
fn indented_code_block_contents_do_not_close_the_div() {
    let input = "\
::: note
    :::
    still code

after
:::
";

    let (arena, node_ref) = parse_first_fenced_div(input);

    assert_eq!(
        child_kind_names(&arena, node_ref),
        vec!["CodeBlock", "Paragraph"]
    );
    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><pre><code>:::\nstill code\n</code></pre>\n<p>after</p>\n</div>"
    );
}

#[test]
fn heading_inside_fenced_div_is_parsed_as_a_heading_block() {
    let input = "\
::: note
# Title

paragraph
:::
";

    let (arena, node_ref) = parse_first_fenced_div(input);

    assert_eq!(
        child_kind_names(&arena, node_ref),
        vec!["Heading", "Paragraph"]
    );
    assert_eq!(
        render(input).trim(),
        "<div class=\"note\"><h1>Title</h1>\n<p>paragraph</p>\n</div>"
    );
}

#[test]
fn parser_resumes_after_fenced_code_block_for_following_blocks() {
    let input = "\
::: outer
```text
::: inner
:::
```

::: actual
content
:::
:::
";

    let (arena, node_ref) = parse_first_fenced_div(input);
    let nested_div_ref = arena[node_ref]
        .children(&arena)
        .nth(1)
        .expect("expected nested fenced div after code block");

    assert_eq!(
        child_kind_names(&arena, node_ref),
        vec!["CodeBlock", "FencedDiv"]
    );
    assert_eq!(
        arena[nested_div_ref]
            .attributes()
            .get("class")
            .unwrap()
            .str(input),
        "actual"
    );
    assert_eq!(child_kind_names(&arena, nested_div_ref), vec!["Paragraph"]);
}

#[test]
fn top_level_fenced_code_block_containing_div_syntax_does_not_create_a_div() {
    let input = "\
```markdown
::: note
inside
:::
```
";

    let (arena, document_ref) = parse_document(input);

    assert_eq!(child_kind_names(&arena, document_ref), vec!["CodeBlock"]);
    assert_eq!(
        render(input).trim(),
        "<pre><code class=\"language-markdown\">::: note\ninside\n:::\n</code></pre>"
    );
}
