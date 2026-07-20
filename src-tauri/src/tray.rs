use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle_window = MenuItem::with_id(
        app,
        "toggle_window",
        "ウィンドウを表示/非表示",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_window, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle_window" => toggle_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    // ウィンドウの「閉じる」はアプリ終了ではなく非表示にする。
    // 常駐アプリの終了は必ずトレイメニューの「終了」から行う。
    if let Some(window) = app.get_webview_window("main") {
        let window_for_close = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_for_close.hide();
            }
        });
    }

    // Dockにアイコンを出さず、メニューバー常駐アプリらしい見た目にする。
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    Ok(())
}

pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// トレイアイコンのタイトル(macOSのメニューバーに出る文字列)を更新する。
pub fn update_title(app: &AppHandle, title: Option<String>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(title);
    }
}
