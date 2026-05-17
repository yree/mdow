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
            script src="https://unpkg.com/htmx.org@2.0.10" {};
            script src="https://unpkg.com/hyperscript.org@0.9.91" {};

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
                 end
               on click
                 if event.target is me then call me.close() end" {
            h2 { "mdow 🌾" }
            p { "A meadow for your " b { "markdown on web." } }
            p style="margin-bottom: 0" { "Write markdown, preview it, and share it as a link. Share links stay active for 31 days — customize with ⚙️:" }
            ul style="margin-top: 0" {
                li { "Custom expiry" }
                li { "Password protection" }
                li { "Unique view tracking" }
            }
            p { span hx-get="/stats" hx-trigger="load" {} }
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
                   call me.close()
                 end" {
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
                    _="on input
                         if (my.value as Int) > 365 then set my.value to 365
                         else if (my.value as Int) < 1 then set my.value to 1
                         end";
            }
            label {
                input type="checkbox" id="settings-tracking" name="tracking";
                " Track unique views"
            }
            div class="grid" {
                button type="button" _="on click call #settings-dialog.close()" { "Cancel" }
                button type="button" _="on click
                             set pwd to #settings-password.value
                             set days to #settings-days.value
                             if #settings-tracking.checked
                               set tracking to 'on'
                             else
                               set tracking to ''
                             end
                             set #share-password.value to pwd
                             set #share-days.value to days
                             set #share-tracking.value to tracking
                             call #settings-dialog.close()
                             if pwd or days is not '31' or tracking is 'on'
                               set #lbl-default.style.display to 'none'
                             else
                               set #lbl-default.style.display to 'inline'
                             end
                             if pwd
                               set #lbl-lock.style.display to 'inline'
                             else
                               set #lbl-lock.style.display to 'none'
                             end
                             if days is not '31'
                               set #lbl-timer.style.display to 'inline'
                             else
                               set #lbl-timer.style.display to 'none'
                             end
                             if tracking is 'on'
                               set #lbl-eyes.style.display to 'inline'
                             else
                               set #lbl-eyes.style.display to 'none'
                             end" { "Save" }
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
            input type="hidden" id="share-tracking" name="tracking" value="";
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
                           set #settings-tracking.checked to (#share-tracking.value is 'on')
                           call #settings-dialog.showModal()" {
                        span id="lbl-default" { "⚙️" }
                        span id="lbl-lock" style="display:none" { "🔒" }
                        span id="lbl-timer" style="display:none" { "⏳" }
                        span id="lbl-eyes" style="display:none" { "👀" }
                    }
                    button
                        id="share-button"
                        hx-post="/share"
                        hx-trigger="click"
                        hx-include="#markdown-input, #share-days, #share-password, #share-tracking"
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
                    @if doc.tracking {
                        p { (doc.views) " views. expires " (expires_at.format("%Y-%m-%d")) }
                    } @else {
                        p { "expires " (expires_at.format("%Y-%m-%d")) }
                    }
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

fn fmt_secs(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    match (m, s) {
        (0, s) => format!("{s}s"),
        (m, 0) => format!("{m}m"),
        (m, s) => format!("{m}m {s}s"),
    }
}

pub fn create_lockout_fragment(id: &str, target: &str, secs: u64) -> Markup {
    html! {
        p id="rate-msg" {
            "Too many failed attempts. Try again in "
            span { (fmt_secs(secs)) }
            "."
        }
        (create_password_form(id, target, true))
    }
}

pub fn create_unlock_form_fragment(id: &str, target: &str) -> Markup {
    create_password_form(id, target, false)
}

fn create_password_form(id: &str, target: &str, disabled: bool) -> Markup {
    html! {
        form method="post" action=(if target == "view" { format!("/view/{}", id) } else { format!("/unlock/{}", id) }) {
            input type="hidden" name="target" value=(target);
            label {
                "Password"
                input type="password" id="pw-input" name="password"
                    autocomplete="current-password" required
                    disabled[disabled];
            }
            button type="submit" id="unlock-btn" disabled[disabled] { "Unlock" }
        }
    }
}

pub fn create_password_prompt_page(id: &str, target: &str, error: bool, lockout: Option<(u64, u64)>) -> Markup {
    let locked = lockout.is_some();
    let (secs, expiry_unix) = lockout.unwrap_or((0, 0));
    html! {
        (create_html_head(Some("Password required")));
        body a="auto" {
            main aria-label="Content" {
                h1 { "Password required" }
                p { "This document is password protected." }
                @if locked {
                    div id="lockout-ui"
                        hx-get=(format!("/countdown?expires={}&target={}&id={}", expiry_unix, target, id))
                        hx-trigger="every 5s"
                        hx-target="this"
                        hx-swap="innerHTML"
                        _="on unlock remove @hx-trigger from me end" {
                        (create_lockout_fragment(id, target, secs))
                    }
                } @else {
                    @if error {
                        p { "Incorrect password, please try again." }
                    }
                    (create_password_form(id, target, false))
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
