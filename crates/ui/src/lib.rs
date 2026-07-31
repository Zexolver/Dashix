#[cfg(target_os = "android")]
mod android_colors;
#[cfg(target_os = "android")]
mod android_priv;
mod api_client;

use slint::{Model, ModelRc, SharedString, VecModel, Weak};

use api_client::{
    Client, Dynv6ConfigDto, MailAccountDto, MailConfigDto, NewRouteDto, RouteTargetDto,
    SecurityConfigDto,
};

slint::include_modules!();

/// The whole app: builds the window, wires every callback, and runs the
/// event loop until the window closes. Shared by the desktop entry point
/// (`src/main.rs`) and the Android one (`android_main` below) -- only
/// which windowing backend Slint picked at startup (desktop winit vs.
/// `i-slint-backend-android-activity`, selected via Cargo features per
/// target, see Cargo.toml) differs.
pub fn run() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let _guard = rt.enter();

    let ui = MainWindow::new().expect("failed to create the main window");
    let client = Client::new();

    #[cfg(target_os = "android")]
    android_colors::apply_dynamic_colors(&ui);
    #[cfg(target_os = "android")]
    android_priv::ensure_daemon_running();

    load_everything(ui.as_weak(), client.clone());

    register_gateway_callbacks(&ui, client.clone());
    register_router_callbacks(&ui, client.clone());
    register_security_callbacks(&ui, client.clone());
    register_mail_callbacks(&ui, client.clone());

    ui.run().expect("event loop failed");
}

/// Android's activity shim calls this (via the `cdylib` build) instead of
/// a normal `fn main()`. Slint's Android backend must be initialized
/// before anything else touches Slint, per `slint::android`'s documented
/// contract, then it's the same `run()` the desktop binary uses.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("failed to initialize Slint's Android backend");
    run();
}

fn set_status(ui_weak: &Weak<MainWindow>, text: String) {
    let ui_weak = ui_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_status_line(text.into());
        }
    });
}

/// Fetches everything from the daemon and populates all four modules. Run
/// once at startup; individual tabs also have their own lighter-weight
/// refresh/save actions afterwards.
fn load_everything(ui_weak: Weak<MainWindow>, client: Client) {
    tokio::spawn(async move {
        let interfaces = client.list_interfaces().await;
        let dynv6 = client.get_dynv6().await;
        let dynv6_status = client.dynv6_status().await;
        let routes = client.list_routes().await;
        let security = client.get_security().await;
        let mail = client.get_mail().await;

        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            match interfaces {
                Ok(list) => apply_interfaces(&ui, list),
                Err(e) => ui.set_status_line(format!("interfaces: {e}").into()),
            }
            if let Ok(cfg) = dynv6 {
                apply_dynv6_config(&ui, cfg);
            }
            if let Ok(statuses) = dynv6_status {
                ui.set_dynv6_status_text(format_dynv6_status(&statuses).into());
            }
            match routes {
                Ok(list) => apply_routes(&ui, list),
                Err(e) => ui.set_status_line(format!("routes: {e}").into()),
            }
            if let Ok(cfg) = security {
                apply_security(&ui, cfg);
            }
            if let Ok(cfg) = mail {
                apply_mail(&ui, cfg);
            }
        });
    });
}

fn apply_interfaces(ui: &MainWindow, list: Vec<api_client::InterfaceDto>) {
    let names: Vec<SharedString> = list.iter().map(|i| i.name.clone().into()).collect();
    let rows: Vec<InterfaceInfo> = list
        .into_iter()
        .map(|i| InterfaceInfo {
            name: i.name.into(),
            mac: i.mac.unwrap_or_default().into(),
            ipv4: i.ipv4.join(", ").into(),
            ipv6: i.ipv6.join(", ").into(),
        })
        .collect();
    ui.set_interfaces(ModelRc::new(VecModel::from(rows)));
    ui.set_interface_names(ModelRc::new(VecModel::from(names)));
}

fn apply_dynv6_config(ui: &MainWindow, cfg: Dynv6ConfigDto) {
    ui.set_dynv6_enabled(cfg.enabled);
    ui.set_dynv6_token(cfg.token.unwrap_or_default().into());
    ui.set_dynv6_interface(cfg.interface.unwrap_or_default().into());
    ui.set_dynv6_domains_text(cfg.domains.join(", ").into());
}

fn format_dynv6_status(statuses: &[api_client::Dynv6StatusDto]) -> String {
    if statuses.is_empty() {
        return "No sync attempts yet.".into();
    }
    statuses
        .iter()
        .map(|s| match &s.last_error {
            Some(err) => format!("{}: error - {err}", s.domain),
            None => format!(
                "{}: ok (last success at {})",
                s.domain,
                s.last_success.as_deref().unwrap_or("?")
            ),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn apply_routes(ui: &MainWindow, list: Vec<api_client::RouteDto>) {
    let rows: Vec<RouteRow> = list
        .into_iter()
        .map(|r| match r.target {
            RouteTargetDto::Static { path, .. } => RouteRow {
                id: r.id.into(),
                subdomain: r.subdomain.into(),
                is_static: true,
                path: path.into(),
                port: 0,
                tls: r.tls,
                internal_port: r.internal_port.unwrap_or(0) as i32,
            },
            RouteTargetDto::Port { port } => RouteRow {
                id: r.id.into(),
                subdomain: r.subdomain.into(),
                is_static: false,
                path: "".into(),
                port: port as i32,
                tls: r.tls,
                internal_port: 0,
            },
        })
        .collect();
    ui.set_routes(ModelRc::new(VecModel::from(rows)));
}

fn apply_security(ui: &MainWindow, cfg: SecurityConfigDto) {
    let rows: Vec<L4RuleRow> = cfg
        .l4_rules
        .into_iter()
        .map(|r| L4RuleRow {
            id: r.id.into(),
            name: r.name.into(),
            is_udp: r.protocol == "udp",
            listen_port: r.listen_port as i32,
            upstream_port: r.upstream_port as i32,
        })
        .collect();
    ui.set_l4_rules(ModelRc::new(VecModel::from(rows)));
    ui.set_blocked_ips_text(cfg.blocked_ips.join(", ").into());
}

fn apply_mail(ui: &MainWindow, cfg: MailConfigDto) {
    ui.set_mail_domain(cfg.domain.unwrap_or_default().into());
    let rows: Vec<MailAccountRow> = cfg
        .accounts
        .into_iter()
        .map(|a| MailAccountRow {
            address: a.address.into(),
            display_name: a.display_name.into(),
        })
        .collect();
    ui.set_mail_accounts(ModelRc::new(VecModel::from(rows)));
}

fn split_csv(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn register_gateway_callbacks(ui: &MainWindow, client: Client) {
    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_refresh_interfaces(move || {
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.list_interfaces().await {
                Ok(list) => {
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_interfaces(&ui, list));
                }
                Err(e) => set_status(&ui_weak, format!("refresh interfaces failed: {e}")),
            }
        });
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_save_dynv6(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let cfg = Dynv6ConfigDto {
            enabled: ui.get_dynv6_enabled(),
            token: non_empty(ui.get_dynv6_token().to_string()),
            interface: non_empty(ui.get_dynv6_interface().to_string()),
            domains: split_csv(&ui.get_dynv6_domains_text()),
        };
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.set_dynv6(&cfg).await {
                Ok(()) => set_status(&ui_weak, "dynv6 config saved".into()),
                Err(e) => set_status(&ui_weak, format!("save dynv6 failed: {e}")),
            }
        });
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_sync_dynv6_now(move || {
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.sync_dynv6_now().await {
                Ok(statuses) => {
                    let text = format_dynv6_status(&statuses);
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_dynv6_status_text(text.into());
                    });
                }
                Err(e) => set_status(&ui_weak, format!("sync failed: {e}")),
            }
        });
    });
}

fn register_router_callbacks(ui: &MainWindow, client: Client) {
    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_add_route(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let subdomain = ui.get_new_subdomain().to_string();
        if subdomain.is_empty() {
            set_status(&ui_weak, "subdomain is required".into());
            return;
        }
        let target = if ui.get_new_is_static() {
            RouteTargetDto::Static {
                path: ui.get_new_path().to_string(),
                hot_reload: true,
            }
        } else {
            RouteTargetDto::Port {
                port: ui.get_new_port() as u16,
            }
        };
        let req = NewRouteDto {
            subdomain,
            target,
            tls: ui.get_new_tls(),
        };

        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.create_route(&req).await {
                Ok(_) => match c.list_routes().await {
                    Ok(list) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_routes(&ui, list));
                    }
                    Err(e) => set_status(&ui_weak, format!("list routes failed: {e}")),
                },
                Err(e) => set_status(&ui_weak, format!("add route failed: {e}")),
            }
        });
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_delete_route(move |id| {
        let id = id.to_string();
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.delete_route(&id).await {
                Ok(()) => match c.list_routes().await {
                    Ok(list) => {
                        let _ = ui_weak.upgrade_in_event_loop(move |ui| apply_routes(&ui, list));
                    }
                    Err(e) => set_status(&ui_weak, format!("list routes failed: {e}")),
                },
                Err(e) => set_status(&ui_weak, format!("delete route failed: {e}")),
            }
        });
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_apply(move || {
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.apply().await {
                Ok(()) => set_status(
                    &ui_weak,
                    "applied: configs regenerated, services restarted".into(),
                ),
                Err(e) => set_status(&ui_weak, format!("apply failed: {e}")),
            }
        });
    });
}

fn register_security_callbacks(ui: &MainWindow, client: Client) {
    let ui_weak = ui.as_weak();
    ui.on_add_l4_rule(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let name = ui.get_new_l4_name().to_string();
        if name.is_empty() {
            set_status(&ui_weak, "rule name is required".into());
            return;
        }
        let row = L4RuleRow {
            id: uuid::Uuid::new_v4().to_string().into(),
            name: name.into(),
            is_udp: ui.get_new_l4_udp(),
            listen_port: ui.get_new_l4_listen_port(),
            upstream_port: ui.get_new_l4_upstream_port(),
        };
        let mut rows: Vec<L4RuleRow> = ui.get_l4_rules().iter().collect();
        rows.push(row);
        ui.set_l4_rules(ModelRc::new(VecModel::from(rows)));
        ui.set_new_l4_name("".into());
    });

    let ui_weak = ui.as_weak();
    ui.on_delete_l4_rule(move |id| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let rows: Vec<L4RuleRow> = ui.get_l4_rules().iter().filter(|r| r.id != id).collect();
        ui.set_l4_rules(ModelRc::new(VecModel::from(rows)));
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_save_security(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let l4_rules = ui
            .get_l4_rules()
            .iter()
            .map(|r| api_client::L4RuleDto {
                id: r.id.to_string(),
                name: r.name.to_string(),
                protocol: if r.is_udp { "udp" } else { "tcp" }.to_string(),
                listen_port: r.listen_port as u16,
                upstream_port: r.upstream_port as u16,
            })
            .collect();
        let cfg = SecurityConfigDto {
            l4_rules,
            blocked_ips: split_csv(&ui.get_blocked_ips_text()),
        };
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.set_security(&cfg).await {
                Ok(()) => set_status(&ui_weak, "security config saved".into()),
                Err(e) => set_status(&ui_weak, format!("save security failed: {e}")),
            }
        });
    });
}

fn register_mail_callbacks(ui: &MainWindow, client: Client) {
    let ui_weak = ui.as_weak();
    ui.on_add_mail_account(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let address = ui.get_new_mail_address().to_string();
        if address.is_empty() {
            set_status(&ui_weak, "address is required".into());
            return;
        }
        let row = MailAccountRow {
            address: address.into(),
            display_name: ui.get_new_mail_display_name(),
        };
        let mut rows: Vec<MailAccountRow> = ui.get_mail_accounts().iter().collect();
        rows.push(row);
        ui.set_mail_accounts(ModelRc::new(VecModel::from(rows)));
        ui.set_new_mail_address("".into());
        ui.set_new_mail_display_name("".into());
    });

    let ui_weak = ui.as_weak();
    ui.on_delete_mail_account(move |address| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let rows: Vec<MailAccountRow> = ui
            .get_mail_accounts()
            .iter()
            .filter(|a| a.address != address)
            .collect();
        ui.set_mail_accounts(ModelRc::new(VecModel::from(rows)));
    });

    let ui_weak = ui.as_weak();
    let c = client.clone();
    ui.on_save_mail(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let accounts = ui
            .get_mail_accounts()
            .iter()
            .map(|a| MailAccountDto {
                address: a.address.to_string(),
                display_name: a.display_name.to_string(),
            })
            .collect();
        let cfg = MailConfigDto {
            domain: non_empty(ui.get_mail_domain().to_string()),
            accounts,
        };
        let ui_weak = ui_weak.clone();
        let c = c.clone();
        tokio::spawn(async move {
            match c.set_mail(&cfg).await {
                Ok(()) => set_status(&ui_weak, "mail config saved".into()),
                Err(e) => set_status(&ui_weak, format!("save mail failed: {e}")),
            }
        });
    });
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
