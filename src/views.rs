use maud::{html, Markup, PreEscaped};
use chrono::Duration;
use crate::models::Document;
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
        footer style="display: flex; justify-content: space-between;" {
            span {
                a href="https://yree.io/mdow" { "mdow" }
                " 🌾 — a "
                a href="https://yree.io" { "Yree" }
                " product ♥ "
            }
            kbd _="on click call #help-dialog.showModal()" { "?" }
        }
    }
}

pub fn create_help_dialog() -> Markup {
    html! {
        dialog
            id="help-dialog"
            _="on load
                 if localStorage.getItem('mdow-visited') is null
                   call me.showModal()
                   call localStorage.setItem('mdow-visited', 'true')
               on click
                 if event.target is me then call me.close()" {
            h2 { "mdow 🌾" }
            p { "A meadow for your " b { "markdown on web." } }
            hr;
            p { "Write markdown, preview it, and share it as a link. Links stay active for " b { "30 days." } }
            p { b { "Supports:" } " tables, task lists, strikethrough, LaTeX math, syntax highlighting." }
            button _="on click call #help-dialog.close()" { "Got it" }
        }
    }
}

pub fn create_settings_dialog() -> Markup {
    html! {
        dialog id="settings-dialog"
            _="on click
                 set r to my.getBoundingClientRect()
                 if event.clientX < r.left or event.clientX > r.right
                    or event.clientY < r.top or event.clientY > r.bottom
                   call me.close()" {
            h3 { "Share settings" }
            label {
                "Password"
                input type="password" id="settings-password" name="password"
                    placeholder="none" autocomplete="new-password";
            }
            label {
                "Days active"
                input type="number" id="settings-days" name="days"
                    value="31" min="1" max="365"
                    _="on input if (my.value as Int) > 365 then set my.value to 365
                                else if (my.value as Int) < 1 then set my.value to 1";
            }
            label {
                input type="checkbox" id="settings-tracking" name="tracking";
                " Track unique views"
            }
            div class="grid" {
                button _="on click call #settings-dialog.close()" { "Cancel" }
                button _="on click
                           set #share-days.value to #settings-days.value
                           set #share-password.value to #settings-password.value
                           call #settings-dialog.close()" { "Save" }
            }
        }
    }
}

pub fn create_markdown_editor_page(initial_content: &str) -> Markup {
    html! {
        (create_html_head(None));
        body a="auto" {
            (create_help_dialog())
            (create_settings_dialog())
            input type="hidden" id="share-days" name="days" value="31";
            input type="hidden" id="share-password" name="password" value="";
            main aria-label="Content" style="display: flex; flex-direction: column;" {
                div class="grid" {
                    button
                        id="preview-button"
                        hx-post="/preview"
                        hx-trigger="click"
                        hx-target="#markdown-preview"
                        hx-swap="innerHTML"
                        hx-include="#markdown-input"
                        hx-validate="true"
                        hx-disabled-elt="this"
                        _="on htmx:afterRequest
                           hide #markdown-input
                           show #markdown-preview
                           hide me
                           show #edit-button"
                           { "Preview" }
                    button
                        id="edit-button"
                        style="display: none;"
                        _="on click
                           hide #markdown-preview
                           show #markdown-input
                           hide me
                           show #preview-button"
                           { "Edit" }
                    button
                        id="settings-button"
                        _="on click
                           set #settings-days.value to #share-days.value
                           set #settings-password.value to #share-password.value
                           call #settings-dialog.showModal()" { "⚙️" }
                    button
                        id="share-button"
                        hx-post="/share"
                        hx-trigger="click"
                        hx-include="#markdown-input, #share-days, #share-password"
                        hx-validate="true"
                        hx-disabled-elt="this"
                        { "Share" }
                }
                textarea
                    id="markdown-input"
                    name="content"
                    placeholder=(if initial_content.is_empty() { "Enter your markdown..." } else { "" })
                    style="flex: 1; resize: none"
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
                div id="markdown-preview" style="display: none; flex: 1; overflow-y: auto;" {}
            }
            (create_page_footer());
        }
    }
}

pub fn create_markdown_viewer_page(doc: &Document) -> Markup {
    let html_output = convert_markdown_to_html(&doc.content);
    let page_title = extract_title_from_html(&html_output);
    let expires_at = doc.created_at + Duration::days(doc.days);

    html! {
        (create_html_head(page_title));
        body a="auto" {
            main aria-label="Content" _="on load call MathJax.typeset()" {
                (PreEscaped(html_output))
            }
            footer class="grid" {
                div {
                    p { "expires " (expires_at.format("%Y-%m-%d")) }
                    p {
                        a href=(format!("/?id={}", doc.id)) { "edit" }
                        " in "
                        a href="/" { "mdow" }
                        " 🌾"
                    }
                }
                div style="justify-self: end;" {
                    (PreEscaped(generate_qr_svg(&doc.id)))
                }
            }
        }
    }
}

pub fn create_password_prompt_page(id: &str, target: &str, error: bool, rate_limit_secs: Option<u64>) -> Markup {
    let locked = rate_limit_secs.is_some();
    let secs = rate_limit_secs.unwrap_or(0);
    html! {
        (create_html_head(Some("Password required")));
        body a="auto" {
            main aria-label="Content" {
                h1 { "Password required" }
                p { "This document is password protected." }
                @if locked {
                    p id="rate-msg"
                        _=(format!(
                            "init set :s to {} \
                             repeat until :s <= 0 \
                               wait 1s \
                               set :s to :s - 1 \
                               put :s into #countdown \
                             end \
                             remove @disabled from #pw-input \
                             remove @disabled from #unlock-btn \
                             remove me",
                            secs
                        )) {
                        "Too many failed attempts. Try again in "
                        span id="countdown" { (secs) }
                        "s."
                    }
                } @else if error {
                    p { "Incorrect password, please try again." }
                }
                form method="post" action=(format!("/unlock/{}", id)) {
                    input type="hidden" name="target" value=(target);
                    label {
                        "Password"
                        input type="password" id="pw-input" name="password"
                            autocomplete="current-password" required
                            disabled[locked];
                    }
                    button type="submit" id="unlock-btn" disabled[locked] { "Unlock" }
                }
            }
            (create_page_footer())
        }
    }
}

pub fn create_404_page() -> Markup {
    html! {
        (create_html_head(Some("404")));
        body a="auto" {
            main aria-label="Content" {
                h1 { "404 - Page Not Found" }
                p { "The page you're looking for doesn't exist." }
                p { a href="/" { "Return to homepage" } }
            }
        }
        (create_page_footer());
    }
}
