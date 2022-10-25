#![allow(non_snake_case)]

use common::{DirDesc, DirEntry, JsonRequest};
use dioxus::{events::FormEvent, prelude::*};
use dioxus_router::{use_router, Route, Router};
use fast_qr::{
    convert::svg::{Shape, SvgBuilder},
    QRBuilder, Version, ECL,
};
use gloo_net::http::Request;
use log::{error, info};
use reqwest::Url;

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

    let update_state = use_state(&cx, || false);
    if *update_state.get() {
        update_state.set(false);
        fut.restart();
    }

    let router_state = use_router(&cx);
    let create_dir_state = use_state(&cx, || None as Option<String>);
    if let Some(dir_path) = create_dir_state.get() {
        create_dir_state.set(None);
        fut.restart();
        router_state.replace_route(dir_path.as_str(), None, None);

        return cx.render(rsx! {
            Listing {}
        });
    }

    let info_state = use_state(&cx, || None as Option<String>);

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

            ListingTable{ dir_desc: dir_desc, cur_url: &url, update_state: update_state, info_state: info_state },

            InfoDialog { info_state: info_state }
        ),
        Some(Err(err)) => rsx!("Error: {err}"),
        _ => rsx!("Loading..."),
    })
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
            let create_dir_state = create_dir_state.to_owned();
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
                        create_dir_state.set(Some(if parent_dir == "/" {
                            dir_name
                        } else {
                            format!("{}/{}", parent_dir, dir_name)
                        }));
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

    let action = format!("/api/upload{}", parent_dir);

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
                        name: "filename",
                        multiple: "true",
                    }
                    button {
                        "Upload"
                    }
                }
            }
        }
    })
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

    let qrcode_state: &UseState<Option<String>> = use_state(&cx, || None);
    let qrcode = match qrcode_state.get() {
        Some(url) => Some(rsx!(QRCode { data: url.as_str() })),
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
                        colspan: "4",
                        class: "empty_directory",
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
                    update_state: cx.props.update_state,
                    info_state: cx.props.info_state,
                    qrcode_state: qrcode_state,
                }))

        }

        qrcode,
    })
}

fn TableRow<'a>(cx: Scope<'a, DirEntryProps<'a>>) -> Element {
    let url_base = if cx.props.url_base.ends_with('/') {
        cx.props.url_base.trim_end_matches('/')
    } else {
        cx.props.url_base
    };

    let entry = cx.props.entry;
    let api_link = if entry.file_type == common::FileType::Directory {
        format!("{}/{}", cx.props.cur_path, entry.file_name)
    } else {
        format!("/api/static{}/{}", cx.props.cur_path, entry.file_name)
    };

    let url = format!("{}/{}", url_base, api_link.trim_start_matches('/'));
    let formatted_bytes = format_bytes(entry.file_size);
    cx.render(rsx! {
        tr {
            rsx!(th {
                a {
                    href: "{api_link}",
                    if entry.file_type == common::FileType::Directory {
                        rsx!("📁 ")
                    } else {
                        rsx!("📝 ")
                    }
                    "{entry.file_name}"
                }
            })

            td { "{formatted_bytes}" }
            td { "{entry.last_accessed}" }
                td {

                    (entry.file_type == common::FileType::File).then(|| rsx!(
                        button {
                            prevent_default: "onclick",
                            r#type: "button",
                            style: "margin-right: 8px",
                            onclick: move |_| {
                                let path = format!("/api/ffprobe{}/{}", cx.props.cur_path, entry.file_name);
                                let info_state = cx.props.info_state.clone();
                                cx.spawn(async move {
                                    let resp = Request::get(path.as_str())
                                        .send()
                                        .await;

                                    match resp {
                                        Ok(resp) => {
                                            let text = resp.text().await.unwrap_or("".to_string());
                                            if text.is_empty() || text == "\"{\\n\\n}\\n\"" {
                                                info_state.set(Some("Not Available!".to_string()));
                                            } else {
                                                let json_resp: serde_json::Value = serde_json::from_str(text.as_str()).unwrap_or(serde_json::Value::default());
                                                let json_str = json_resp.to_string().replace("\\n", "\n").replace("\\", "");
                                                info_state.set(Some(json_str));
                                            }
                                        }
                                        Err(err) => {
                                            error!("failed: {}", err);
                                        }
                                    }
                                });
                            },
                            "MediaInfo",
                        }
                    )),

                    button {
                        prevent_default: "onclick",
                        r#type: "button",
                        style: "margin-right: 8px",
                        onclick: move |_| cx.props.qrcode_state.set(Some(url.clone())),
                        onmouseout: move |_| {
                            cx.props.qrcode_state.set(None)
                        },
                        "QRCode"
                    }

                    button {
                        prevent_default: "onclick",
                        r#type: "button",
                        onclick: move |_| {
                            let path = format!("/api/delete{}/{}", cx.props.cur_path, entry.file_name);
                            let update_state = cx.props.update_state.clone();
                            cx.spawn(async move {
                                let resp = Request::post(path.as_str())
                                    .send()
                                    .await;

                                match resp {
                                    Ok(resp) => {
                                        info!("succeeded: {:?}", resp);
                                        update_state.set(true);
                                    }
                                    Err(err) => {
                                        info!("failed: {}", err);
                                    }
                                }
                            });
                        },
                        "Delete"
                    }
                }
            }
    })
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
        style: "
            position: fixed;
            width: 360px;
            height: 360px;
            left: 50%;
            top: 50%;
            margin-left: -180px;
            margin-top: -180px;
            z-index: 100;
            border: dotted 2px #000;
        ",
        dangerous_inner_html: "{svg}",
    }))
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
fn InfoDialog<'a>(cx: Scope<'a>, info_state: &'a UseState<Option<String>>) -> Element {
    cx.render(if let Some(str_info) = info_state.get() {
        rsx!(div {
            style: "
                position: fixed;
                width: 860px;
                height: 1000px;
                left: 50%;
                margin-left: -430px;
                top: 50%;
                margin-top: -500px;
                z-index: 20;
                border-radius: 5px;
                border: 2px solid #ccc;
                background: #eee;
                overflow: scroll;
            ",
            textarea {
                style: "
                    width:850px;
                    height: 940px;
                    margin:5px;
                    border: none;
                    box-sizing:border-box;
                    resize: none;
                    border-bottom: 2px solid #ccc;
                ",
                disabled: "true",
                value: "{str_info}",
            }
            p {
                style: "
                    text-align: center;
                    width: 100%;
                    position: absolute;
                    bottom: 0px;
                    margin:0px;
                    box-sizing:border-box;
                    padding: 10px;
                ",
                button {
                    prevent_default: "onclick",
                    r#type: "button",
                    onclick: |_| info_state.set(None),
                    "Close",
                }
            }
        })
    } else {
        rsx!("")
    })
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2}M", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2}K", bytes as f64 / 1024.0)
    } else {
        format!("{}", bytes)
    }
}

// fn Tooltip<'a>(cx: Scope<'a, TooltipProps<'a>>) -> Element {
//     let props = cx.props;
//     cx.render(rsx!(div {
//         class: "tooltip",
//         style: "width:{props.w}px; height:{props.h}px; left:{props.x}px; top:{props.y}px;",
//         &props.children
//     }))
// }

// #[inline_props]
// fn AlertDialog<'a>(
//     cx: Scope<'a>,
//     msg: &'a str,
//     positive_callback: Box<dyn Fn(MouseEvent) -> ()>,
//     negative_callback: Box<dyn Fn(MouseEvent) -> ()>,
// ) -> Element {
//     cx.render(rsx!(div {
//         "{msg}",

//         input {
//             prevent_default: "onclick",
//             r#type: "button",
//             onclick: positive_callback,
//             value: "Yes"
//         }

//         input {
//             prevent_default: "onclick",
//             r#type: "button",
//             onclick: negative_callback,
//             value: "No"
//         }
//     }))
// }

// #[derive(Props)]
// struct TooltipProps<'a> {
//     w: i32,
//     h: i32,
//     x: i32,
//     y: i32,
//     children: Element<'a>,
// }

#[derive(Props)]
pub struct DirDescProps<'a> {
    cur_url: &'a Url,
    dir_desc: &'a DirDesc,
    update_state: &'a UseState<bool>,
    info_state: &'a UseState<Option<String>>,
}

#[derive(Props)]
struct DirEntryProps<'a> {
    url_base: &'a str,
    entry: &'a DirEntry,
    cur_path: &'a str,
    update_state: &'a UseState<bool>,
    qrcode_state: &'a UseState<Option<String>>,
    info_state: &'a UseState<Option<String>>,
}
