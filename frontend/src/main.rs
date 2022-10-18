#![allow(non_snake_case)]

use common::{DirDesc, JsonRequest};
use dioxus::{events::FormEvent, prelude::*};
use dioxus_router::{use_router, Route, Router};
use fast_qr::{
    convert::svg::{Shape, SvgBuilder},
    QRBuilder, Version, ECL,
};
use gloo_net::http::Request;
use log::info;
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
    let host_str = url.host_str().unwrap_or("");
    let scheme = url.scheme();
    let port = url
        .port()
        .unwrap_or_else(|| if scheme == "http" { 80 } else { 443 });

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
            QRCode { data: "http://baidu.com/?asdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljglaasdfafasdfasdfkaljgla" }

            div {
                class: "title",
                a { href: "{scheme}://{host_str}:{port}", "{scheme}://{host_str}:{port}" }
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

    let handle_upload_files = move |ev: FormEvent| {
        info!(">>>>>> file:{:?}", ev.values);
    };

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
                    prevent_default: "onsubmit",
                    onsubmit: handle_upload_files,
                    method: "post",
                    enctype: "multipart/form-data",
                    input {
                        r#type: "file",
                        name: "file",
                        multiple: "true",
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

            cx.props
            .dir_desc
            .descendants
            .iter()
            .map(|f|
                rsx!(
                    tr {
                        key: "{cur_path}/{f.file_name}",

                        onmouseover: |e| {
                            qrcode_state.set(Some(QRCodeParams {
                                x: e.data.client_x,
                                y: e.data.client_y,
                                w: 180,
                                h: 180,
                                url: f.file_name.clone(),
                            }));
                        },

                        if f.file_type == common::FileType::Directory {
                            rsx!(th {
                                a {
                                    href: "{cur_path}/{f.file_name}",
                                    "{f.file_name}"
                                }
                            })
                        } else {
                            rsx!(th { "{f.file_name}" })
                        }

                        td { "{f.file_size}" }
                        td { "{f.last_accessed}" }
                        td { "Delete" }
                    }
                )
            )

        }

        qrcode,
    })
}

struct QRCodeParams {
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    url: String,
}
