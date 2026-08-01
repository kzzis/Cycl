use dioxus::prelude::*;

use crate::tauri_api::mcp as api;

/// MCPサーバーの有効化と、Claude Desktopに貼る接続設定。
#[component]
pub fn McpSettings() -> Element {
    let mut status = use_resource(move || async move { api::status().await.ok() });
    let mut pending = use_signal(|| false);
    let mut last_error = use_signal(|| None::<String>);

    let current = status.value().read().clone().flatten();
    let enabled = current.as_ref().map(|s| s.enabled).unwrap_or(false);
    let port = current.as_ref().map(|s| s.port).unwrap_or(3737);
    let token = current
        .as_ref()
        .map(|s| s.token.clone())
        .unwrap_or_default();

    let toggle = move |_| {
        if *pending.read() {
            return;
        }
        pending.set(true);
        spawn(async move {
            match api::set_enabled(!enabled).await {
                Ok(next) => last_error.set(next.error),
                Err(e) => last_error.set(Some(e)),
            }
            pending.set(false);
            status.restart();
        });
    };

    // Claude Desktopのconfigにそのまま貼れる形。
    let config = format!(
        "{{\n  \"mcpServers\": {{\n    \"cycl\": {{\n      \"url\": \"http://127.0.0.1:{port}/mcp\",\n      \"headers\": {{ \"Authorization\": \"Bearer {token}\" }}\n    }}\n  }}\n}}"
    );

    rsx! {
        section { class: "settings__section",
            h2 { class: "settings__title", "MCP server" }
            p { class: "settings__hint muted",
                "Lets Claude Desktop read and control your todos, timer and stats. "
                "Runs locally on 127.0.0.1 only and needs the token below."
            }

            div { class: "settings__row",
                span { class: "settings__row-name",
                    if enabled { "Running on port {port}" } else { "Off" }
                }
                button {
                    class: if enabled { "btn btn--ghost" } else { "btn btn--primary" },
                    r#type: "button",
                    disabled: *pending.read(),
                    onclick: toggle,
                    if *pending.read() {
                        "Working…"
                    } else if enabled {
                        "Stop"
                    } else {
                        "Start"
                    }
                }
            }

            if let Some(error) = last_error.read().clone() {
                p { class: "settings__error", "{error}" }
            }

            if enabled {
                div { class: "mcp-config",
                    span { class: "settings__hint muted", "Add this to your Claude Desktop config:" }
                    pre { class: "mcp-config__code", "{config}" }
                    p { class: "settings__hint muted",
                        "The token changes every time Cycl restarts, so update the config after a restart."
                    }
                }
            }
        }
    }
}
