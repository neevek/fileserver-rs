#![allow(non_snake_case)]

use common::DirDesc;
use dioxus::prelude::*;
use dioxus_router::{Route, Router};
use gloo_net::http::Request;
use log::info;
use reqwest::Url;
use serde::Deserialize;

fn main() {
    dioxus::web::launch(app);
}

#[derive(Deserialize)]
struct Query {
    path: String,
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

    // let query = route.
    //     .query::<Query>()
    //     .unwrap_or(Query {
    //         path: "".to_string(),
    //     });

    // let state = use_state(&cx, || "".to_string());
    let fut = use_future(&cx, (), |_| async move {
        Request::get(format!("/api{}", path).as_str())
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
                "{host_str}{dir_desc.dir_name}"
            }
            div {
                ListingTable{ props: DirDescProps { dir_desc, cur_url: &url }  }
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
                        td { "value2" }
                    }
                )
            )

        }
    })
}
