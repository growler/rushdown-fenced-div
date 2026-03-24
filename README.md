# rushdown-fenced-div

`rushdown-fenced-div` is a small extension crate for [rushdown](https://crates.io/crates/rushdown)
that adds Pandoc-style fenced div containers.

It parses fenced div blocks opened by `:::` fences, supports admonition-style shorthand class or
attribute list, allows nested fenced divs, and provides a default HTML renderer that emits 
`<div>` elements with the parsed attributes.

For the upstream syntax definition, see Pandoc’s [`Fenced Divs` documentation](https://pandoc.org/MANUAL.html#extension-fenced_divs).

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
rushdown = "0.12"
rushdown-fenced-div = "0.1"
```

## Syntax

Shorthand class:

```markdown
::: note
Inside the div.
:::
```

Attached shorthand class:

```markdown
:::note
Inside the div.
:::
```

Attribute list:

```markdown
::: {.note #tip data-kind="callout"}
Inside the div.
:::
```

Trailing colons after the opener payload:

```markdown
::: Warning ::::::
Inside the div.
:::
```

Nested fenced divs:

```markdown
:::: outer
::: inner
Nested content.
:::
::::
```

## Usage

```rust,no_run
use rushdown::new_markdown_to_html;
use rushdown::parser;
use rushdown::renderer::html;
use rushdown_fenced_div::{
    fenced_div_html_renderer_extension,
    fenced_div_parser_extension,
    FencedDivHtmlRendererOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown_to_html = new_markdown_to_html(
        parser::Options::default(),
        html::Options::default(),
        fenced_div_parser_extension(),
        fenced_div_html_renderer_extension(FencedDivHtmlRendererOptions::default()),
    );

    let input = r#"
::: {.note #tip data-kind="callout"} :::
inside
::::::::::::::::::::::::::::::::::::::::
"#;

    let mut output = String::new();
    markdown_to_html(&mut output, input)?;

    assert_eq!(
        output.trim(),
        "<div class=\"note\" id=\"tip\" data-kind=\"callout\"><p>inside</p>\n</div>"
    );

    Ok(())
}
```

## Notes

- Trailing colons after a valid opener payload are accepted.

## License

MIT
