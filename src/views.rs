use maud::{html, Markup, PreEscaped};
use crate::models::MarkdownDocument;
use crate::utils::{convert_markdown_to_html, extract_title_from_html, generate_qr_svg};

pub fn create_html_head(page_title: Option<&str>) -> Markup {
    html! {
        head {
            title { (page_title.unwrap_or("mdow")) };

            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";

            meta name="title" content="mdow 🌾 | markdown on web";
            meta name="description" content="A meadow for your markdown on web. A lightweight, browser-based markdown editor and previewer that makes sharing markdown files as simple as sharing a link.";
            meta name="keywords" content="markdown editor, online markdown, markdown preview, markdown sharing, web markdown, browser markdown";

            meta name="application-name" content="mdow";
            meta name="mobile-web-app-capable" content="yes";
            meta name="apple-mobile-web-app-capable" content="yes";
            meta name="apple-mobile-web-app-title" content="mdow";
            meta name="apple-mobile-web-app-status-bar-style" content="default";
            meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)";
            meta name="theme-color" content="#000000" media="(prefers-color-scheme: dark)";

            link rel="apple-touch-icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";

            link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";

            script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {};
            script src="https://unpkg.com/htmx.org@1.9.10" {};
            script src="https://unpkg.com/hyperscript.org@0.9.12" {};

            script data-goatcounter="https://yree.goatcounter.com/count" async src="//gc.zgo.at/count.js" {};
        }
    }
}

pub fn create_page_footer() -> Markup {
    html! {
        footer {
            div class="w" {
                p style="display: flex; justify-content: space-between; margin: 0;" {
                    span {
                        a href="https://yree.io/mdow" { "mdow" }
                        " 🌾 — a "
                        a href="https://yree.io" { "Yree" }
                        " product ♥ "
                    }
                    kbd { "?" }
                }
            }
        }
    }
}

pub async fn create_markdown_editor_page(initial_content: &str) -> Markup {
    html! {
        (create_html_head(None));
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "mdow 🌾" }
                    p { dfn {"A meadow for your " b {"markdown on web."} } }
                    p { "Enter your markdown, preview it, and share it." }
                    div class="grid" {
                        button
                            id="preview-button"
                            hx-post="/preview"
                            hx-trigger="click"
                            hx-target="#markdown-input"
                            hx-swap="outerHTML"
                            hx-include="#markdown-input"
                            hx-validate="true"
                            hx-disabled-elt="this"
                            _="on htmx:afterRequest
                               hide me
                               show #edit-button"
                               { "Preview" }
                        button
                            id="edit-button"
                            hx-post="/edit"
                            hx-trigger="click"
                            hx-target="#markdown-preview"
                            hx-swap="outerHTML"
                            hx-include="#markdown-preview"
                            style="display: none;"
                            hx-disabled-elt="this"
                            _="on htmx:afterRequest
                               hide me
                               show #preview-button"
                               { "Edit" }
                        button
                            id="share-button"
                            hx-post="/share"
                            hx-trigger="click"
                            hx-include="[name='content']"
                            hx-validate="true"
                            hx-disabled-elt="this"
                            { "Share" }
                    }
                    textarea
                        id="markdown-input"
                        name="content"
                        placeholder=(if initial_content.is_empty() { "Enter your markdown..." } else { "" })
                        style="width: 100%; height: 30ch; resize: vertical"
                        required="required"
                        _=(if initial_content.is_empty() {
                            "on load
                                set my.value to (localStorage.getItem('markdownContent'))
                             on input
                                wait 500ms then
                                call localStorage.setItem('markdownContent', my.value)"
                        } else {
                            "on input
                                wait 500ms then
                                call localStorage.setItem('markdownContent', my.value)"
                        })
                        { (initial_content) }
                }
            }
        }
        (create_page_footer());
    }
}

pub fn create_markdown_viewer_page(doc: &MarkdownDocument) -> Markup {
    let html_output = convert_markdown_to_html(&doc.content);
    let page_title = extract_title_from_html(&html_output);

    html! {
        (create_html_head(page_title));
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" id="markdown-view" _="on load call MathJax.typeset()" {
                    (PreEscaped(html_output))
                }
            }
            footer {
                div class="w grid" {
                    (PreEscaped(generate_qr_svg(&doc.id)))
                    div {
                        p {
                            "created on " (doc.created_at.format("%Y-%m-%d"))
                        }
                        p {
                            a href=(format!("/?content={}", urlencoding::encode(&doc.content))) { "edit" }
                            " in "
                            a href="/" { "mdow" }
                            " 🌾"
                        }
                    }
                }
            }
        }
    }
}

pub fn create_404_page() -> Markup {
    html! {
        (create_html_head(Some("404")));
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "404 - Page Not Found" }
                    p { "The page you're looking for doesn't exist." }
                    p { a href="/" { "Return to homepage" } }
                }
            }
        }
        (create_page_footer());
    }
}
