use gloo_net::http::Request;
use log::info;
use serde::Serialize;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/listing/:path")]
    Listing { path: String },
    #[at("/hello")]
    Hello,
}

fn switch(routes: &Route) -> Html {
    match routes {
        Route::Listing { path } => html! { <Listing path={ path.clone() } /> },
        Route::Hello => html! { "Hello!" },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={Switch::render(switch)} />
        </BrowserRouter>
    }
}

#[function_component(ListingItem)]
fn listing_item() -> Html {
    html! {
        <tr>
            // <td>David</td>
            // <td>Male</td>
            // <td>23</td>
        </tr>
    }
}

#[derive(Properties, PartialEq, Serialize)]
pub struct ListingProps {
    pub path: String,
}

#[function_component(Listing)]
fn listing(props: &ListingProps) -> Html {
    info!(">>>>>>>>>>> render 1: {}", props.path);
    let listing_dir = props.path.clone();
    let dir_desc = use_state(|| None);
    let dir_desc2 = dir_desc.clone();
    // spawn_local(async move {
    //     let resp = Request::get(format!("/api/{}", listing_dir).as_str())
    //         .send()
    //         .await
    //         .unwrap();
    //     let result = {
    //         if !resp.ok() {
    //             Err(format!(
    //                 "Error fetching data {} ({})",
    //                 resp.status(),
    //                 resp.status_text()
    //             ))
    //         } else {
    //             resp.text().await.map_err(|err| err.to_string())
    //         }
    //     };
    //     info!(">>>>>>>> haha result: {:?}", result);
    //     dir_desc.set(Some(result));
    // });

    use_effect(move || {
        if dir_desc.is_none() {
            spawn_local(async move {
                let resp = Request::get(format!("/api/{}", listing_dir).as_str())
                    .send()
                    .await
                    .unwrap();
                let result = {
                    if !resp.ok() {
                        Err(format!(
                            "Error fetching data {} ({})",
                            resp.status(),
                            resp.status_text()
                        ))
                    } else {
                        resp.text().await.map_err(|err| err.to_string())
                    }
                };
                info!(">>>>>>>> side effect: {:?}", result);
                dir_desc.set(Some(result));
            });
        }

        || {
            info!(">>>>>> unmount");
        }
    });

    html! {
        <div>
        {
            if let Some(Ok(result)) = dir_desc2.as_ref() {
                result

            } else {
                "loading..."
            }
        }
        </div>
        // <BrowserRouter>
        //     <Switch<Route> render={Switch::render(switch)} />
        // </BrowserRouter>
        // <table>
            // <thead>
            //     <tr>
            //         <td>Name</td>
            //         <td>Size</td>
            //         <td>Last Access Time</td>
            //         <td>Operation</td>
            //     </tr>
            // </thead>
            // <tbody>
            //     <tr>
            //         <td>David</td>
            //         <td>Male</td>
            //         <td>23</td>
            //     </tr>
            //     <tr>
            //         <td>Jessica</td>
            //         <td>Female</td>
            //         <td>47</td>
            //     </tr>
            //     <tr>
            //         <td>Warren</td>
            //         <td>Male</td>
            //         <td>12</td>
            //     </tr>
            // </tbody>
        // </table>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();

    yew::start_app::<App>();
}
