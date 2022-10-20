#![allow(non_snake_case)]

use std::fmt::Arguments;

use common::{DirDesc, DirEntry, JsonRequest};
use dioxus::{
    events::{ondragenter, FormData, FormEvent, MouseData},
    prelude::*,
};
use dioxus_router::{use_router, Route, Router};
use fast_qr::{
    convert::svg::{Shape, SvgBuilder},
    QRBuilder, Version, ECL,
};
use gloo_net::http::Request;
use log::info;
use reqwest::Url;
use wasm_bindgen::JsValue;

fn main() {
    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    wasm_logger::init(wasm_logger::Config::default());

    cx.render(rsx! {
        Router {
            Route { to: "", Listing {} }
        }
    })
}

fn Listing(cx: Scope) -> Element {
    let route = dioxus_router::use_route(&cx);
    let url = route.url();
    let url_base = get_url_base(url);

    let mut path = url.path().to_string();
    if path.ends_with('/') {
        path = path.trim_end_matches('/').to_string();
    }

    let fut = use_future(&cx, (), |_| async move {
        Request::get(format!("/api/listing{}", path).as_str())
            .send()
            .await
            .unwrap()
            .json::<DirDesc>()
            .await
    });

    let create_dir_state: &UseState<Option<String>> = use_state(&cx, || None);
    if let Some(dir_path) = create_dir_state.get() {
        create_dir_state.set(None);
        fut.restart();
        use_router(&cx).replace_route(dir_path.as_str(), None, None);

        return cx.render(rsx! {
            Router {
                Route { to: "", Listing {} }
            }
        });
    }

    cx.render(match fut.value() {
        Some(Ok(dir_desc)) => rsx!(
            div {
                class: "title",
                a { href: "{url_base}", "{url_base}" }
                "{dir_desc.dir_name}"
            }

            CreateDirectory {
                parent_dir: dir_desc.dir_name.clone(),
                create_dir_state: create_dir_state.clone(),
            }

            div {
                ListingTable{ dir_desc: dir_desc, cur_url: &url },
            }
        ),
        Some(Err(err)) => rsx!("Error: {err}"),
        _ => rsx!("Loading..."),
    })
}

fn get_url_base(url: &Url) -> String {
    match url.host_str() {
        Some(host_str) => {
            let scheme = url.scheme();
            let port = url.port().unwrap_or_default();
            if port > 0 {
                format!("{}://{}:{}", scheme, host_str, port)
            } else {
                format!("{}://{}", scheme, host_str)
            }
        }
        None => "".to_string(),
    }
}

#[inline_props]
fn QRCode<'a>(cx: Scope, data: &'a str) -> Element {
    let qrcode = QRBuilder::new(data.to_string())
        .ecl(ECL::L)
        .version(Version::V09)
        .build();

    let svg = SvgBuilder::default()
        .shape(Shape::RoundedSquare)
        .to_str(&qrcode.unwrap());

    cx.render(rsx!(div {
        class: "qrcode",
        dangerous_inner_html: "{svg}",
    }))
}

#[derive(Props)]
struct TooltipProps<'a> {
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    children: Element<'a>,
}

fn Tooltip<'a>(cx: Scope<'a, TooltipProps<'a>>) -> Element {
    let props = cx.props;
    cx.render(rsx!(div {
        class: "tooltip",
        style: "width:{props.w}px; height:{props.h}px; left:{props.x}px; top:{props.y}px;",
        &props.children
    }))
}

#[inline_props]
fn CreateDirectory(
    cx: Scope,
    parent_dir: String,
    create_dir_state: UseState<Option<String>>,
) -> Element {
    let handle_create_dir = move |ev: FormEvent| {
        if let Some(dir_name) = ev.values.get("dir_name") {
            let dir_name = dir_name.trim().to_string();
            if dir_name.is_empty() {
                return;
            }

            let parent_dir = parent_dir.clone();
            let create_dir_state = create_dir_state.clone();
            cx.spawn(async move {
                let json_req = JsonRequest::CreateDirectory {
                    dir_name: dir_name.clone(),
                };

                let resp = Request::post(format!("/api/listing{}", parent_dir).as_str())
                    .json(&json_req)
                    .unwrap()
                    .send()
                    .await;

                match resp {
                    Ok(resp) => {
                        info!("created directory: {:?}", resp);
                        create_dir_state.set(Some(format!("{}/{}", parent_dir, dir_name)));
                    }
                    Err(err) => {
                        info!("failed to create directory: {}", err);
                    }
                }
            });
        }
    };

    let _handle_upload_files = move |ev: FormEvent| {
        info!("upload: {:?}", ev.values);
    };

    let action = format!("/api/upload/{}?filename=test", parent_dir);

    cx.render(rsx! {
        div {
            class: "header_card_container",
            div {
                class: "card",
                div { "Create a new sub-directory under current directory" }
                form {
                    prevent_default: "onsubmit",
                    onsubmit: handle_create_dir,
                    method: "post",
                    input {
                        r#type: "text",
                        name: "dir_name"
                    }
                    button {
                        "Create Directory"
                    }
                }
            }

            div {
                class: "card",
                div { "Select files to upload to current directory" }

                form {
                    action: "{action}",
                    prevent_default: "onsubmit",
                    // onsubmit: handle_upload_files,
                    method: "post",
                    enctype: "multipart/form-data",
                    input {
                        r#type: "file",
                        name: "file",
                        multiple: "false",
                    }
                    input {
                        r#type: "submit",
                        value: "Upload"
                    }
                }
            }
        }
    })
}

#[derive(Props)]
pub struct DirDescProps<'a> {
    cur_url: &'a Url,
    dir_desc: &'a DirDesc,
}

fn ListingTable<'a>(cx: Scope<'a, DirDescProps<'a>>) -> Element {
    let mut cur_path = cx.props.cur_url.path();
    cur_path = cur_path.trim_end_matches(|c| c == '/');
    let mut parent = "/";
    if let Some(idx) = cur_path.rfind('/') {
        if idx > 0 {
            parent = &cur_path[..idx];
        }
    }

    let qrcode_state: &UseState<Option<QRCodeParams>> = use_state(&cx, || None);
    let qrcode = match qrcode_state.get() {
        Some(qrcode_params) => Some(rsx!(Tooltip {
            w: qrcode_params.w,
            h: qrcode_params.h,
            x: qrcode_params.x,
            y: qrcode_params.y,
            QRCode {
                data: qrcode_params.url.as_str()
            }
        })),
        _ => None,
    };

    let url_base = use_state(&cx, || get_url_base(cx.props.cur_url));

    cx.render(rsx! {
        table {
            thead {
                tr {
                    td { "Name" }
                    td { "Size" }
                    td { "Last Access Time" }
                    td { "Operation" }
                }
            }

            (!cur_path.is_empty()).then(|| rsx!(
                tr {
                    th {
                        colspan: "4",
                        a {
                            href: "{parent}",
                            "◄ Parent Directory"
                        }
                    }
                }
            ))

            cx.props.dir_desc.descendants.is_empty().then(|| rsx!{
                tr {
                    th {
                        class: "empty_directory",
                        colspan: "4",
                        "This directory is empty."
                    }
                }
            })

            cx.props
            .dir_desc
            .descendants
            .iter()
            .map(|entry| rsx!(
                TableRow {
                    key: "{cur_path}/{entry.file_name}",
                    url_base: url_base.as_str(),
                    entry: entry,
                    cur_path: cur_path,
                    qrcode_state: qrcode_state
                }))

        }

        qrcode,
    })
}

#[derive(Props)]
struct DirEntryProps<'a> {
    url_base: &'a str,
    entry: &'a DirEntry,
    cur_path: &'a str,
    qrcode_state: &'a UseState<Option<QRCodeParams>>,
}

fn TableRow<'a>(cx: Scope<'a, DirEntryProps<'a>>) -> Element {
    let url_base = if cx.props.url_base.ends_with('/') {
        cx.props.url_base.trim_end_matches('/')
    } else {
        cx.props.url_base
    };

    let entry = cx.props.entry;
    cx.render(rsx! {
        tr {
            rsx!(th {
                a {
                    href: "#",
                    style: "margin-right: 8px",
                    onmouseover: move |e| {
                        cx.props.qrcode_state.set(Some(QRCodeParams {
                            x: e.data.client_x + 20,
                            y: e.data.client_y + 20,
                            w: 240,
                            h: 240,
                            url: format!("{}{}/{}", url_base, cx.props.cur_path, entry.file_name),
                        }));
                    },
                    onmouseout: move |_| {
                        cx.props.qrcode_state.set(None)
                    },
                    img { src:"data:image/x-icon;base64,AAABAAEAEBACAAAAAACwAAAAFgAAACgAAAAQAAAAIAAAAAEAAQAAAAAAQAAAAAAAAAAAAAAAAgAAAAAAAAAAAAAA////AAKnAAB6MgAASlIAAEtCAAB7AAAAAnkAAP/YAACDBQAAUGMAAPy/AAACQAAAel4AAEpSAABK0gAAel4AAAJAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", alt:"QRCode" }
                }

                a {
                    href: "{cx.props.cur_path}/{entry.file_name}",
                    if entry.file_type == common::FileType::Directory {
                        rsx!("📁 ")
                    } else {
                        rsx!("📝 ")
                    }
                    "{entry.file_name}"
                }
            })

            td { "{entry.file_size}" }
            td { "{entry.last_accessed}" }
            td { "Delete" }
        }
    })
}

struct QRCodeParams {
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    url: String,
}
