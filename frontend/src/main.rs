#![allow(non_snake_case)]

use common::{DirDesc, JsonRequest};
use dioxus::{events::FormEvent, prelude::*};
use dioxus_router::{Route, Router};
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
    let path = url.path().to_string();
    let scheme = url.scheme();
    let port = url
        .port()
        .unwrap_or_else(|| if scheme == "http" { 80 } else { 443 });

    let fut = use_future(&cx, (), |_| async move {
        Request::get(format!("/api/listing{}", path).as_str())
            .send()
            .await
            .unwrap()
            .json::<DirDesc>()
            .await
    });

    cx.render(match fut.value() {
        Some(Ok(dir_desc)) => rsx!(
            div {
                class: "title",
                a { href: "{scheme}://{host_str}:{port}", "{scheme}://{host_str}:{port}" }
                "{dir_desc.dir_name}"
            }

            CreateDirectory {
                parent_dir: dir_desc.dir_name.clone(),
            }

            div {
                ListingTable{ props: DirDescProps { dir_desc, cur_url: &url }  },
            }
        ),
        Some(Err(err)) => rsx!("Error: {err}"),
        _ => rsx!("Loading..."),
    })
}

#[derive(Props)]
pub struct DirDescProps<'a> {
    cur_url: &'a Url,
    dir_desc: &'a DirDesc,
}

#[inline_props]
fn CreateDirectory(cx: Scope, parent_dir: String) -> Element {
    let handle_submit = move |ev: FormEvent| {
        if let Some(dir_name) = ev.values.get("dir_name") {
            let dir_name = dir_name.clone();
            let parent_dir = parent_dir.clone();
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
                    }
                    Err(err) => {
                        info!("failed to create directory: {}", err);
                    }
                }
            });
        }
    };

    cx.render(rsx! {
        div {
            class: "card",
            div { "Create a new sub-directory under current directory" }
            form {
                prevent_default: "onsubmit",
                onsubmit: handle_submit,
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
    })
}

#[inline_props]
fn ListingTable<'a>(cx: Scope, props: DirDescProps<'a>) -> Element {
    let mut cur_path = props.cur_url.path();
    let mut parent = "";
    cur_path = cur_path.trim_end_matches(|c| c == '/');
    if !cur_path.is_empty() {
        parent = &cur_path[..(cur_path.rfind('/').unwrap_or(0) + 1)];
    }

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

            props
            .dir_desc
            .descendants
            .iter()
            .map(|f|
                rsx!(
                    tr {
                        key: "{cur_path}/{f.file_name}",

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
    })
}
